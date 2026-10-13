(() => {
  // The supported Codex build requests this layer even when its remote value is empty.
  // Restrict that fallback to the confirmed language layer; unrelated layers stay intact.
  const knownLanguageLayers = new Set(['72216192']);
  const patchVersion = 2;
  const sentinel = Symbol.for('sz003.i18n.sentinel');
  const isLanguageLayer = (layer, name) => {
    try {
      return !!layer && typeof layer.get === 'function' &&
        (knownLanguageLayers.has(name || layer.name) ||
          layer.get('enable_i18n', sentinel) !== sentinel || layer.get('locale_source', sentinel) !== sentinel);
    } catch { return false; }
  };
  const wrap = (layer, name) => {
    if (!isLanguageLayer(layer, name) || layer.__sz003I18nLayerOverride) return layer;
    globalThis.__sz003I18nLayerId = name || layer.name;
    const value = {...(layer.__value || layer.value || {}), enable_i18n: true, locale_source: 'SYSTEM'};
    return {...layer, value, __value: value, __sz003I18nLayerOverride: true,
      get(key, fallback) {
        if (key === 'enable_i18n') return true;
        if (key === 'locale_source') return 'SYSTEM';
        return layer.get(key, fallback);
      }};
  };
  const install = client => {
    if (!client || typeof client.getLayer !== 'function' || client.__sz003I18nPatchVersion === patchVersion) return false;
    const original = client.getLayer;
    client.getLayer = function(name, options) { return wrap(original.call(this, name, options), name); };
    const previous = client.overrideAdapter || {};
    client.overrideAdapter = {...previous,
      getGateOverride: (...args) => previous.getGateOverride?.(...args),
      getDynamicConfigOverride: (...args) => previous.getDynamicConfigOverride?.(...args),
      getExperimentOverride: (...args) => previous.getExperimentOverride?.(...args),
      getParamStoreOverride: (...args) => previous.getParamStoreOverride?.(...args),
      getLayerOverride(layer, user, options) {
        return wrap(previous.getLayerOverride?.(layer, user, options) || layer, layer?.name);
      }};
    Object.defineProperty(client, '__sz003I18nPatchVersion', {value: patchVersion, configurable: true});
    client._memoCache = {};
    client.$emt?.({name: 'values_updated', status: 'Ready', values: client.getContext?.().values});
    globalThis.__sz003I18nHookReady = true;
    return true;
  };
  globalThis.__sz003InstallI18nOverride = install;
  globalThis.__sz003IsI18nLayer = isLanguageLayer;
  const arm = namespace => {
    if (!namespace) return;
    const prototype = namespace.StatsigClient?.prototype;
    if (prototype?.getLayer && prototype.__sz003I18nPrototypeVersion !== patchVersion) {
      const original = prototype.getLayer;
      prototype.getLayer = function(name, options) { return wrap(original.call(this, name, options), name); };
      Object.defineProperty(prototype, '__sz003I18nPrototypeVersion', {value: patchVersion, configurable: true});
    }
    const existing = namespace.firstInstance || namespace.instance?.();
    if (existing) { install(existing); return; }
    let attempts = 0;
    const timer = setInterval(() => {
      const client = namespace.firstInstance || namespace.instance?.();
      if (client || ++attempts >= 200) { install(client); clearInterval(timer); }
    }, 50);
  };
  if (!globalThis.__sz003I18nGlobalArmed) {
    let namespace = globalThis.__STATSIG__;
    try {
      Object.defineProperty(window, '__STATSIG__', {configurable: true, enumerable: true,
        get: () => namespace, set(value) { namespace = value; arm(value); }});
      globalThis.__sz003I18nGlobalArmed = true;
    } catch { /* A non-configurable namespace can still expose an existing client. */ }
    arm(namespace);
  }
  return {ok: true, reason: 'early-hook-installed'};
})()
