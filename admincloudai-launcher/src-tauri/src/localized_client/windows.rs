use super::LocalizedClient;
use std::path::{Path, PathBuf};

pub(super) fn validate_path(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    let normalized = path.to_string_lossy().replace('\\', "/").to_lowercase();
    if !path.is_absolute()
        || !matches!(name.as_str(), "codex.exe" | "chatgpt.exe")
        || [
            "/node_modules/",
            "/.cargo/",
            "/.codex/bin/",
            "/.local/bin/",
            "/npm/",
            "/vendor/",
        ]
        .iter()
        .any(|part| normalized.contains(part))
    {
        return Err("请选择官方 Codex.exe 或 ChatGPT.exe 图形客户端，不能选择 CLI".into());
    }
    let canonical =
        std::fs::canonicalize(path).map_err(|_| "客户端路径不存在或无法访问".to_string())?;
    let normalized = canonical
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    if [
        "/node_modules/",
        "/.cargo/",
        "/.codex/bin/",
        "/.local/bin/",
        "/npm/",
        "/vendor/",
    ]
    .iter()
    .any(|part| normalized.contains(part))
    {
        return Err("不能选择 CLI 客户端".into());
    }
    validate_gui_executable(&canonical)?;
    Ok(canonical)
}

fn validate_gui_executable(path: &Path) -> Result<(), String> {
    use std::io::{Read, Seek, SeekFrom};
    let check = || -> std::io::Result<bool> {
        let mut file = std::fs::File::open(path)?;
        let mut dos = [0; 64];
        file.read_exact(&mut dos)?;
        if &dos[..2] != b"MZ" {
            return Ok(false);
        }
        let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
        if offset < 64 || offset > file.metadata()?.len().saturating_sub(94) {
            return Ok(false);
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut pe = [0; 94];
        file.read_exact(&mut pe)?;
        let magic = u16::from_le_bytes([pe[24], pe[25]]);
        let subsystem = u16::from_le_bytes([pe[92], pe[93]]);
        Ok(&pe[..4] == b"PE\0\0" && matches!(magic, 0x10b | 0x20b) && subsystem == 2)
    };
    match check() {
        Ok(true) => Ok(()),
        _ => Err("请选择 Windows 图形客户端，不能选择命令行程序或无效 EXE".into()),
    }
}

fn client(path: &Path, source: &str) -> Result<LocalizedClient, String> {
    let executable = validate_path(path)?;
    Ok(LocalizedClient {
        name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        executable: executable.to_string_lossy().into_owned(),
        source: source.into(),
        version: None,
        app_id: None,
    })
}

#[cfg(windows)]
async fn powershell(script: &str, variables: &[(&str, &str)]) -> Result<String, String> {
    use base64::Engine;
    use std::os::windows::process::CommandExt;
    let encoded = base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let mut command = std::process::Command::new("powershell.exe");
    command.creation_flags(0x08000000);
    command.args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded]);
    for (name, value) in variables {
        command.env(name, value);
    }
    let mut command = tokio::process::Command::from(command);
    command.kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(12), command.output())
        .await
        .map_err(|_| "客户端检测或操作超时".to_string())?
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("Windows 客户端操作失败，请检查安装和访问权限".into());
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

#[cfg(windows)]
const DISCOVERY: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$found = @()
Get-AppxPackage -Name OpenAI.Codex -ErrorAction SilentlyContinue | ForEach-Object {
  $package = $_
  try {
    $manifest = Get-AppxPackageManifest -Package $package.PackageFullName
    foreach ($application in $manifest.Package.Applications.Application) {
      $exe = Join-Path $package.InstallLocation ([string]$application.Executable)
      if ([IO.Path]::GetFileName($exe) -match '^(Codex|ChatGPT)\.exe$') {
        $found += @{name=[IO.Path]::GetFileNameWithoutExtension($exe); executable=$exe; source='Microsoft Store'; version=[string]$package.Version; appId=($package.PackageFamilyName + '!' + [string]$application.Id)}
      }
    }
  } catch {}
}
Get-CimInstance Win32_Process -Filter "Name='Codex.exe' OR Name='ChatGPT.exe'" -ErrorAction SilentlyContinue | ForEach-Object {
  if ($_.ExecutablePath) { $found += @{name=[IO.Path]::GetFileNameWithoutExtension($_.ExecutablePath); executable=$_.ExecutablePath; source='running'; version=$null; appId=$null} }
}
ConvertTo-Json -InputObject @($found) -Compress -Depth 4
"#;

#[cfg(windows)]
pub(super) async fn discover(manual: Option<&str>) -> Result<Option<LocalizedClient>, String> {
    discover_with_lookup(manual, async {
        let output = powershell(DISCOVERY, &[]).await?;
        serde_json::from_str(output.trim()).map_err(|_| "客户端检测结果无效".to_string())
    })
    .await
}

#[cfg(windows)]
async fn discover_with_lookup(
    manual: Option<&str>,
    lookup: impl std::future::Future<Output = Result<Vec<LocalizedClient>, String>>,
) -> Result<Option<LocalizedClient>, String> {
    if let Some(path) = manual {
        // A saved path is authoritative; package discovery only enriches activation metadata.
        let mut selected = client(Path::new(path), "manual")?;
        let candidates = lookup.await.unwrap_or_else(|error| {
            log::debug!("Optional client package discovery failed: {error}");
            Vec::new()
        });
        for candidate in &candidates {
            if let Ok(canonical) = validate_path(Path::new(&candidate.executable)) {
                if canonical
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&selected.executable)
                {
                    selected.app_id = candidate.app_id.clone();
                    selected.version = candidate.version.clone();
                    break;
                }
            }
        }
        return Ok(Some(selected));
    }
    let candidates = lookup.await?;
    for mut candidate in candidates {
        if let Ok(path) = validate_path(Path::new(&candidate.executable)) {
            candidate.executable = path.to_string_lossy().into_owned();
            return Ok(Some(candidate));
        }
    }
    let mut paths = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        for suffix in [
            "Programs/Codex/Codex.exe",
            "Programs/ChatGPT/ChatGPT.exe",
            "Codex/Codex.exe",
            "ChatGPT/ChatGPT.exe",
        ] {
            paths.push(PathBuf::from(&local).join(suffix));
        }
    }
    if let Some(programs) = std::env::var_os("ProgramFiles") {
        for suffix in ["Codex/Codex.exe", "ChatGPT/ChatGPT.exe"] {
            paths.push(PathBuf::from(&programs).join(suffix));
        }
    }
    Ok(paths.iter().find_map(|path| client(path, "standard").ok()))
}

#[cfg(not(windows))]
pub(super) async fn discover(_manual: Option<&str>) -> Result<Option<LocalizedClient>, String> {
    Ok(None)
}

#[derive(serde::Deserialize, Default)]
pub(super) struct Running {
    pub running: bool,
    pub controlled: bool,
}

#[cfg(windows)]
const PROCESS_STATE: &str = r#"
$ErrorActionPreference='Stop'
$selected = $env:ADMINCLOUD_CLIENT_PATH -replace '^\\\\\?\\',''
$items = @(Get-CimInstance Win32_Process -Filter "Name='Codex.exe' OR Name='ChatGPT.exe'" | Where-Object { $_.ExecutablePath -and ($_.ExecutablePath -ieq $selected) })
$controlled = $false
foreach ($item in $items) {
  if ($env:ADMINCLOUD_DEBUG_PORT -and $item.CommandLine -match ('(?:^|\s)--remote-debugging-port=' + [regex]::Escape($env:ADMINCLOUD_DEBUG_PORT) + '(?:\s|$)') -and $item.CommandLine -match '(?:^|\s)--remote-debugging-address=127\.0\.0\.1(?:\s|$)') {
    $owners = @(Get-NetTCPConnection -State Listen -LocalPort ([int]$env:ADMINCLOUD_DEBUG_PORT) -ErrorAction SilentlyContinue | Where-Object { $_.LocalAddress -eq '127.0.0.1' })
    if (@($owners | Where-Object { $_.OwningProcess -in @($items.ProcessId) }).Count -gt 0) { $controlled=$true }
  }
}
@{running=($items.Count -gt 0); controlled=$controlled} | ConvertTo-Json -Compress
"#;

#[cfg(windows)]
pub(super) async fn running(
    client: &LocalizedClient,
    port: Option<u16>,
) -> Result<Running, String> {
    let output = powershell(
        PROCESS_STATE,
        &[
            ("ADMINCLOUD_CLIENT_PATH", &client.executable),
            (
                "ADMINCLOUD_DEBUG_PORT",
                &port.map(|p| p.to_string()).unwrap_or_default(),
            ),
        ],
    )
    .await?;
    serde_json::from_str(output.trim()).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
pub(super) async fn running(
    _client: &LocalizedClient,
    _port: Option<u16>,
) -> Result<Running, String> {
    Ok(Running::default())
}

#[cfg(windows)]
pub(super) async fn stop(client: &LocalizedClient) -> Result<(), String> {
    powershell(r#"
$ErrorActionPreference='Stop'
$selected = $env:ADMINCLOUD_CLIENT_PATH -replace '^\\\\\?\\',''
Get-CimInstance Win32_Process -Filter "Name='Codex.exe' OR Name='ChatGPT.exe'" | Where-Object { $_.ExecutablePath -and ($_.ExecutablePath -ieq $selected) } | ForEach-Object {
  $process = Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue
  if ($process -and $process.Path -ieq $selected) { Stop-Process -InputObject $process -Force }
}
"#, &[("ADMINCLOUD_CLIENT_PATH", &client.executable)]).await?;
    for _ in 0..8 {
        if !running(client, None).await?.running {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    Err("客户端仍在退出，请稍后重试".into())
}

#[cfg(not(windows))]
pub(super) async fn stop(_client: &LocalizedClient) -> Result<(), String> {
    Err("仅支持 Windows".into())
}

#[cfg(windows)]
pub(super) async fn launch(client: &LocalizedClient, port: Option<u16>) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let path = validate_path(Path::new(&client.executable))?;
    let args = port
        .map(|port| {
            vec![
                "--lang=zh-CN".to_string(),
                "--remote-debugging-address=127.0.0.1".to_string(),
                format!("--remote-debugging-port={port}"),
            ]
        })
        .unwrap_or_default();
    let mut command = std::process::Command::new(path);
    command.args(&args).creation_flags(0x08000000);
    match command.spawn() {
        Ok(mut child) => {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if child.try_wait().map_err(|e| e.to_string())?.is_none()
                || running(client, port).await?.running
            {
                return Ok(());
            }
        }
        Err(error) if client.app_id.is_none() => return Err(error.to_string()),
        Err(_) => {}
    }
    let app_id = client
        .app_id
        .as_deref()
        .ok_or("客户端启动后立即退出，请检查安装")?;
    powershell(r#"
$ErrorActionPreference='Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
[ComImport, Guid("2e941141-7f97-4756-ba1d-9decde894a3d"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IApplicationActivationManager {
 [PreserveSig] int ActivateApplication([MarshalAs(UnmanagedType.LPWStr)] string app, [MarshalAs(UnmanagedType.LPWStr)] string args, uint options, out uint pid);
}
[ComImport, Guid("45BA127D-10A8-46EA-8AB7-56EA9078943C")] class ApplicationActivationManager {}
public static class AdminCloudActivation {
 public static void Activate(string app, string args) {
  var manager = (IApplicationActivationManager)new ApplicationActivationManager();
  uint pid; int result = manager.ActivateApplication(app, args, 0, out pid);
  Marshal.ThrowExceptionForHR(result);
 }
}
'@
[AdminCloudActivation]::Activate($env:ADMINCLOUD_APP_ID, $env:ADMINCLOUD_CLIENT_ARGS)
"#, &[("ADMINCLOUD_APP_ID", app_id), ("ADMINCLOUD_CLIENT_ARGS", &args.join(" "))]).await?;
    Ok(())
}

#[cfg(not(windows))]
pub(super) async fn launch(_client: &LocalizedClient, _port: Option<u16>) -> Result<(), String> {
    Err("仅支持 Windows".into())
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[tokio::test]
    async fn manual_path_survives_failed_automatic_discovery() {
        let dir = tempfile::tempdir().unwrap();
        let gui = dir.path().join("Codex.exe");
        std::fs::write(&gui, executable_fixture(2)).unwrap();
        let selected = super::discover_with_lookup(gui.to_str(), async {
            Err("fixture discovery timeout".into())
        })
        .await
        .expect("A valid manual path must survive failed automatic discovery")
        .unwrap();
        assert_eq!(
            selected.executable,
            super::validate_path(&gui).unwrap().to_string_lossy()
        );
        assert_eq!(selected.source, "manual");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn manual_path_keeps_available_package_activation_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let gui = dir.path().join("Codex.exe");
        std::fs::write(&gui, executable_fixture(2)).unwrap();
        let mut package = super::client(&gui, "Microsoft Store").unwrap();
        package.app_id = Some("OpenAI.Codex_family!CodexApp".into());
        package.version = Some("1.2.3".into());
        let selected = super::discover_with_lookup(gui.to_str(), async { Ok(vec![package]) })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            selected.app_id.as_deref(),
            Some("OpenAI.Codex_family!CodexApp")
        );
        assert_eq!(selected.version.as_deref(), Some("1.2.3"));
    }

    fn executable_fixture(subsystem: u16) -> Vec<u8> {
        let mut bytes = vec![0; 256];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        bytes[220..222].copy_from_slice(&subsystem.to_le_bytes());
        bytes
    }

    #[test]
    fn accepts_gui_and_rejects_cli_non_exe_and_missing_paths() {
        let dir = tempfile::tempdir().unwrap();
        let gui = dir.path().join("Codex.exe");
        std::fs::write(&gui, executable_fixture(2)).unwrap();
        assert!(super::validate_path(&gui).is_ok());
        let non_exe = dir.path().join("Codex.cmd");
        std::fs::write(&non_exe, b"MZfixture").unwrap();
        assert!(super::validate_path(&non_exe).is_err());
        let cli = dir
            .path()
            .join("node_modules/@openai/codex/vendor/codex.exe");
        std::fs::create_dir_all(cli.parent().unwrap()).unwrap();
        std::fs::write(&cli, b"MZfixture").unwrap();
        assert!(super::validate_path(&cli).is_err());
        assert!(super::validate_path(&dir.path().join("ChatGPT.exe")).is_err());
    }

    #[test]
    fn rejects_console_codex_even_outside_a_known_cli_directory() {
        let dir = tempfile::tempdir().unwrap();
        let cli = dir.path().join("codex.exe");
        std::fs::write(&cli, executable_fixture(3)).unwrap();
        assert!(
            super::validate_path(&cli).is_err(),
            "Console PE executables must not be accepted as the GUI client"
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn discovery_uses_manifest_executable_and_preserves_package_activation_id() {
        let fixture = r#"
function Get-AppxPackage { [pscustomobject]@{InstallLocation='C:\FixturePackage'; PackageFullName='OpenAI.Codex_test'; PackageFamilyName='OpenAI.Codex_family'; Version='1.2.3'} }
function Get-AppxPackageManifest { [pscustomobject]@{Package=@{Applications=@{Application=@(@{Id='CodexApp'; Executable='app\Codex.exe'})}}} }
function Get-CimInstance { [pscustomobject]@{ExecutablePath='C:\RunningFixture\ChatGPT.exe'} }
"#;
        let output = super::powershell(&format!("{fixture}\n{}", super::DISCOVERY), &[])
            .await
            .unwrap();
        let clients: Vec<super::LocalizedClient> = serde_json::from_str(output.trim()).unwrap();
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].executable, "C:\\FixturePackage\\app\\Codex.exe");
        assert_eq!(
            clients[0].app_id.as_deref(),
            Some("OpenAI.Codex_family!CodexApp")
        );
        assert_eq!(clients[1].source, "running");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn controlled_endpoint_requires_exact_process_path_flags_and_listener_owner() {
        let fixture = r#"
function Get-CimInstance { @(
  [pscustomobject]@{ExecutablePath='C:\Fixture\Codex.exe'; ProcessId=101; CommandLine='Codex.exe --remote-debugging-port=9222 --remote-debugging-address=127.0.0.1'},
  [pscustomobject]@{ExecutablePath='C:\Other\Codex.exe'; ProcessId=202; CommandLine='Codex.exe --remote-debugging-port=9222 --remote-debugging-address=127.0.0.1'}
) }
function Get-NetTCPConnection { [pscustomobject]@{LocalAddress='127.0.0.1'; OwningProcess=[int]$env:ADMINCLOUD_FIXTURE_OWNER} }
"#;
        for (owner, port, expected) in [
            ("101", "9222", true),
            ("202", "9222", false),
            ("101", "922", false),
        ] {
            let output = super::powershell(
                &format!("{fixture}\n{}", super::PROCESS_STATE),
                &[
                    ("ADMINCLOUD_CLIENT_PATH", "\\\\?\\C:\\Fixture\\Codex.exe"),
                    ("ADMINCLOUD_DEBUG_PORT", port),
                    ("ADMINCLOUD_FIXTURE_OWNER", owner),
                ],
            )
            .await
            .unwrap();
            let state: super::Running = serde_json::from_str(output.trim()).unwrap();
            assert!(state.running);
            assert_eq!(state.controlled, expected);
        }
    }
}
