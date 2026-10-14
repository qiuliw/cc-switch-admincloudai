<script lang="ts">
  // 管理端视图：Basic 鉴权（ADMIN_PASSWORD），上传新安装包 / 更新客服二维码。
  const savedPassword = sessionStorage.getItem("admin-pass") ?? "";
  let password = $state(savedPassword);
  let loggedIn = $state(false);
  let message = $state("");
  let busy = $state(false);

  // 上传新包表单
  let packageFile = $state<File | null>(null);
  let sigFile = $state<File | null>(null);
  let version = $state("");
  let notes = $state("");
  // 二维码表单
  let qrFile = $state<File | null>(null);

  // 挂载时若有保存的密码则静默登录一次（不是响应式，避免击键触发请求）
  if (savedPassword) {
    void login();
  }

  function authHeader(): string {
    return "Basic " + btoa("admin:" + password);
  }

  async function login() {
    message = "";
    try {
      const res = await fetch("/api/admin/ping", {
        headers: { Authorization: authHeader() },
      });
      loggedIn = res.ok;
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        message = data.error ?? "密码错误";
        sessionStorage.removeItem("admin-pass");
      } else {
        sessionStorage.setItem("admin-pass", password);
      }
    } catch {
      message = "服务不可达";
    }
  }

  function pickFile(e: Event, set: (f: File | null) => void) {
    set((e.target as HTMLInputElement).files?.[0] ?? null);
  }

  async function uploadPackage() {
    if (!packageFile || !sigFile || !version.trim()) {
      message = "请填写版本号并选择 .exe 与 .sig 文件";
      return;
    }
    busy = true;
    message = "上传中…";
    try {
      const form = new FormData();
      form.append("file", packageFile);
      form.append("sig", sigFile);
      form.append("version", version);
      form.append("notes", notes);
      const res = await fetch("/api/admin/package", {
        method: "POST",
        headers: { Authorization: authHeader() },
        body: form,
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data.error ?? `失败 (${res.status})`);
      message = `已发布 v${data.version}`;
      notes = "";
    } catch (e) {
      message = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function uploadQr() {
    if (!qrFile) {
      message = "请选择 PNG 图片";
      return;
    }
    busy = true;
    message = "上传中…";
    try {
      const form = new FormData();
      form.append("file", qrFile);
      const res = await fetch("/api/admin/qr", {
        method: "POST",
        headers: { Authorization: authHeader() },
        body: form,
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data.error ?? `失败 (${res.status})`);
      message = "二维码已更新";
      qrFile = null;
    } catch (e) {
      message = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<header>
  <span class="brand">AdminCloudAI</span>
  <span class="sub">管理端</span>
</header>

<main>
  {#if !loggedIn}
    <section class="card narrow">
      <h2>管理员登录</h2>
      <div class="row">
        <input
          type="password"
          placeholder="ADMIN_PASSWORD"
          bind:value={password}
          onkeydown={(e) => e.key === "Enter" && login()}
        />
        <button class="btn primary" onclick={login}>登录</button>
      </div>
    </section>
  {:else}
    <section class="card">
      <h2>发布新版本</h2>
      <div class="col">
        <label>版本号 <input placeholder="4.0.11" bind:value={version} /></label>
        <label>安装包（.exe）<input type="file" accept=".exe" onchange={(e) => pickFile(e, (f) => (packageFile = f))} /></label>
        <label>签名（.sig）<input type="file" accept=".sig" onchange={(e) => pickFile(e, (f) => (sigFile = f))} /></label>
        <label>更新说明（可选）<textarea rows="3" bind:value={notes}></textarea></label>
        <button class="btn primary" disabled={busy} onclick={uploadPackage}>发布</button>
      </div>
    </section>

    <section class="card">
      <h2>更新客服二维码</h2>
      <div class="col">
        <label>二维码图片（PNG）<input type="file" accept=".png" onchange={(e) => pickFile(e, (f) => (qrFile = f))} /></label>
        <button class="btn primary" disabled={busy} onclick={uploadQr}>更新二维码</button>
      </div>
    </section>
  {/if}

  {#if message}
    <p class="notice">{message}</p>
  {/if}
</main>

<footer>
  <span><a href="/">← 返回首页</a></span>
</footer>
