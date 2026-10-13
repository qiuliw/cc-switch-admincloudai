(() => {
  // Exact action routes establish identity; translated text alone never does.
  // Unknown client markup deliberately remains unverified.
  const actions = new Map([
    ['/new', ['新对话', '新聊天', '新建对话']],
    ['/tasks', ['定时任务']],
    ['/plugins', ['插件']],
    ['/projects', ['项目']],
    ['/settings', ['设置']],
  ]);
  const matched = new Set();
  for (const node of document.querySelectorAll('nav a[href], aside a[href], [role="navigation"] a[href]')) {
    if (node.closest('main, article, [role="main"], [role="article"]')) continue;
    const action = node.getAttribute('href');
    const labels = actions.get(action);
    if (!labels) continue;
    const visible = (node.innerText || '').trim();
    const label = visible || (node.getAttribute('aria-label') || '').trim();
    if (labels.includes(label)) matched.add(action);
  }
  // Current Codex renders fixed destinations as buttons instead of route links.
  const destinations = new Map([
    ['builtin:automations', ['定时任务', '计划任务', '自动化']],
    ['builtin:skills', ['插件']],
  ]);
  for (const node of document.querySelectorAll('button.sidebar-item[data-sidebar-destination]')) {
    if (node.closest('main, article, [role="main"], [role="article"]')) continue;
    const destination = node.getAttribute('data-sidebar-destination');
    const labels = destinations.get(destination);
    if (!labels) continue;
    const visible = (node.innerText || '').trim();
    const label = visible || (node.getAttribute('aria-label') || '').trim();
    if (labels.includes(label)) matched.add(destination === 'builtin:automations' ? '/tasks' : '/plugins');
  }
  const ok = matched.size >= 2;
  return {ok, reason: ok ? 'locale-active' : 'localized-shell-not-verified'};
})()
