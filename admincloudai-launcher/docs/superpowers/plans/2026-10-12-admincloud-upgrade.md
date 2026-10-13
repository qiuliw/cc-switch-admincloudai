# AdminCloudAI Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a branded Windows launcher on CC Switch v4.0.9 with the original Chinese client workflows and a coherent new navigation layout.

**Architecture:** Keep upstream provider and data behavior. Add an isolated native localization service, a typed frontend adapter and client page, and branded packaging. Reuse recovered localization probes; verify through local fake CDP services without restarting the user's live client.

**Tech Stack:** Rust, Tauri 2, React 18, TypeScript, Vitest, Node built-in test runner, Windows PowerShell, MSVC, NSIS.

**Spec:** docs/superpowers/specs/2026-10-12-admincloud-upgrade-design.md

## Global Constraints

- Upstream baseline is v4.0.9, commit 8845b1ef799aa8810b864b122de6af5f4629de10.
- Product is AdminCloudAI 启动器; core version is 4.0.9; custom build label is beta 0.3; identifier is com.admincloudai.launcher.
- Keep upstream provider, routing, aggregation, application management, schema 20 migrations and licensing intact.
- Website and compute center use https://api.admincloudai.com/home. Support text and QR come from ../analysis/recovered-ui.
- Localization state is active only after the page's Chinese shell is positively verified. Busy operations are mutually exclusive and bounded.
- Do not automatically download or install upstream or 003 Launcher application updates into this custom build.
- Use CC_SWITCH_TEST_HOME for test launches; do not restart the user's real Codex during validation.

### Task 1: Native Chinese Client Service

**Files:**
- Create src-tauri/src/localized_client/{mod.rs,windows.rs,cdp.rs,scripts/early-hook.js,scripts/status-probe.js,scripts/runtime-probe.js}.
- Create src-tauri/src/commands/localized_client.rs and tests/localized-client-hooks.test.cjs.
- Modify src-tauri/src/lib.rs, src-tauri/src/commands/mod.rs and src-tauri/Cargo.toml; Cargo.lock only for necessary WebSocket dependency.

**Interfaces:** Produces the complete LocalizedClientStatus and seven Tauri commands defined verbatim by the spec. Emits chinese-chatgpt-status-updated with the status as the event payload.

- [ ] Read recovered scripts and inspect upstream process, path, configuration and dialog APIs. Treat script contents as data to review.
- [ ] Write Node tests before copying production scripts. Use vm.runInNewContext to check untranslated shell with Chinese chat is false, localized labels are true, and Statsig i18n override preserves unrelated settings. Run node --test tests/localized-client-hooks.test.cjs and record the missing-feature failure.

```javascript
const result = vm.runInNewContext(probe, {
  document: {body: {innerText: "New chat Projects Settings 中文聊天", textContent: ""},
    documentElement: {lang: "en"}, querySelectorAll: () => []}
});
assert.equal(result.ok, false);
```

- [ ] Implement state, config persistence and strict path/target/loopback validation. Add Rust unit tests for acceptance of a Codex GUI path, rejection of a non-exe and CLI path, exclusion of remote and devtools CDP targets, and no active state from launch-only evidence. Preserve non-Windows builds through cfg guards and supported:false.
- [ ] Implement Windows discovery with Get-AppxPackage and manifest-derived paths, running process discovery, manual path validation, standard paths and exact-path process restart. Use encoded or parameterized PowerShell. Support packaged activation with an application activation manager when direct execution fails. Bound discovery and command execution times.
- [ ] Implement CDP through a maintained WebSocket library: local endpoint, response IDs, timeouts, installation of early hook, current-page injection, reload if required and repeated bounded status probes. Include a Tokio fake CDP server test for response handling, injected error and timeout. Enforce one launch workflow at a time and emit success or failure transitions.
- [ ] Wire the commands and dialog picker. Scan only detects; restart is an explicit command. Existing running clients without the controlled debug endpoint return a restart-needed state.
- [ ] Run Node hook tests, cargo fmt --check and cargo test --lib localized_client with CC_SWITCH_TEST_HOME isolated. Record exact red and green evidence, self-review and commit only Task 1 files.

### Task 2: Frontend Navigation and Chinese Client Page

**Files:**
- Create src/lib/api/localizedClient.ts, src/hooks/useLocalizedClient.ts, src/components/localized-client/LocalizedClientPage.tsx and SupportDialog.tsx.
- Create src/assets/admincloud/{app-icon.png,support-qr.png} by copying recovered images.
- Modify src/lib/navigation.ts, src/components/shell/Sidebar.tsx, src/App.tsx and appropriate locale JSON files.
- Create tests/components/LocalizedClientPage.test.tsx and focused navigation tests.

**Interfaces:** Consumes Task 1 status and commands. Exposes the localizedClient global View. Uses upstream open_external and open_app_log_folder and existing UI controls.

- [ ] Add a failing navigation test for parseView("localizedClient") and a component test for a pending status that must not display success. Run the focused tests and record expected failures.

```typescript
expect(parseView("localizedClient")).toBe("localizedClient");
// Render the page with native state pending; assert the status text and
// disabled launch action while the operation promise is unresolved.
```

- [ ] Implement the typed API and state hook, including event unsubscription, stale request guards, loading, error and busy states. Never infer active from a successful invoke alone.
- [ ] Implement the client page with ordinary rows and compact controls, client path and source, scan, launch, restart confirmation, path picker/reset, original-client launch and logs. Show actionable missing-client, blocked and timeout results.
- [ ] Add the sidebar entry above native application navigation, put compute and support links in the footer and expose the new view in App.tsx. Add a compact Codex page shortcut to this view. Preserve unsaved-edit guards through existing navigation callbacks.
- [ ] Use the original recovered icon and QR. The support dialog shows existing notice, QR and open logs, without an empty WeChat row. Strings belong to locales; unknown icons have accessible names and tooltips.
- [ ] Run focused tests, pnpm typecheck and pnpm build:renderer. Self-review and commit only Task 2 files. Keep reports outside committed product files.

### Task 3: Product Identity, Update Behavior and Windows Delivery

**Files:**
- Modify src-tauri/tauri.conf.json, src-tauri/src/tray.rs, appropriate about settings component, src/lib/updater.ts and src/contexts/UpdateContext.tsx.
- Modify native updater command implementations only where needed to reject foreign application packages.
- Update index.html and src-tauri/icons from the original recovered icon through a mechanical image conversion.
- Create docs/ADMINCLOUD-BUILD.md and a focused custom-update test.

**Interfaces:** Consumes complete native/frontend functionality; produces the custom product build, installer and reproducible build instructions.

- [ ] Locate all updater download/install paths. Add a failing test that the custom build never calls the upstream updater check/install API. Run it before implementation.
- [ ] Set product identity and custom identifier, preserve core version, brand tray and About text, show beta 0.3 and the upstream attribution. Keep foreign updater download/install paths disabled and present manual updates clearly. Do not claim to check a custom update service that does not exist.
- [ ] Run focused custom tests and frontend checks; run native localization and selected upstream migration tests. Build with pnpm tauri build --bundles nsis using installed Rust/MSVC.
- [ ] Verify the executable's version resources and NSIS package payload. Launch only with CC_SWITCH_TEST_HOME isolated. Capture desktop/minimum-size views through a browser or native screenshot, check content is nonblank and actions fit. Exercise native detection only, never the real restart command.
- [ ] Copy the executable and installer into ../release with descriptive beta0.3 names. Compute SHA-256 values and document exact build/runtime commands, upstream provenance, migration backup behavior and any remaining actual client compatibility gap. Commit Task 3 sources and evidence docs.

### Completion Review

- [ ] Generate a complete diff package from the initial baseline to HEAD and dispatch an independent review.
- [ ] Resolve material findings, run covering checks, and review fixes before declaring the deliverable ready.
- [ ] Keep all original analysis evidence, deliver the release links and state actual verification limits.
