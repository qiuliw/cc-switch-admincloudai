# AdminCloudAI 启动器 beta 0.3.1

核心版本：4.0.9。自定义应用标识：`com.admincloudai.launcher`。

上游为 CC Switch v4.0.9，提交 `8845b1ef799aa8810b864b122de6af5f4629de10`，作者 Jason Young。保留仓库根目录 MIT LICENSE、原版权、供应商/路由/聚合/工具管理功能和 schema 20 迁移。本项目是 AdminCloudAI 定制发行，不代表 OpenAI 官方产品。

## Windows 构建

需要 Windows x64、Node.js 20.19+、pnpm、Rust 1.95 MSVC 工具链、Visual Studio 2022 Build Tools（Desktop development with C++ 与 Windows SDK）。运行程序需要 WebView2 Runtime。安装包由 Tauri NSIS bundler 生成，首次打包会下载 NSIS 工具。

在 x64 Native Tools / Developer PowerShell 中进入源码根目录：

```powershell
rustup toolchain install 1.95 --profile minimal
$env:RUSTUP_TOOLCHAIN = '1.95'
pnpm install --frozen-lockfile
pnpm typecheck
cargo +1.95 test --manifest-path src-tauri/Cargo.toml --lib localized_client
cargo +1.95 test --manifest-path src-tauri/Cargo.toml --lib migrate_v
pnpm tauri build --bundles nsis
```

`package.json` 声明 pnpm 10.12.3；本次环境提供 pnpm 11.19.0，已用锁定依赖构建，不升级依赖版本。执行上面的标准命令不需要本机 Codex runtime 路径或工作区外的辅助脚本。

产物位于 `src-tauri/target/release/cc-switch.exe` 和 `src-tauri/target/release/bundle/nsis/`。内部 Cargo 包名及 EXE 名保留上游兼容性，Windows 产品资源、安装程序名、窗口和托盘显示 AdminCloudAI 启动器。交付目录中的 EXE 可直接运行；它默认仍使用下述共享配置目录，不代表数据自动便携化。

桌面快捷方式名称为 `codex中文版启动器`。NSIS 使用 `src-tauri/nsis/installer.nsi` 中的 Tauri CLI 2.10.1 模板，单独设置桌面快捷方式名称；开始菜单和应用品牌继续使用产品名。安装时仅迁移指向同一安装目录的旧桌面快捷方式，卸载时按新名称移除。修改安装模板后可用 `pnpm tauri bundle --bundles nsis` 直接重新打包已有程序。

Tauri 在 NSIS 打包时将 EXE 的安装类型标记从 `UNK` 临时写为 `NSS`，打包后恢复 target 中的 EXE。本次实际比较两者只有该处 3 字节不同。为使交付的独立 EXE 与安装包载荷严格一致，交付 EXE 直接从已生成的 NSIS 安装包解出，再以描述性文件名复制；没有手工修改二进制。最后的隔离启动与截图验证针对这份交付 EXE。可用 `scripts/admincloud-verify-artifacts.ps1` 验证载荷、版本和全部 ICO 图像帧。

网络无法访问 crates.io 时，可创建临时 TOML 文件：

```toml
[source.crates-io]
replace-with = "rsproxy-sparse"
[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"
```

然后运行 `pnpm tauri build --bundles nsis -- --config C:/absolute/path/cargo-mirror.toml`。此参数透传 Cargo，仅影响本次命令，不修改用户/全局 Cargo 配置。Windows 的 pnpm.cmd 会剥离部分嵌套引号，传 TOML 文件路径比在命令行内嵌 TOML 字符串可靠。

## 数据与更新

默认复用 `~/.cc-switch`。新的应用标识会使用独立的 Tauri `app_paths.json`；如果旧应用配置过非默认数据目录，需要在新启动器设置中重新选择原目录，不会自动搬运其他应用身份下的路径覆盖设置。真实升级前先退出启动器，完整备份该目录。上游对已有旧 schema 数据库在迁移前创建 `backups/db_backup_*.db`，并执行 SQLite 完整性验证和原子发布；但是自动备份失败时上游会记录警告并继续迁移，因此不能代替手动完整备份。测试使用 `CC_SWITCH_TEST_HOME`，不修改 HOME、USERPROFILE 或 CODEX_HOME。

在线升级为上游原生实现（`tauri-plugin-updater`），仅将升级 feed 指向自建服务：
`https://api.admincloudai.com/api/update/{{target}}/{{current_version}}`，公钥见
`tauri.conf.json` 的 `plugins.updater.pubkey`。构建安装包前需设置
`TAURI_SIGNING_PRIVATE_KEY`（及可选 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`），
`tauri build` 会生成 `.sig` 签名文件；把安装包与签名发布到后端服务的
`data/updates/` 并更新 `data/versions.json` 即完成发版（见 `../admincloudai-backend/README.md`）。
关于页提供「检查更新 / 更新到 vX.Y.Z」，下载安装由后端命令 `install_update_and_restart`
完成并自动重启；数据库恢复页在上游流程上仅把发布页链接指向 AdminCloudAI 官网。
在线客服二维码地址按同样方式配置：`tauri.conf.json` → `plugins.admincloud.qrUrl`，
拉取失败时界面回落打包内的内置二维码。
上游项目链接仅用于来源和许可证说明，不会下载或安装上游应用更新。应用页的独立工具升级功能保留。

随安装包内置的技能位于 `src-tauri/resources/skills/`（imagegen-enhance，vendor 自
https://github.com/qiuliw/imagegen-enhance，Apache-2.0）。用户可在「技能」页的更多菜单
一键安装，安装命令对目录做白名单校验（`commands::skill::BUILTIN_SKILLS`）。

## 隔离启动与界面验证

```powershell
$env:CC_SWITCH_TEST_HOME = Join-Path (Get-Location) '.test-home/manual'
& './src-tauri/target/release/cc-switch.exe'
```

自动验证脚本只启动它自己的新启动器进程，通过该进程的 WebView2 回环调试端口读取真实页面；只扫描客户端，不启动、重启或注入用户的真实 Codex/ChatGPT。它检查两个应用更新原生命令及已移除的插件均拒绝调用，并在 1000×650 和 900×600 下检查主要操作边界、非空内容、横向溢出及截图。可打开重启确认后取消，不确认重启。

```powershell
node scripts/admincloud-runtime-smoke.mjs ./src-tauri/target/release/cc-switch.exe ./.test-home/smoke ../release/verification C:/path/to/node_modules/playwright
```

Playwright 仅作为外部验证工具，不是应用依赖。脚本最后只结束它创建的启动器子进程。截图使用真实原生 WebView 的不同渲染视口，不是模拟 Tauri 的网页；客户端路径字段在共享截图中遮盖。

## 图标与兼容性

原图为恢复资料 `assets/app-icon-DbFvPcDW.png`，已原样保存为 `src/assets/admincloud/app-icon.png`。两者 SHA-256 均为 `7843B3BDBE006F19539A49F177FE407956B885C78BFAF053B2F74C10E87ADA24`。构建图标由 `pnpm tauri icon src/assets/admincloud/app-icon.png --output <临时目录>` 机械转换，再将对应 PNG/ICO/ICNS 复制到 `src-tauri/icons`；未重新绘制图标。

汉化服务使用恢复并审阅的 Statsig i18n hook，只有确认官方客户端页面外壳已为中文才返回 active。Node hook、假 CDP 服务、路径/进程/端口归属、忙碌互斥和错误处理有自动测试。beta 0.3.1 修复了远程语言配置为空时漏掉汉化的情况：当前已核对的官方客户端使用语言层 `72216192`，该层即使为空也仅覆盖 `enable_i18n` 和 `locale_source`，不修改其他功能层。已有旧钩子的受控客户端也可升级钩子。2026-10-12 已在安装的 Microsoft Store 客户端 `26.1007.2314.0` 中注入脚本、刷新页面，并验证菜单和侧栏为中文；未结束客户端进程。安装包未覆盖安装到用户的活动目录；仅解包检查。

外壳验证使用固定动作身份白名单：导航区域内精确指向 `/new`、`/tasks`、`/plugins`、`/projects`、`/settings` 的链接，以及当前 Codex 的 `button.sidebar-item[data-sidebar-destination]` 内置按钮（仅 `builtin:automations` 和 `builtin:skills`）。至少两个不同动作各自显示对应中文标签才算成功；同一动作的链接和按钮不会重复计数。历史会话、具体项目、外部链接、内容区域里的按钮和链接均不匹配。普通按钮、任意中文文本、`title`、文档语言及用户条目的 `aria-label` 均不能单独作为依据；仅已识别动作的无文字图标控件可读取对应中文 `aria-label`。未知动作或其他界面结构仍保持未验证/等待状态。

精确产物 SHA-256、Windows 版本资源、安装包载荷比较和本次测试结果见交付目录 `verification` 与任务报告。
