use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const EARLY: &str = include_str!("scripts/early-hook.js");
const STATUS: &str = include_str!("scripts/status-probe.js");
const RUNTIME: &str = include_str!("scripts/runtime-probe.js");

pub(super) fn valid_socket(value: &str, port: u16) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    url.scheme() == "ws"
        && url.host_str() == Some("127.0.0.1")
        && url.port() == Some(port)
        && port != 0
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path().starts_with("/devtools/page/")
}

pub(super) fn target_socket(target: &Value, port: u16) -> Option<String> {
    if target["type"] != "page" {
        return None;
    }
    let url = url::Url::parse(target["url"].as_str()?).ok()?;
    if url.scheme() != "app"
        || url.host_str() != Some("-")
        || !matches!(url.path(), "/" | "/index.html")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
    {
        return None;
    }
    let socket = target["webSocketDebuggerUrl"].as_str()?;
    valid_socket(socket, port).then(|| socket.to_string())
}

pub(super) struct Cdp {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_id: u64,
}

impl Cdp {
    pub(super) async fn connect(socket: &str, port: u16) -> Result<Self, String> {
        if !valid_socket(socket, port) {
            return Err("CDP endpoint is not the controlled loopback endpoint".into());
        }
        let (socket, _) =
            tokio::time::timeout(REQUEST_TIMEOUT, tokio_tungstenite::connect_async(socket))
                .await
                .map_err(|_| "CDP connection timeout".to_string())?
                .map_err(|e| e.to_string())?;
        Ok(Self { socket, next_id: 0 })
    }

    pub(super) async fn request(
        &mut self,
        method: &str,
        params: Value,
        deadline: Duration,
    ) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        tokio::time::timeout(deadline, async {
            self.socket
                .send(Message::Text(
                    json!({"id":id,"method":method,"params":params})
                        .to_string()
                        .into(),
                ))
                .await
                .map_err(|e| e.to_string())?;
            while let Some(message) = self.socket.next().await {
                let message = message.map_err(|e| e.to_string())?;
                let Message::Text(text) = message else {
                    continue;
                };
                let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                if value["id"].as_u64() != Some(id) {
                    continue;
                }
                if let Some(error) = value.get("error") {
                    return Err(format!("CDP request failed: {error}"));
                }
                return value
                    .get("result")
                    .cloned()
                    .ok_or_else(|| "CDP response missing result".into());
            }
            Err("CDP connection closed".into())
        })
        .await
        .map_err(|_| "CDP request timeout".to_string())?
    }

    async fn evaluate(&mut self, expression: &str) -> Result<Value, String> {
        let result = self
            .request(
                "Runtime.evaluate",
                json!({"expression":expression,"returnByValue":true,"awaitPromise":true}),
                REQUEST_TIMEOUT,
            )
            .await?;
        if result.get("exceptionDetails").is_some() {
            return Err("Client rejected the localization script".into());
        }
        result["result"]
            .get("value")
            .cloned()
            .ok_or_else(|| "Client probe returned no value".into())
    }
}

async fn page(port: u16) -> Result<Cdp, String> {
    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let response = http
        .get(format!("http://127.0.0.1:{port}/json/list"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let response = response.error_for_status().map_err(|e| e.to_string())?;
    if response
        .content_length()
        .is_some_and(|size| size > 1024 * 1024)
    {
        return Err("CDP target list too large".into());
    }
    let targets: Vec<Value> = response.json().await.map_err(|e| e.to_string())?;
    let socket = targets
        .iter()
        .find_map(|target| target_socket(target, port))
        .ok_or("Official client main page not ready")?;
    Cdp::connect(&socket, port).await
}

pub(super) async fn probe(port: u16) -> Result<bool, String> {
    let mut client = page(port).await?;
    Ok(client.evaluate(STATUS).await?["ok"] == true)
}

pub(super) async fn localize(port: u16) -> Result<bool, String> {
    let mut client = page(port).await?;
    client
        .request("Page.enable", json!({}), REQUEST_TIMEOUT)
        .await?;
    client
        .request(
            "Page.addScriptToEvaluateOnNewDocument",
            json!({"source":EARLY}),
            REQUEST_TIMEOUT,
        )
        .await?;
    client.evaluate(EARLY).await?;
    client.evaluate(RUNTIME).await?;
    if client.evaluate(STATUS).await?["ok"] == true {
        return Ok(true);
    }
    client
        .request("Page.reload", json!({"ignoreCache":false}), REQUEST_TIMEOUT)
        .await?;
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(600)).await;
        // Navigation can temporarily destroy the execution context.
        if let Ok(value) = client.evaluate(STATUS).await {
            if value["ok"] == true {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    async fn fake_page(exception: bool) -> (u16, tokio::task::JoinHandle<Vec<String>>) {
        use futures::{SinkExt, StreamExt};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut http, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let read = http.read(&mut request).await.unwrap();
            assert!(std::str::from_utf8(&request[..read])
                .unwrap()
                .starts_with("GET /json/list "));
            let body = json!([{"type":"page", "url":"app://-/index.html", "webSocketDebuggerUrl":format!("ws://127.0.0.1:{port}/devtools/page/1")}]).to_string();
            http.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            drop(http);
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let mut methods = Vec::new();
            while let Some(Ok(message)) = socket.next().await {
                let value: serde_json::Value =
                    serde_json::from_str(message.to_text().unwrap()).unwrap();
                let method = value["method"].as_str().unwrap();
                methods.push(method.to_string());
                let result = if method == "Runtime.evaluate" {
                    if exception {
                        json!({"exceptionDetails":{"text":"fixture exception"}})
                    } else {
                        json!({"result":{"type":"object","value":{"ok":true,"reason":"locale-active"}}})
                    }
                } else {
                    json!({"identifier":"fixture-script"})
                };
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        json!({"id":value["id"],"result":result}).to_string().into(),
                    ))
                    .await
                    .unwrap();
            }
            methods
        });
        (port, server)
    }

    #[tokio::test]
    async fn localization_installs_early_script_then_checks_actual_page() {
        let (port, server) = fake_page(false).await;
        assert!(super::localize(port).await.unwrap());
        assert_eq!(
            server.await.unwrap(),
            vec![
                "Page.enable",
                "Page.addScriptToEvaluateOnNewDocument",
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate"
            ]
        );
    }

    #[tokio::test]
    async fn javascript_exception_cannot_report_active() {
        let (port, server) = fake_page(true).await;
        assert!(super::localize(port)
            .await
            .unwrap_err()
            .contains("rejected"));
        server.await.unwrap();
    }
    #[test]
    fn only_official_app_pages_and_exact_loopback_port_are_allowed() {
        let target = |url: &str, kind: &str| json!({"url": url, "type": kind, "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/1"});
        assert!(super::target_socket(&target("app://-/index.html", "page"), 9222).is_some());
        for url in [
            "https://chatgpt.com",
            "devtools://devtools/",
            "app://evil/index.html",
            "app://-/devtools/index.html",
            "app://-/index.html?window=devtools",
            "app://-:1234/index.html",
        ] {
            assert!(super::target_socket(&target(url, "page"), 9222).is_none());
        }
        assert!(super::target_socket(&target("app://-/index.html", "worker"), 9222).is_none());
        for url in [
            "ws://example.com:9222/devtools/page/1",
            "ws://127.0.0.1:9223/devtools/page/1",
            "ws://user@127.0.0.1:9222/devtools/page/1",
        ] {
            assert!(!super::valid_socket(url, 9222));
        }
    }
    #[tokio::test]
    async fn cdp_correlates_responses_and_reports_injection_errors_and_timeouts() {
        use futures::{SinkExt, StreamExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            for index in 0..3 {
                let request = socket.next().await.unwrap().unwrap();
                let value: serde_json::Value =
                    serde_json::from_str(request.to_text().unwrap()).unwrap();
                if index == 2 {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    break;
                }
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        json!({"method":"Runtime.event"}).to_string().into(),
                    ))
                    .await
                    .unwrap();
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        json!({"id":999,"result":{}}).to_string().into(),
                    ))
                    .await
                    .unwrap();
                let reply = if index == 0 {
                    json!({"id":value["id"],"result":{"answer":42}})
                } else {
                    json!({"id":value["id"],"error":{"message":"injected failure"}})
                };
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        reply.to_string().into(),
                    ))
                    .await
                    .unwrap();
            }
        });
        let mut client =
            super::Cdp::connect(&format!("ws://127.0.0.1:{port}/devtools/page/1"), port)
                .await
                .unwrap();
        assert_eq!(
            client
                .request(
                    "Runtime.evaluate",
                    json!({}),
                    std::time::Duration::from_secs(1)
                )
                .await
                .unwrap()["answer"],
            42
        );
        assert!(client
            .request(
                "Runtime.evaluate",
                json!({}),
                std::time::Duration::from_secs(1)
            )
            .await
            .unwrap_err()
            .contains("injected failure"));
        assert!(client
            .request(
                "Runtime.evaluate",
                json!({}),
                std::time::Duration::from_millis(40)
            )
            .await
            .unwrap_err()
            .contains("timeout"));
        server.await.unwrap();
    }
}
