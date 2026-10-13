use crate::localized_client::{Action, LocalizedClientService, LocalizedClientStatus};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

fn emit(app: &AppHandle, status: &LocalizedClientStatus) {
    if let Err(error) = app.emit("chinese-chatgpt-status-updated", status) {
        log::warn!("Could not emit localized client status: {error}");
    }
}

#[tauri::command]
pub async fn get_chinese_chatgpt_status(
    app: AppHandle,
    service: State<'_, LocalizedClientService>,
) -> Result<LocalizedClientStatus, String> {
    Ok(service.get_status(&|status| emit(&app, status)).await)
}

#[tauri::command]
pub async fn scan_chinese_chatgpt(
    app: AppHandle,
    service: State<'_, LocalizedClientService>,
) -> Result<LocalizedClientStatus, String> {
    Ok(service
        .run(Action::Scan, &|status| emit(&app, status))
        .await)
}

#[tauri::command]
pub async fn open_chinese_chatgpt(
    app: AppHandle,
    service: State<'_, LocalizedClientService>,
) -> Result<LocalizedClientStatus, String> {
    Ok(service
        .run(Action::OpenChinese, &|status| emit(&app, status))
        .await)
}

#[tauri::command]
pub async fn restart_chinese_chatgpt(
    app: AppHandle,
    service: State<'_, LocalizedClientService>,
) -> Result<LocalizedClientStatus, String> {
    Ok(service
        .run(Action::RestartChinese, &|status| emit(&app, status))
        .await)
}

#[tauri::command]
pub async fn open_chatgpt_client(
    app: AppHandle,
    service: State<'_, LocalizedClientService>,
) -> Result<LocalizedClientStatus, String> {
    Ok(service
        .run(Action::OpenOriginal, &|status| emit(&app, status))
        .await)
}

#[tauri::command]
pub async fn pick_chinese_chatgpt_path(app: AppHandle) -> Result<Option<String>, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Codex / ChatGPT", &["exe"])
        .pick_file(move |file| {
            let _ = sender.send(file.map(|path| path.to_string()));
        });
    receiver.await.map_err(|_| "文件选择窗口已关闭".into())
}

#[tauri::command]
pub async fn set_chinese_chatgpt_path(
    path: Option<String>,
    app: AppHandle,
    service: State<'_, LocalizedClientService>,
) -> Result<LocalizedClientStatus, String> {
    service.set_path(path, &|status| emit(&app, status)).await
}

/// 拉取在线客服二维码（AdminCloudAI 服务端可随时轮换）。
/// 地址与升级 feed 一样配置在 tauri.conf.json 的 plugins 段（plugins.admincloud.qrUrl）。
/// 返回 data URL 供 <img> 直接使用；服务不可达或响应非图片时返回 None，前端回落内置图片。
#[tauri::command]
pub async fn get_online_support_qr(app: AppHandle) -> Result<Option<String>, String> {
    const MAX_QR_BYTES: u64 = 2 * 1024 * 1024;

    let Some(qr_url) = app
        .config()
        .plugins
        .0
        .get("admincloud")
        .and_then(|v| v.get("qrUrl"))
        .and_then(|v| v.as_str())
    else {
        return Ok(None);
    };

    let response = tokio::time::timeout(std::time::Duration::from_secs(10), reqwest::get(qr_url))
        .await
        .map_err(|_| "请求超时".to_string())?
        .map_err(|e| format!("请求失败: {e}"))?;

    if !response.status().is_success() {
        return Ok(None);
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .split(';')
        .next()
        .unwrap_or("image/png")
        .to_string();
    if !content_type.starts_with("image/") {
        return Ok(None);
    }
    let bytes = tokio::time::timeout(std::time::Duration::from_secs(10), response.bytes())
        .await
        .map_err(|_| "下载超时".to_string())?
        .map_err(|e| format!("读取失败: {e}"))?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_QR_BYTES {
        return Ok(None);
    }
    use base64::Engine as _;
    Ok(Some(format!(
        "data:{content_type};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    )))
}
