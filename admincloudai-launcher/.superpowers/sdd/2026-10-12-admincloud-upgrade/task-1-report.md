# Task 1: Native Chinese Client Service

Status: implemented and verified; no real-client compatibility test performed.

## Scope

Implemented the seven specified Tauri commands and the complete camelCase status payload, emitted on `chinese-chatgpt-status-updated`. The service persists only the selected executable and controlled debug endpoint metadata in the upstream application config directory. Windows discovery uses package manifest executables, running process paths and standard locations. Manual paths are validated before persistence. Restart is explicit and restricts process termination to the selected executable's exact path. Store activation fallback uses IApplicationActivationManager with arguments passed as environment data to encoded PowerShell.

CDP uses tokio-tungstenite, an exact 127.0.0.1 endpoint/port check, official `app://-/` or `app://-/index.html` page targets, correlation IDs, per-request timeouts, a bounded workflow, early-script registration, current-page injection, reload and repeated shell probes. Scan does not inject, launch or terminate. Existing clients without the controlled endpoint return restart-required. Only positively verified translated navigation controls can establish active state.

The recovered scripts were read and reviewed before reuse. Their Statsig technique is retained, while unrelated layers are left intact and page-body Chinese text is excluded from shell evidence.

## RED evidence

Command (repository root):

```powershell
& 'C:/Program Files/nodejs/node.exe' --test tests/localized-client-hooks.test.cjs
```

Initial result: exit 1, 5 tests, 0 pass, 5 fail. Assertions reported `status-probe production script must exist`, `early-hook production script must exist`, and `runtime-probe production script must exist`. Production scripts did not yet exist.

Native tests were written before native implementation. A pre-implementation native RED run was unavailable: `rustup toolchain list` reported `no installed toolchains` while the parent installed the compiler. This is a documented TDD limitation, not a claimed passing or failing native run. Additional native regression RED/GREEN evidence is recorded below when available.

## Environment and commands

Rust 1.95 and Visual Studio 2022 BuildTools were installed by the parent. No environment reinstallation or toolchain-file change was performed. The original crates.io fetch timed out repeatedly. This command uses a temporary, command-scoped sparse mirror without changing repository/global Cargo configuration:

```powershell
. 'C:/admincloud启动器/tools/Enter-BuildEnvironment.ps1'
$env:CC_SWITCH_TEST_HOME = 'C:/admincloud启动器/app/.test-home/native-client'
& 'C:/Users/jsqjsxxcc/.cargo/bin/cargo.exe' +1.95 --config 'source.crates-io.replace-with="rsproxy-sparse"' --config 'source.rsproxy-sparse.registry="sparse+https://rsproxy.cn/index/"' test --lib localized_client
```

## Verification so far

- Node hooks: 6 tests passed, 0 failed, exit 0. Covers English shell plus Chinese chat, Chinese chat containing shell words, localized navigation controls, preservation of unrelated Statsig settings/layers, runtime-hook-only evidence, late Statsig registration and idempotence.
- `cargo +1.95 fmt --check` initially reported formatting differences in newly written Rust files. `cargo +1.95 fmt` applied standard formatting.
- Native test GREEN: 11 passed, 0 failed, 3517 filtered out, exit 0. Final rerun finished in 4.33 seconds after 1m25s compile.

## Native regression RED and GREEN

The full native command above initially compiled successfully in 6m51s and produced 9 pass / 1 fail. The intended new regression failed with:

```text
localized_client::windows::tests::rejects_console_codex_even_outside_a_known_cli_directory ... FAILED
Console PE executables must not be accepted as the GUI client
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 3517 filtered out
```

Cause: initial executable validation read only the MZ signature, so a renamed or relocated console Codex binary passed. Fix: read PE signature, optional-header magic and Windows GUI subsystem with bounded reads. GUI PE fixture passes, console fixture fails. Added process/port ownership fixtures as well. The same full command then passed all 11 tests. A final simplification removed a redundant status-lock precheck and the entire suite passed again.

Final native test coverage includes path/PE validation; CLI and missing paths; exact official target URL/loopback/port validation; fake HTTP discovery and WebSocket injection order; response-ID correlation; injected CDP errors and timeout; JavaScript exceptions; launch-only evidence; mutex exclusion/release; complete emitted error status; manifest-derived executable/AppUserModelID; and exact-path flags plus TCP listener ownership.

Final checks: Node 6/6; cargo fmt --check exit 0; git diff --check exit 0. Compile/test runs emitted no Rust warnings or compiler errors. Cargo lock changes add tokio-tungstenite/tungstenite/data-encoding and Tokio process support; Cargo normalized an existing rustix Windows dependency edge while resolving features. No broad package-version update was performed.

## Files and self-review

Created `src-tauri/src/localized_client/mod.rs`, `windows.rs`, `cdp.rs`, `scripts/early-hook.js`, `scripts/status-probe.js`, `scripts/runtime-probe.js`; created `src-tauri/src/commands/localized_client.rs` and `tests/localized-client-hooks.test.cjs`. Modified only task-scoped `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, plus this report.

Self-review confirmed: explicit restart only; no name-wide process kill; fixture-only Windows process tests; exact selected-path and owned loopback port proof before CDP; no remote target or redirect; no active state from process, open port or successful injection; bounded request/discovery/workflow and lock release; config uses existing atomic writer and upstream directory; non-Windows paths are cfg-guarded and report unsupported. The `configured` field means a manual path is saved, independent of automatic discovery. Other workspace edits belong to the parent/other task and are excluded from this commit.

## Limits

No real Codex/ChatGPT instance was launched, closed, restarted or injected during this task. No credential files were read. Windows discovery tests substitute package and process fixtures; CDP tests use local fake servers. Compatibility with any particular installed official client build remains untested and must be disclosed in delivery.

No subagents or reviewers were spawned. Commit information is supplied in the task completion message.

## Review Fix Round 1

Base: `f1deb20`. Both reported issues were reproduced and fixed. `run` now returns `blocked`, `active: false`, and `reason: operation-in-progress` on contention, with a clear retry message. This rejected request does not overwrite or emit a replacement for the active workflow's shared state. The service-level regression calls `run` for all three explicit actions while an operation holds the guard, rather than only testing the Tokio mutex.

Manual executable validation now occurs before polling automatic discovery. Discovery errors/timeouts become optional metadata failures for a valid manual selection. Successful metadata lookup still attaches the matched package AppUserModelID and version for Store activation. A small async lookup boundary permits a deterministic failing fixture without real package/process access.

RED command (same environment and command-scoped mirror as above):

```powershell
. 'C:/admincloud启动器/tools/Enter-BuildEnvironment.ps1'
$env:CC_SWITCH_TEST_HOME = 'C:/admincloud启动器/app/.test-home/native-client'
& 'C:/Users/jsqjsxxcc/.cargo/bin/cargo.exe' +1.95 --config 'source.crates-io.replace-with="rsproxy-sparse"' --config 'source.rsproxy-sparse.registry="sparse+https://rsproxy.cn/index/"' test --lib localized_client
```

RED output: exit 1, `12 passed; 2 failed; 3517 filtered out; finished in 4.16s`. `explicit_actions_report_busy_instead_of_cached_success` failed with `left: Active, right: Blocked`. `manual_path_survives_failed_automatic_discovery` failed with `A valid manual path must survive failed automatic discovery: "fixture discovery timeout"`. The package activation metadata test passed.

GREEN: identical native command, exit 0 after 1m16s compile; `14 passed; 0 failed; 0 ignored; 0 measured; 3517 filtered out; finished in 4.04s`. Node hook suite: `6 passed; 0 failed`, exit 0. `cargo +1.95 fmt --manifest-path src-tauri/Cargo.toml --check` and `git diff --check` passed, exit 0. No compiler warnings. Files in this fix: `localized_client/mod.rs`, `localized_client/windows.rs`, and this report. No real clients were launched, stopped or inspected during these tests. Existing compatibility limitations still apply. Fix commit SHA is returned with completion.
