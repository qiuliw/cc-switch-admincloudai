(() => {
  const client = globalThis.__STATSIG__?.firstInstance || globalThis.__STATSIG__?.instance?.();
  if (client && typeof globalThis.__sz003InstallI18nOverride === 'function') {
    globalThis.__sz003InstallI18nOverride(client);
  }
  // This probe reports hook readiness only; the separate shell probe owns active state.
  return {ok: false, reason: client && globalThis.__sz003I18nHookReady ? 'locale-pending' : 'statsig-client-not-ready'};
})()
