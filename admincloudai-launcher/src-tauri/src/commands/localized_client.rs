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
