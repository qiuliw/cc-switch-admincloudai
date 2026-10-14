//! 管理端接口：上传新安装包、更新客服二维码。
//!
//! 鉴权：HTTP Basic（用户名任意，密码 = `ADMIN_PASSWORD` 环境变量）。
//! 未设置 `ADMIN_PASSWORD` 时管理接口整体禁用（返回 503）。

use axum::extract::{Multipart, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;

use crate::{error_json, read_versions, Shared};

/// 包上传大小上限（100 MB，NSIS 安装包通常 5–20 MB）
const MAX_PACKAGE_BYTES: usize = 100 * 1024 * 1024;
/// 二维码大小上限（2 MB）
const MAX_QR_BYTES: usize = 2 * 1024 * 1024;
/// 签名文件内容上限（minisign 签名实际 <1KB）
const MAX_SIG_BYTES: usize = 16 * 1024;

pub fn router() -> Router<Shared> {
    Router::new()
        .route("/api/admin/ping", get(ping))
        .route("/api/admin/package", post(upload_package))
        .route("/api/admin/qr", post(upload_qr))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_PACKAGE_BYTES))
}

/// 鉴权失败时的统一 401（带 WWW-Basic 提示，浏览器/前端可据此弹登录）
fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Basic realm=\"admin\"")],
        Json(serde_json::json!({ "error": "需要管理员密码" })),
    )
        .into_response()
}

/// 恒时比较，避免逐字符短路泄露前缀
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// 校验 Basic 凭据；密码未配置时返回 503，凭据错误返回 401。
fn ensure_admin(shared: &Shared, headers: &HeaderMap) -> Result<(), Response> {
    let Some(expected) = shared.admin_password.as_deref() else {
        return Err(error_json(
            StatusCode::SERVICE_UNAVAILABLE,
            "管理端未启用：未设置 ADMIN_PASSWORD",
        ));
    };
    let Some(raw) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return Err(unauthorized());
    };
    let Some(encoded) = raw.strip_prefix("Basic ") else {
        return Err(unauthorized());
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok());
    let Some(credentials) = decoded else {
        return Err(unauthorized());
    };
    // 用户名占位（约定 "admin"），只校验密码段
    let password = credentials.split_once(':').map(|(_, pass)| pass);
    match password {
        Some(pass) if ct_eq(pass.as_bytes(), expected.as_bytes()) => Ok(()),
        _ => Err(unauthorized()),
    }
}

/// 版本号：仅允许字母数字与 . _ + -，长度 ≤ 32
fn sanitize_version(raw: &str) -> Option<String> {
    let version = raw.trim().trim_start_matches('v');
    let ok = !version.is_empty()
        && version.len() <= 32
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'));
    ok.then(|| version.to_string())
}

/// 安装包文件名：取末段、仅保留安全字符，必须以 .exe 结尾
fn sanitize_filename(raw: &str) -> Option<String> {
    let name = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let safe: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    let ok = !safe.is_empty()
        && safe.len() <= 128
        && safe.eq_ignore_ascii_case(&name.trim().to_string())
        && safe.to_ascii_lowercase().ends_with(".exe");
    ok.then(|| safe)
}

/// 用上传内容更新发版清单（纯函数，便于测试）：返回新的 JSON 值。
fn apply_release(
    mut manifest: serde_json::Value,
    version: &str,
    notes: &str,
    signature: &str,
    download_url: &str,
) -> serde_json::Value {
    if !manifest.is_object() {
        manifest = serde_json::json!({});
    }
    let object = manifest.as_object_mut().expect("checked object");
    object.insert("version".into(), version.into());
    if notes.trim().is_empty() {
        object.remove("notes");
    } else {
        object.insert("notes".into(), notes.into());
    }
    object.remove("pub_date");
    let platforms = object
        .entry("platforms")
        .or_insert_with(|| serde_json::json!({}));
    if let Some(platforms) = platforms.as_object_mut() {
        platforms.insert(
            "windows-x86_64".into(),
            serde_json::json!({ "signature": signature, "url": download_url }),
        );
    }
    manifest
}

/// 原子写文件：先写同目录临时文件再改名，避免写一半被读走
async fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, bytes)
        .await
        .map_err(|e| format!("写入失败: {e}"))?;
    tokio::fs::rename(&tmp, path)
        .await
        .map_err(|e| format!("替换失败: {e}"))
}

async fn ping(State(shared): State<Shared>, headers: HeaderMap) -> Response {
    if let Err(response) = ensure_admin(&shared, &headers) {
        return response;
    }
    (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response()
}

/// 上传新安装包并更新发版清单。
/// multipart 字段：file（.exe）、sig（.sig 内容）、version、notes（可选）。
async fn upload_package(
    State(shared): State<Shared>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    if let Err(response) = ensure_admin(&shared, &headers) {
        return response;
    }

    let mut version = String::new();
    let mut notes = String::new();
    let mut signature: Option<Vec<u8>> = None;
    let mut package: Option<(String, Vec<u8>)> = None;

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => match field.name() {
                Some("version") => {
                    version = field.text().await.unwrap_or_default();
                }
                Some("notes") => {
                    notes = field.text().await.unwrap_or_default();
                }
                Some("sig") => {
                    let name = field.file_name().map(str::to_string);
                    let bytes = match field.bytes().await {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            return error_json(
                                StatusCode::BAD_REQUEST,
                                &format!("读取签名失败: {e}"),
                            )
                        }
                    };
                    match name {
                        // .sig 以文件形式上传
                        Some(_) => signature = Some(bytes.to_vec()),
                        // 以纯文本字段粘贴
                        None => signature = Some(bytes.to_vec()),
                    }
                }
                Some("file") => {
                    let Some(name) = field.file_name().map(str::to_string) else {
                        return error_json(StatusCode::BAD_REQUEST, "缺少安装包文件名");
                    };
                    let bytes = match field.bytes().await {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            return error_json(
                                StatusCode::BAD_REQUEST,
                                &format!("读取安装包失败: {e}"),
                            )
                        }
                    };
                    package = Some((name, bytes.to_vec()));
                }
                _ => {}
            },
            Ok(None) => break,
            Err(e) => {
                return error_json(
                    StatusCode::BAD_REQUEST,
                    &format!("解析上传失败（可能超过 100MB）: {e}"),
                )
            }
        }
    }

    let Some(version) = sanitize_version(&version) else {
        return error_json(StatusCode::BAD_REQUEST, "版本号非法（仅允许字母数字与 . _ + -）");
    };
    let Some((raw_name, bytes)) = package else {
        return error_json(StatusCode::BAD_REQUEST, "缺少安装包文件");
    };
    if bytes.is_empty() || bytes.len() > MAX_PACKAGE_BYTES {
        return error_json(StatusCode::BAD_REQUEST, "安装包为空或超过 100MB");
    }
    let Some(filename) = sanitize_filename(&raw_name) else {
        return error_json(StatusCode::BAD_REQUEST, "文件名非法（需为 .exe）");
    };
    let Some(signature_bytes) = signature else {
        return error_json(StatusCode::BAD_REQUEST, "缺少 .sig 签名内容");
    };
    if signature_bytes.is_empty() || signature_bytes.len() > MAX_SIG_BYTES {
        return error_json(StatusCode::BAD_REQUEST, "签名为空或超过 16KB");
    }
    let signature = String::from_utf8_lossy(&signature_bytes).trim().to_string();

    let updates_dir = shared.data_dir.join("updates");
    if let Err(e) = tokio::fs::create_dir_all(&updates_dir).await {
        tracing::error!("创建 updates 目录失败: {e}");
        return error_json(StatusCode::INTERNAL_SERVER_ERROR, "服务器存储异常");
    }
    let package_path = updates_dir.join(&filename);
    if let Err(e) = atomic_write(&package_path, &bytes).await {
        tracing::error!("写入 {} 失败: {e}", package_path.display());
        return error_json(StatusCode::INTERNAL_SERVER_ERROR, "安装包写入失败");
    }

    let download_url = format!("{}/downloads/{filename}", shared.public_base);
    let manifest = read_versions(&shared.data_dir).unwrap_or_else(|| serde_json::json!({}));
    let updated = apply_release(manifest, &version, &notes, &signature, &download_url);
    let manifest_path = shared.data_dir.join("versions.json");
    let mut serialized = serde_json::to_vec_pretty(&updated).unwrap_or_default();
    serialized.push(b'\n');
    if let Err(e) = atomic_write(&manifest_path, &serialized).await {
        tracing::error!("写入 versions.json 失败: {e}");
        return error_json(StatusCode::INTERNAL_SERVER_ERROR, "发版清单写入失败");
    }

    tracing::info!("已发布版本 {version}（{filename}，{} 字节）", bytes.len());
    (
        StatusCode::OK,
        Json(serde_json::json!({ "ok": true, "version": version, "url": download_url })),
    )
        .into_response()
}

/// 更新客服二维码（仅接受 PNG，覆盖 data/qr.png）。
async fn upload_qr(
    State(shared): State<Shared>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    if let Err(response) = ensure_admin(&shared, &headers) {
        return response;
    }

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }
        let is_png = field
            .content_type()
            .map(|ct| ct == "image/png")
            .unwrap_or(false);
        if !is_png {
            return error_json(StatusCode::BAD_REQUEST, "二维码仅支持 PNG 图片");
        }
        let bytes = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => return error_json(StatusCode::BAD_REQUEST, &format!("读取失败: {e}")),
        };
        if bytes.is_empty() || bytes.len() > MAX_QR_BYTES {
            return error_json(StatusCode::BAD_REQUEST, "二维码为空或超过 2MB");
        }
        let qr_path = shared.data_dir.join("qr.png");
        if let Err(e) = atomic_write(&qr_path, &bytes).await {
            tracing::error!("写入 {} 失败: {e}", qr_path.display());
            return error_json(StatusCode::INTERNAL_SERVER_ERROR, "二维码写入失败");
        }
        tracing::info!("已更新客服二维码（{} 字节）", bytes.len());
        return (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response();
    }
    error_json(StatusCode::BAD_REQUEST, "缺少 multipart 字段 file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_sanitizing() {
        assert_eq!(
            sanitize_version(" v4.0.10-beta+1 "),
            Some("4.0.10-beta+1".into())
        );
        assert_eq!(sanitize_version(""), None);
        assert_eq!(sanitize_version("../evil"), None);
        assert_eq!(sanitize_version(&"a".repeat(33)), None);
    }

    #[test]
    fn filename_sanitizing() {
        assert_eq!(
            sanitize_filename("C:\\Downloads\\AdminCloudAI-Setup_4.1.exe"),
            Some("AdminCloudAI-Setup_4.1.exe".into())
        );
        assert_eq!(
            sanitize_filename("../../evil.exe"),
            Some("evil.exe".into())
        );
        assert_eq!(sanitize_filename("payload.txt"), None);
        assert_eq!(sanitize_filename(""), None);
    }

    #[test]
    fn constant_time_compare() {
        assert!(ct_eq(b"secret", b"secret"));
        assert!(!ct_eq(b"secret", b"secrets"));
        assert!(!ct_eq(b"secret", b"secretx"));
        assert!(ct_eq(b"", b""));
    }

    #[test]
    fn release_manifest_update() {
        let existing = serde_json::json!({
            "version": "4.0.9",
            "notes": "old",
            "pub_date": "2026-01-01T00:00:00Z",
            "platforms": { "windows-x86_64": { "signature": "old", "url": "old" } }
        });
        let updated = apply_release(existing, "4.1.0", "new notes", "SIG", "https://x/d.exe");
        assert_eq!(updated["version"], "4.1.0");
        assert_eq!(updated["notes"], "new notes");
        assert!(updated.get("pub_date").is_none());
        assert_eq!(updated["platforms"]["windows-x86_64"]["signature"], "SIG");
        assert_eq!(
            updated["platforms"]["windows-x86_64"]["url"],
            "https://x/d.exe"
        );

        // 非对象内容（损坏清单）也能重建
        let repaired = apply_release(serde_json::json!([]), "1.0", "", "SIG", "u");
        assert_eq!(repaired["version"], "1.0");
        assert!(repaired.get("notes").is_none());
        assert_eq!(repaired["platforms"]["windows-x86_64"]["url"], "u");
    }
}
