mod cdp;
mod windows;

use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedClient {
    pub name: String,
    pub executable: String,
    pub source: String,
    pub version: Option<String>,
    pub app_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClientState {
    NotFound,
    Ready,
    Launching,
    Pending,
    Active,
    Blocked,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedClientStatus {
    pub supported: bool,
    pub configured: bool,
    pub active: bool,
    pub state: ClientState,
    pub client: Option<LocalizedClient>,
    pub message: String,
    pub reason: Option<String>,
    pub debug_port: Option<u16>,
}

impl Default for LocalizedClientStatus {
    fn default() -> Self {
        Self {
            supported: cfg!(windows),
            configured: false,
            active: false,
            state: ClientState::NotFound,
            client: None,
            message: if cfg!(windows) {
                "尚未检测客户端"
            } else {
                "中文客户端目前仅支持 Windows"
            }
            .into(),
            reason: None,
            debug_port: None,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientConfig {
    path: Option<String>,
    debug_port: Option<u16>,
    debug_executable: Option<String>,
}

impl ClientConfig {
    fn load() -> Result<Self, String> {
        let path = crate::config::get_app_config_dir().join("localized-client.json");
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|_| "客户端配置损坏，请重置客户端路径".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(format!("无法读取客户端配置: {error}")),
        }
    }
    fn save(&self) -> Result<(), String> {
        crate::config::write_json_file(
            &crate::config::get_app_config_dir().join("localized-client.json"),
            self,
        )
        .map_err(|e| e.to_string())
    }
    fn port_for(&self, client: &LocalizedClient) -> Option<u16> {
        self.debug_executable
            .as_ref()
            .filter(|path| path.eq_ignore_ascii_case(&client.executable))
            .and(self.debug_port)
            .filter(|port| *port != 0)
    }
}

#[derive(Default)]
pub struct LocalizedClientService {
    operation: Mutex<()>,
    status: Mutex<LocalizedClientStatus>,
}

#[derive(Clone, Copy)]
pub enum Action {
    Scan,
    OpenChinese,
    RestartChinese,
    OpenOriginal,
}

type Notify<'a> = &'a (dyn Fn(&LocalizedClientStatus) + Send + Sync);

fn verified_active(shell_verified: bool) -> bool {
    shell_verified
}

impl LocalizedClientService {
    async fn publish(
        &self,
        status: LocalizedClientStatus,
        notify: Notify<'_>,
    ) -> LocalizedClientStatus {
        let mut previous = self.status.lock().await;
        if *previous != status {
            *previous = status.clone();
            notify(&status);
        }
        status
    }

    pub async fn get_status(&self, notify: Notify<'_>) -> LocalizedClientStatus {
        self.run(Action::Scan, notify).await
    }

    pub async fn set_path(
        &self,
        path: Option<String>,
        notify: Notify<'_>,
    ) -> Result<LocalizedClientStatus, String> {
        let guard = self
            .operation
            .try_lock()
            .map_err(|_| "客户端操作正在进行，请稍后重试".to_string())?;
        let path = path
            .map(|path| {
                windows::validate_path(Path::new(&path))
                    .map(|path| path.to_string_lossy().into_owned())
            })
            .transpose()?;
        ClientConfig {
            path,
            ..Default::default()
        }
        .save()?;
        // Reset stale active evidence before discovering the newly selected executable.
        self.publish(LocalizedClientStatus::default(), notify).await;
        let result =
            tokio::time::timeout(Duration::from_secs(25), self.perform(Action::Scan, notify)).await;
        let status = self.finish(result, notify).await;
        drop(guard);
        Ok(status)
    }

    pub async fn run(&self, action: Action, notify: Notify<'_>) -> LocalizedClientStatus {
        let Ok(_guard) = self.operation.try_lock() else {
            // Reject this request without replacing the workflow's shared progress.
            let mut busy = self.status.lock().await.clone();
            busy.active = false;
            busy.state = ClientState::Blocked;
            busy.reason = Some("operation-in-progress".into());
            busy.message = "客户端操作正在进行，请稍后重试".into();
            return busy;
        };
        if !cfg!(windows) {
            return self.publish(LocalizedClientStatus::default(), notify).await;
        }
        let result =
            tokio::time::timeout(Duration::from_secs(55), self.perform(action, notify)).await;
        self.finish(result, notify).await
    }

    async fn finish(
        &self,
        result: Result<Result<LocalizedClientStatus, String>, tokio::time::error::Elapsed>,
        notify: Notify<'_>,
    ) -> LocalizedClientStatus {
        match result {
            Ok(Ok(status)) => self.publish(status, notify).await,
            error => {
                let message = match error {
                    Ok(Err(message)) => message,
                    _ => "客户端操作超时，请检查客户端后重试".into(),
                };
                let mut status = self.status.lock().await.clone();
                status.active = false;
                status.state = ClientState::Error;
                status.reason = Some(message.clone());
                status.message = message;
                self.publish(status, notify).await
            }
        }
    }

    async fn perform(
        &self,
        action: Action,
        notify: Notify<'_>,
    ) -> Result<LocalizedClientStatus, String> {
        let mut config = ClientConfig::load()?;
        let mut status = LocalizedClientStatus {
            configured: config.path.is_some(),
            ..Default::default()
        };
        let Some(client) = windows::discover(config.path.as_deref()).await? else {
            status.message = "未找到官方 Codex 或 ChatGPT 客户端，请选择安装路径".into();
            return Ok(status);
        };
        status.client = Some(client.clone());
        status.state = ClientState::Ready;
        status.message = "已找到客户端，可以启动中文界面".into();
        let mut port = config.port_for(&client);
        let running = windows::running(&client, port).await?;
        status.debug_port = if running.controlled { port } else { None };
        if matches!(action, Action::Scan) {
            if running.controlled {
                let proof = cdp::probe(port.ok_or("受控端口缺失")?)
                    .await
                    .unwrap_or(false);
                status.active = verified_active(proof);
                status.state = if status.active {
                    ClientState::Active
                } else {
                    ClientState::Pending
                };
                status.message = if status.active {
                    "客户端中文界面已验证"
                } else {
                    "客户端运行中，中文界面尚未验证"
                }
                .into();
            } else if running.running {
                status.state = ClientState::Blocked;
                status.reason = Some("restart-required".into());
                status.message = "客户端正在运行，需要确认重启后启用中文".into();
            }
            return Ok(status);
        }
        if matches!(action, Action::OpenOriginal) {
            if running.running {
                status.state = ClientState::Blocked;
                status.reason = Some("client-already-running".into());
                status.message = "客户端已运行，请先自行关闭客户端再原版启动".into();
            } else {
                windows::launch(&client, None).await?;
                status.message = "已原版启动客户端".into();
            }
            return Ok(status);
        }
        if running.running && !running.controlled && !matches!(action, Action::RestartChinese) {
            status.state = ClientState::Blocked;
            status.reason = Some("restart-required".into());
            status.message = "客户端正在运行，需要确认重启后启用中文".into();
            return Ok(status);
        }
        status.state = ClientState::Launching;
        status.message = "正在准备中文客户端".into();
        self.publish(status.clone(), notify).await;
        if matches!(action, Action::RestartChinese) {
            windows::stop(&client).await?;
            port = None;
        }
        if !running.running || matches!(action, Action::RestartChinese) {
            // Hold an exclusive socket while selecting a fresh port; never reuse a stale endpoint.
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                .map_err(|e| e.to_string())?;
            let selected = listener.local_addr().map_err(|e| e.to_string())?.port();
            config.debug_port = Some(selected);
            config.debug_executable = Some(client.executable.clone());
            config.save()?;
            drop(listener);
            windows::launch(&client, Some(selected)).await?;
            port = Some(selected);
        }
        let port = port.ok_or("无法分配客户端调试端口")?;
        status.debug_port = Some(port);
        status.state = ClientState::Pending;
        status.message = "正在等待客户端中文界面验证".into();
        self.publish(status.clone(), notify).await;
        let mut last_error = String::new();
        for _ in 0..12 {
            // Port metadata alone is insufficient: the exact executable must carry our flags.
            if windows::running(&client, Some(port)).await?.controlled {
                match cdp::localize(port).await {
                    Ok(proof) => {
                        status.active = verified_active(proof);
                        status.state = if status.active {
                            ClientState::Active
                        } else {
                            ClientState::Pending
                        };
                        status.message = if status.active {
                            "客户端中文界面已验证"
                        } else {
                            "已注入语言设置，但中文外壳尚未验证；可稍后重新扫描"
                        }
                        .into();
                        return Ok(status);
                    }
                    Err(error) => last_error = error,
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(format!("无法连接客户端受控调试页面: {last_error}"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn launch_evidence_never_means_active() {
        let status = super::LocalizedClientStatus::default();
        assert!(!status.active);
        assert!(!super::verified_active(false));
        assert!(super::verified_active(true));
    }
    #[tokio::test]
    async fn operations_are_exclusive_and_release_on_drop() {
        let service = super::LocalizedClientService::default();
        let guard = service.operation.try_lock().unwrap();
        assert!(service.operation.try_lock().is_err());
        drop(guard);
        assert!(service.operation.try_lock().is_ok());
    }

    #[tokio::test]
    async fn explicit_actions_report_busy_instead_of_cached_success() {
        let service = super::LocalizedClientService::default();
        let cached = super::LocalizedClientStatus {
            active: true,
            state: super::ClientState::Active,
            ..Default::default()
        };
        *service.status.lock().await = cached.clone();
        let _in_progress = service.operation.lock().await;
        for action in [
            super::Action::OpenChinese,
            super::Action::RestartChinese,
            super::Action::OpenOriginal,
        ] {
            let status = service.run(action, &|_| {}).await;
            assert_eq!(status.state, super::ClientState::Blocked);
            assert_eq!(status.reason.as_deref(), Some("operation-in-progress"));
            assert!(!status.active);
        }
        assert_eq!(
            *service.status.lock().await,
            cached,
            "A rejected request must not overwrite the running workflow's status"
        );
    }

    #[tokio::test]
    async fn failure_clears_active_and_emits_the_complete_status_once() {
        let service = super::LocalizedClientService::default();
        let events = std::sync::Mutex::new(Vec::new());
        let notify =
            |status: &super::LocalizedClientStatus| events.lock().unwrap().push(status.clone());
        let status = super::LocalizedClientStatus {
            active: true,
            state: super::ClientState::Active,
            ..Default::default()
        };
        service.publish(status.clone(), &notify).await;
        service.publish(status, &notify).await;
        let failed = service
            .finish(Ok(Err("fixture failure".into())), &notify)
            .await;
        assert!(!failed.active);
        assert_eq!(failed.state, super::ClientState::Error);
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1], failed);
        assert_eq!(events[1].reason.as_deref(), Some("fixture failure"));
    }
}
