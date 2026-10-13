import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createRequire } from "node:module";

// Pass the installed Playwright package directory; no dependency is added to the app.
const [exeArg, homeArg, outputArg, playwrightArg] = process.argv.slice(2);
if (!exeArg || !homeArg || !outputArg || !playwrightArg) {
  throw new Error("Usage: node scripts/admincloud-runtime-smoke.mjs EXE TEST_HOME OUTPUT PLAYWRIGHT_PACKAGE");
}
const require = createRequire(import.meta.url);
const { chromium } = require(resolve(playwrightArg));
const testHome = resolve(homeArg);
const output = resolve(outputArg);
await mkdir(testHome, { recursive: true });
await mkdir(output, { recursive: true });
const listener = createServer();
await new Promise(resolve => listener.listen(0, "127.0.0.1", resolve));
const port = listener.address().port;
await new Promise(resolve => listener.close(resolve));
const child = spawn(resolve(exeArg), [], {
  env: { ...process.env, CC_SWITCH_TEST_HOME: testHome, WEBVIEW2_USER_DATA_FOLDER: resolve(testHome, "webview2"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1` },
  windowsHide: true,
  stdio: "ignore",
});
let browser;
const evidence = { pid: child.pid, nativeWebView: true, frames: [], guards: {}, scan: null };
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
try {
  const deadline = Date.now() + 60000;
  while (true) {
    if (child.exitCode !== null) throw new Error(`Test launcher exited: ${child.exitCode}`);
    try {
      const result = await fetch(`http://127.0.0.1:${port}/json/version`);
      if (result.ok) break;
    } catch {}
    if (Date.now() > deadline) throw new Error("Test WebView CDP endpoint did not become ready");
    await delay(500);
  }
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const context = browser.contexts()[0];
  let page = context.pages()[0];
  if (!page) page = await context.waitForEvent("page", { timeout: 15000 });
  await page.waitForFunction(() => document.body.innerText.includes("AdminCloudAI"), undefined, { timeout: 30000 });
  await delay(1500);
  const welcome = page.getByRole("dialog").getByRole("button", { name: /我知道了|知道了|Got it/ });
  if (await welcome.count()) {
    evidence.welcome = await page.getByRole("dialog").innerText();
    if (!evidence.welcome.includes("AdminCloudAI") || !evidence.welcome.includes("CC Switch v4.0.9")) {
      throw new Error("Packaged welcome content is stale or missing product attribution");
    }
    await welcome.last().click();
  }
  await page.getByRole("button", { name: "启动中文 ChatGPT", exact: true }).waitFor();
  for (const command of ["install_update_and_restart", "check_app_update_available", "plugin:updater|check"]) {
    evidence.guards[command] = await page.evaluate(async command => {
      try { await window.__TAURI_INTERNALS__.invoke(command); return { rejected: false }; }
      catch (error) { return { rejected: true, error: String(error) }; }
    }, command);
    if (!evidence.guards[command].rejected) throw new Error(`Application updater escaped its guard: ${command}`);
  }
  await page.waitForFunction(() => [...document.querySelectorAll("button")].some(
    button => button.textContent.trim() === "重新扫描" && !button.disabled
  ), undefined, { timeout: 45000 });
  evidence.scan = await page.evaluate(async () => {
    const status = await window.__TAURI_INTERNALS__.invoke("scan_chinese_chatgpt");
    return { state: status.state, active: status.active, supported: status.supported,
      reason: status.reason, client: status.client ? { name: status.client.name, source: status.client.source, version: status.client.version } : null };
  });
  if (evidence.scan.reason === "operation-in-progress") throw new Error("Native scan did not reach a stable result");
  for (const [width, height] of [[1000, 650], [900, 600]]) {
    await page.setViewportSize({ width, height });
    await delay(500);
    const frame = await page.evaluate(() => {
      const actions = [...document.querySelectorAll("button")].filter(el =>
        ["启动中文 ChatGPT", "重启中文 ChatGPT"].includes(el.textContent.trim())
      ).map(el => { const b = el.getBoundingClientRect(); return { text: el.textContent.trim(), x: b.x, y: b.y, width: b.width, height: b.height }; });
      return { width: innerWidth, height: innerHeight, textLength: document.body.innerText.length,
        overflow: document.documentElement.scrollWidth > innerWidth, actions };
    });
    if (frame.textLength < 100 || frame.overflow || frame.actions.length !== 2 ||
      frame.actions.some(b => b.x < 0 || b.y < 0 || b.x + b.width > width || b.y + b.height > height)) {
      throw new Error(`Invalid native frame: ${JSON.stringify(frame)}`);
    }
    // Hide only the installed executable path in shareable evidence.
    const privatePath = page.locator("dl > div").filter({ has: page.locator("dt", { hasText: "客户端路径" }) }).locator("dd");
    await page.screenshot({ path: resolve(output, `native-${width}x${height}.png`), scale: "css", maskColor: "#e5e7eb", mask: await privatePath.count() ? [privatePath] : [] });
    evidence.frames.push(frame);
  }
  const restart = page.getByRole("button", { name: "重启中文 ChatGPT", exact: true });
  if (await restart.isEnabled()) {
    await restart.click();
    const dialog = page.getByRole("dialog");
    await dialog.waitFor();
    await dialog.getByRole("button", { name: /取消/ }).click();
    evidence.restartConfirmationCancelled = true;
  }
  await writeFile(resolve(output, "native-smoke.json"), JSON.stringify(evidence, null, 2));
  console.log(JSON.stringify(evidence, null, 2));
} finally {
  if (browser) await browser.close().catch(() => {});
  if (child.exitCode === null) {
    child.kill();
    await Promise.race([new Promise(resolve => child.once("exit", resolve)), delay(5000)]);
  }
}
