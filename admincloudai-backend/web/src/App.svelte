<script lang="ts">
  // AdminCloudAI 官网单页：最新版本下载 + 在线客服二维码 + 管理端（/admin）。
  // 极简实现：原生 fetch + 手写样式，无 UI 框架与路由库。
  import Admin from "./Admin.svelte";

  // 路径路由（无路由库）：popstate 时同步，首次渲染由 $state 初始化完成
  function currentRoute(): string {
    return location.pathname.replace(/\/+$/, "") || "/";
  }
  let route = $state(currentRoute());
  window.addEventListener("popstate", () => {
    route = currentRoute();
  });

  let latest: { version: string; notes?: string; url?: string } | null = $state(null);
  let qrFailed = $state(false);

  async function loadLatest() {
    try {
      // 用升级 feed 查最新版本（current 传 0.0.0 必触发 200）
      const res = await fetch("/api/update/windows-x86_64/0.0.0");
      if (!res.ok) return;
      const data = await res.json();
      latest = {
        version: data.version,
        notes: data.notes,
        url: data.platforms?.["windows-x86_64"]?.url,
      };
    } catch {
      /* 后端不可用时静默降级 */
    }
  }
  loadLatest();
</script>

{#if route === "/admin"}
  <Admin />
{:else}
  <header>
    <span class="brand">AdminCloudAI</span>
    <span class="sub">启动器 · 升级服务</span>
  </header>

  <main>
    <section class="card">
      <h2>最新版本</h2>
      {#if latest}
        <p class="version">{latest.version}</p>
        {#if latest.notes}<p class="notes">{latest.notes}</p>{/if}
        {#if latest.url}
          <a class="btn primary" href={latest.url}>下载 Windows 安装包</a>
        {:else}
          <p class="muted">暂无下载地址</p>
        {/if}
      {:else}
        <p class="muted">尚未发布版本</p>
      {/if}
    </section>

    <section class="card">
      <h2>客服支持</h2>
      {#if qrFailed}
        <p class="muted">二维码暂不可用，请稍后再试。</p>
      {:else}
        <img
          class="qr"
          src="/api/qr"
          alt="客服二维码"
          onerror={() => (qrFailed = true)}
        />
      {/if}
    </section>
  </main>

  <footer>
    <span>AdminCloudAI · <a href="/api/qr" target="_blank">联系支持</a></span>
  </footer>
{/if}
