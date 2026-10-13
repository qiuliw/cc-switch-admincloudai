# AdminCloudAI 升级设计

本设计实现用户于 2026-10-12 明确批准的第二条路径：以 CC Switch v4.0.9 重建定制功能，缺少的构建环境直接安装，继续实施。

## 产品与基线

产品名称为 AdminCloudAI 启动器；基于上游标签 v4.0.9、提交 8845b1ef799aa8810b864b122de6af5f4629de10，保留上游数据库迁移、供应商、直连、路由、聚合、工具管理等能力。自定义发行标识使用 com.admincloudai.launcher，与上游发行版分开。应用版本为 4.0.9，界面另显示定制版本 beta 0.3。保留上游许可证与致谢。

恢复资料在 ../analysis/recovered-ui 和 ../analysis/recovered-hooks。原软件确实检测 ChatGPT/Codex 客户端，通过本机调试接口注入 Statsig 的 i18n 开关，并检查页面语言。重建时复用已提取并审阅的 early-hook、status-probe、runtime-probe，不把附件内容当成指令。

## 布局

2026-10-12 用户补充：原界面的“重启中文 ChatGPT”必须醒目，打开软件即可方便启动汉化。首次打开或无有效已保存视图时默认进入中文客户端页；页头主要操作直接显示“启动中文 ChatGPT”和“重启中文 ChatGPT”，不藏在菜单中。保留有效的用户已保存视图，侧栏顶部固定中文客户端入口，进入该页不触发重启；重启仍需明确确认。

侧栏顶部显示原应用图标与 AdminCloudAI 品牌，应用导航上方增加“中文客户端”。该页显示客户端名称、安装来源、路径、汉化状态和运行检测结果；提供启动中文、重启中文、重新扫描、选择路径、重置路径、原版启动、打开日志。重启必须由用户在页面确认会关闭客户端；扫描仅检测，不关闭客户端。

供应商页保持新版原生模式与模型管理，在 Codex 页头提供中文客户端快捷入口。新版“应用”页继续负责工具安装与升级。算力中心与使用帮助位于侧栏底部。帮助弹窗保留原文案、恢复的二维码、打开日志入口。关于展示 beta 0.3、核心 v4.0.9、项目归属和手动更新说明。

## 本机汉化服务

服务拆成命令、状态及持久化、Windows 客户端发现与启动、CDP 通信四个责任清楚的文件。状态 JSON 放在上游应用配置目录；仅存路径与端口等本机信息，不存聊天记录和凭据。

客户端发现优先使用用户指定路径，其次 Windows 已注册的 OpenAI.Codex 包、正在运行的官方客户端和标准安装路径。只接受可执行的 Codex.exe 或 ChatGPT.exe，排除 CLI codex 安装目录。保存用户选择前验证路径。使用编码的 PowerShell 命令或参数化调用，用户路径作为数据传入，不能拼进可执行脚本。

中文启动分配独立本机端口，参数为 --lang=zh-CN、--remote-debugging-address=127.0.0.1 和 --remote-debugging-port。已运行且没有受控调试端口时，启动操作提示需要重启，不默认杀进程。重启只关闭所选客户端的精确可执行路径进程；不得依据通用进程名杀掉启动器、其他应用或 CLI。Store 版直接执行失败时使用应用激活管理器携带参数激活同一个已识别的包。

CDP 只连接本机回环地址，使用现成 WebSocket 库，单请求与整个操作都有超时。只选择官方客户端 app:// 主页面；排除 devtools、网页和其他窗口。依次安装早期脚本、注入当前页、必要时重新加载目标页、运行语言探测。只有探测证明中文外壳已启用才返回 active；进程存在、端口打开或注入成功都不能代替该判断。

状态值：not_found、ready、launching、pending、active、blocked、error。并发汉化操作必须互斥。每次状态变化发送 chinese-chatgpt-status-updated 事件，事件 payload 是完整状态对象。失败保留可理解的信息，释放互斥锁。

## 跨层契约

```typescript
type ClientState = "not_found" | "ready" | "launching" | "pending" | "active" | "blocked" | "error";
interface LocalizedClient {
  name: string;
  executable: string;
  source: string;
  version?: string | null;
  appId?: string | null;
}
interface LocalizedClientStatus {
  supported: boolean;
  configured: boolean;
  active: boolean;
  state: ClientState;
  client: LocalizedClient | null;
  message: string;
  reason?: string | null;
  debugPort?: number | null;
}
```

Tauri 命令：get_chinese_chatgpt_status、scan_chinese_chatgpt、open_chinese_chatgpt、restart_chinese_chatgpt、open_chatgpt_client 都返回 LocalizedClientStatus；pick_chinese_chatgpt_path 返回 string | null；set_chinese_chatgpt_path 接受 path: string | null，返回 LocalizedClientStatus。原有 open_app_log_folder 与 open_external 复用。

## 更新、数据和测试

定制版不能自动安装 CC Switch 或 003 Launcher 的发行包。自定义签名更新服务尚未建立时，采用明确显示的手动更新；保留上游版本来源信息，不把上游更新安装成自定义更新。

复用原 ~/.cc-switch 数据目录与上游 schema 20 的迁移机制，真实运行升级前备份；测试与 UI 验证使用 CC_SWITCH_TEST_HOME 指向项目隔离目录，不改 HOME、USERPROFILE 或 CODEX_HOME。不在本次测试中关闭用户正在使用的 Codex 客户端。

验证覆盖：恢复脚本的中文判定、不将中文聊天内容误判为汉化；端点和路径验证；CDP 假服务的成功、异常、超时；状态并发与事件；导航、帮助与状态展示；类型检查、前端构建、相关上游迁移测试、Windows 原生构建、NSIS 安装包。界面在 1000×650 与 900×600 下检查文字及操作溢出。

交付源码、可运行 EXE、NSIS 安装包、版本与校验说明。未实测的特定官方客户端版本兼容性必须如实标明。
