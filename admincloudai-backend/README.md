# AdminCloudAI Backend

AdminCloudAI 启动器的后端服务：**在线升级 feed + 在线客服二维码**。
单二进制（axum）+ 单页官网（Svelte 静态构建），Docker Compose 一键部署，无数据库。

## 目录

```
admincloudai-backend/
├── server/          # axum 后端（Rust）
├── web/             # 官网单页（Svelte 5 + Vite）
├── data/            # 运行数据（挂载卷，改文件即生效）
│   ├── versions.json   # 升级信息（Tauri v2 updater 格式）
│   ├── qr.png          # 客服二维码（替换文件即轮换）
│   └── updates/        # 安装包存放处
├── Dockerfile
└── docker-compose.yml
```

## API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/update/{target}/{current_version}` | Tauri v2 updater feed，有新版返回 200 JSON，否则 204 |
| GET | `/api/qr` | 在线客服二维码（`versions.json` 的 `qr_url` 优先，302 跳转；否则返回 `data/qr.png`） |
| GET | `/downloads/{file}` | 下载安装包 |
| GET | `/health` | 健康检查 |

静态官网挂在 `/`。

## 部署

```bash
docker compose up -d --build
```

环境变量：`BIND`（默认 0.0.0.0:8080）、`DATA_DIR`（默认 /data）、`WEB_DIR`。

## 发新版本流程

1. 生成签名密钥（一次性，私钥务必保存好）：
   ```bash
   cd ../admincloudai-launcher
   pnpm tauri signer generate -w ../admincloudai-backend/.secrets/admincloudai.key
   ```
   公钥已写入 `tauri.conf.json` 的 `plugins.updater.pubkey`；构建时通过
   `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 提供私钥。
2. 构建 NSIS 安装包：`pnpm tauri build --bundles nsis`（产物带 `.sig` 签名文件）。
3. 把安装包放入 `data/updates/`，更新 `data/versions.json` 的 `version`、`notes`、
   `signature`（`.sig` 文件全文）与 `url`。客户端启动即自动检查更新。

## 本地开发

```bash
# 终端 1：后端
cd server && cargo run
# 终端 2：网页（vite 代理 /api 到 127.0.0.1:8080）
cd web && pnpm install && pnpm dev
```

不写 Svelte 时后端也可单独跑：`cargo run -- --web-dir` 不支持时可用
`WEB_DIR=../web/dist`（先 `pnpm build`）。

## 客户端对接

- 启动器（`admincloudai-launcher`）通过 `tauri-plugin-updater` 请求
  `https://api.admincloudai.com/api/update/{{target}}/{{current_version}}`。
- 客服二维码：启动器 `get_online_support_qr` 命令拉取 `/api/qr`，失败回落内置图片。
- 图片技能（imagegen-enhance）不依赖本服务：它随启动器安装包内置
  （客户端 `src-tauri/resources/skills/`），离线可用。
