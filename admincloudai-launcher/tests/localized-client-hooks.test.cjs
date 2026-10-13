const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const {JSDOM} = require('jsdom');
const scripts = path.join(__dirname, '../src-tauri/src/localized_client/scripts');
function script(name) {
  const file = path.join(scripts, `${name}.js`);
  assert.ok(fs.existsSync(file), `${name} production script must exist`);
  return fs.readFileSync(file, 'utf8');
}
function probe(markup, lang = 'en') {
  const dom = new JSDOM(`<!doctype html><html lang="${lang}"><body>${markup}</body></html>`);
  // jsdom has no layout-backed innerText; keep its actual elements and selectors.
  Object.defineProperty(dom.window.HTMLElement.prototype, 'innerText', {get() {return this.textContent;}});
  try {
    return vm.runInNewContext(script('status-probe'), {document: dom.window.document});
  } finally {
    dom.window.close();
  }
}
test('English shell with Chinese chat is not localized', () => {
  assert.equal(probe('<nav><a href="/new">New chat</a><a href="/settings">Settings</a></nav><main>中文聊天</main>').ok, false);
});
test('Chinese conversation containing shell words is not shell proof', () => {
  assert.equal(probe('<main>新对话 项目 设置 中文聊天</main>', 'zh-CN').ok, false);
});
test('Two distinct fixed shell actions with corresponding translations prove Chinese shell', () => {
  assert.equal(probe('<nav><a href="/new">新对话</a><a href="/settings">设置</a></nav>').ok, true);
});
test('Chinese history names in an English navigation shell are not shell proof', () => {
  assert.equal(probe('<nav><button>New chat</button><button>Settings</button><section aria-label="History"><a href="/c/a">项目</a><a href="/c/b">设置</a></section></nav>').ok, false);
});
test('Chinese title and aria-label on history, project and profile entries are not shell proof', () => {
  assert.equal(probe('<nav><a href="/new">New chat</a><a href="/settings">Settings</a><section aria-label="History"><a href="/c/a" title="项目" aria-label="项目">项目</a><a href="/c/b" title="设置" aria-label="设置">设置</a></section><a href="/projects/user-project" title="定时任务" aria-label="定时任务">定时任务</a><button title="新聊天" aria-label="新聊天">新聊天</button></nav>', 'zh-CN').ok, false);
});
test('Unknown navigation markup remains unverified even with translated labels', () => {
  assert.equal(probe('<aside><button title="新对话" aria-label="新对话">新对话</button><button title="设置" aria-label="设置">设置</button></aside>').ok, false);
});
test('Translations must correspond to their fixed action identity', () => {
  assert.equal(probe('<nav><a href="/new">设置</a><a href="/settings">项目</a></nav>').ok, false);
});
test('Two translated synonyms for one fixed action do not establish two actions', () => {
  assert.equal(probe('<nav><a href="/new">新对话</a><a href="/new">新聊天</a></nav>').ok, false);
});
test('External URLs and user-specific route suffixes cannot claim fixed action identity', () => {
  assert.equal(probe('<nav><a href="https://example.com/settings" aria-label="设置">设置</a><a href="/projects/mine" aria-label="项目">项目</a><a href="/tasks/user-task" aria-label="定时任务">定时任务</a></nav>').ok, false);
});
test('Fixed action links in conversation content are not shell controls', () => {
  assert.equal(probe('<main><a href="/new" aria-label="新对话">新对话</a><a href="/settings" aria-label="设置">设置</a></main>').ok, false);
});
test('Translated metadata does not override visible English shell action text', () => {
  assert.equal(probe('<nav><a href="/new" aria-label="新对话" title="新对话">New chat</a><a href="/settings" aria-label="设置" title="设置">Settings</a></nav>').ok, false);
});
test('Icon-only fixed shell actions can use their corresponding accessible translations', () => {
  assert.equal(probe('<nav><a href="/new" aria-label="新对话"></a><a href="/settings" aria-label="设置"></a></nav>').ok, true);
});
test('Statsig language override preserves unrelated layers and adapter settings', () => {
  const layers = {i18n: {name: 'i18n', value: {enable_i18n: false, locale_source: 'USER', other: 42}, get(k, fallback) {return this.value[k] ?? fallback;}},
    billing: {name: 'billing', value: {discount: 7}, get(k, fallback) {return this.value[k] ?? fallback;}}};
  const client = {getLayer(name) {return layers[name];}, overrideAdapter: {getGateOverride: () => 'preserved'}};
  const context = {__STATSIG__: {firstInstance: client}, setInterval: () => 1, clearInterval() {}};
  context.window = context;
  vm.runInNewContext(script('early-hook'), context);
  assert.equal(client.getLayer('i18n').get('enable_i18n', false), true);
  assert.equal(client.getLayer('i18n').get('locale_source'), 'SYSTEM');
  assert.equal(client.getLayer('i18n').get('other'), 42);
  assert.equal(client.getLayer('billing'), layers.billing);
  assert.equal(client.overrideAdapter.getGateOverride(), 'preserved');
  assert.equal(layers.i18n.value.enable_i18n, false);
});
test('Runtime hook installation alone is not localization evidence', () => {
  const context = {__STATSIG__: {firstInstance: {getLayer() {}}}, __sz003InstallI18nOverride() {}, __sz003I18nHookReady: true};
  assert.equal(vm.runInNewContext(script('runtime-probe'), context).ok, false);
});
test('Late Statsig namespace registration patches instances once', () => {
  const context = {setInterval: () => 1, clearInterval() {}};
  context.window = context;
  vm.runInNewContext(script('early-hook'), context);
  let events = 0;
  const client = {getLayer: () => ({get: (key, fallback) => key === 'enable_i18n' ? false : fallback}), $emt() {events += 1;}};
  context.__STATSIG__ = {firstInstance: client};
  vm.runInNewContext(script('early-hook'), context);
  assert.equal(client.getLayer('language').get('enable_i18n', false), true);
  assert.equal(events, 1);
});

test('Known Codex language layer is enabled even when its remote value is empty', () => {
  const language = {name: '72216192', value: {}, get(key, fallback) {return this.value[key] ?? fallback;}};
  const unrelated = {name: 'unrelated', value: {}, get(key, fallback) {return this.value[key] ?? fallback;}};
  const client = {getLayer(name) {return name === '72216192' ? language : unrelated;}};
  const context = {__STATSIG__: {firstInstance: client}, setInterval: () => 1, clearInterval() {}};
  context.window = context;
  vm.runInNewContext(script('early-hook'), context);
  assert.equal(client.getLayer('72216192').get('enable_i18n', false), true);
  assert.equal(client.getLayer('72216192').get('locale_source', 'IDE'), 'SYSTEM');
  assert.equal(client.getLayer('unrelated'), unrelated);
  assert.deepEqual(language.value, {});
});

test('Existing older hook can be upgraded without restarting an active client', () => {
  const language = {name: '72216192', value: {}, get(key, fallback) {return this.value[key] ?? fallback;}};
  let updates = 0;
  const client = {getLayer() {return language;}, $emt() {updates++;}};
  Object.defineProperty(client, '__sz003I18nClientPatched', {value:true});
  const context = {__STATSIG__: {firstInstance:client}, __sz003I18nGlobalArmed:true};
  context.window = context;
  vm.runInNewContext(script('early-hook'), context);
  vm.runInNewContext(script('runtime-probe'), context);
  assert.equal(client.getLayer('72216192').get('enable_i18n', false), true);
  assert.equal(updates, 1);
});

test('Codex built-in sidebar destination buttons establish translated shell identity', () => {
  assert.equal(probe('<div><button class="sidebar-item" data-sidebar-destination="builtin:automations">定时任务</button><button class="sidebar-item" data-sidebar-destination="builtin:skills">插件</button></div>').ok, true);
});

test('English built-in sidebar buttons with Chinese project names remain unverified', () => {
  assert.equal(probe('<div><button class="sidebar-item" data-sidebar-destination="builtin:automations">Scheduled</button><button class="sidebar-item" data-sidebar-destination="builtin:skills">Plugins</button><button class="sidebar-item" data-sidebar-destination="project:automations">定时任务</button><button class="sidebar-item" data-sidebar-destination="project:skills">插件</button></div>').ok, false);
});

test('Built-in destination markers in conversation content do not establish shell identity', () => {
  assert.equal(probe('<main><button class="sidebar-item" data-sidebar-destination="builtin:automations">定时任务</button><button class="sidebar-item" data-sidebar-destination="builtin:skills">插件</button></main>').ok, false);
});
