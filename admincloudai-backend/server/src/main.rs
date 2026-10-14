//! AdminCloudAI 后端服务：在线升级 feed、在线客服二维码、管理端上传。
//!
//! 极简设计：单二进制 + 数据目录（卷挂载），无数据库。
//! - `data/versions.json`  - 升级信息（Tauri v2 updater 格式），管理端上传或改文件即发版
//! - `data/qr.png`         - 客服二维码图片，管理端上传或替换文件即轮换
//! - `data/updates/`       - 安装包（配合 versions.json 里的 url）

mod admin;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use axum::extract::{Path as AxPath, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::Value;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
struct Shared {
    data_dir: PathBuf,
    /// 管理端密码；未设置则管理接口禁用
    admin_password: Option<String>,
    /// 对外基础地址（拼接安装包下载链接）
    public_base: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "admincloudai_server=info,tower_http=warn".into()),
        )
        .init();

    let bind: SocketAddr = std::env::var("BIND")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| "0.0.0.0:8080".parse().unwrap());
    let data_dir =
        PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "data".into()));
    let web_dir =
        PathBuf::from(std::env::var("WEB_DIR").unwrap_or_else(|_| "web-dist".into()));
    let admin_password = std::env::var("ADMIN_PASSWORD").ok().filter(|p| !p.is_empty());
    let public_base = std::env::var("PUBLIC_BASE_URL")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("http://{bind}"))
        .trim_end_matches('/')
        .to_string();

    let _ = std::fs::create_dir_all(data_dir.join("updates"));

    let shared = Shared {
        data_dir,
        admin_password,
        public_base,
    };
    let admin_enabled = shared.admin_password.is_some();

    let app = Router::new()
        .route("/api/update/{target}/{current}", get(update_handler))
        .route("/api/qr", get(qr_handler))
        .route("/health", get(|| async { "ok" }))
        .merge(admin::router())
        .nest_service("/downloads", ServeDir::new(shared.data_dir.join("updates")))
        .fallback_service(
            ServeDir::new(&web_dir)
                .append_index_html_on_directories(true)
                // SPA 单页：未知路径回落到首页
                .fallback(ServeDir::new(&web_dir).append_index_html_on_directories(true)),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(shared);

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .unwrap_or_else(|e| panic!("监听 {bind} 失败: {e}"));
    tracing::info!(
        "AdminCloudAI server listening on http://{bind}（管理端：{}）",
        if admin_enabled { "已启用" } else { "未启用（未设置 ADMIN_PASSWORD）" }
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("server error");
}

/// 读取 data/versions.json；不存在或损坏返回 None。
fn read_versions(data_dir: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(data_dir.join("versions.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

fn error_json(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

/// 语义化版本比较（只比数字段，非法段按 0 处理）
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim_start_matches('v')
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parse(latest), parse(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// Tauri v2 updater feed：`GET /api/update/{target}/{current_version}`
/// 返回 200 + JSON（有新版）或 204（无更新 / 未发版）。
async fn update_handler(
    State(shared): State<Shared>,
    AxPath((target, current)): AxPath<(String, String)>,
) -> Response {
    let Some(versions) = read_versions(&shared.data_dir) else {
        return StatusCode::NO_CONTENT.into_response();
    };
    let Some(latest) = versions.get("version").and_then(|v| v.as_str()) else {
        return StatusCode::NO_CONTENT.into_response();
    };
    if !is_newer(latest, &current) {
        return StatusCode::NO_CONTENT.into_response();
    }
    // 客户端要的 target 不在 platforms 里时按无更新处理
    if !target.is_empty() {
        if versions.get("platforms").and_then(|p| p.get(&target)).is_none() {
            tracing::warn!("versions.json 缺少平台 {target}，按无更新处理");
            return StatusCode::NO_CONTENT.into_response();
        }
    }
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    (headers, Json(versions)).into_response()
}

/// 在线客服二维码：优先 302 跳转 versions.json 里的 qr_url，
/// 否则直接返回 data/qr.png。都没有则 404（客户端回落本地内置图）。
async fn qr_handler(State(shared): State<Shared>) -> Response {
    if let Some(url) = read_versions(&shared.data_dir)
        .and_then(|v| v.get("qr_url").and_then(|u| u.as_str()).map(String::from))
    {
        if !url.is_empty() {
            let mut headers = HeaderMap::new();
            headers.insert(header::LOCATION, url.parse().unwrap());
            headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
            return (StatusCode::FOUND, headers).into_response();
        }
    }
    match std::fs::read(shared.data_dir.join("qr.png")) {
        Ok(bytes) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
            headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
            (headers, bytes).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("4.0.10", "4.0.9"));
        assert!(is_newer("5.0", "4.9.9"));
        assert!(!is_newer("4.0.9", "4.0.9"));
        assert!(!is_newer("4.0.8", "4.0.9"));
        assert!(!is_newer("abc", "4.0.9")); // 非法版本不触发更新
    }
}
