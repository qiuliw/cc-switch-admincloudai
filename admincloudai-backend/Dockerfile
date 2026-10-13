# ---- 构建网页 ----
FROM node:24-slim AS web
WORKDIR /web
COPY web/package.json ./
RUN corepack enable && pnpm install
COPY web/ ./
RUN pnpm build

# ---- 构建服务 ----
FROM rust:1-slim AS server
WORKDIR /server
COPY server/Cargo.toml server/Cargo.lock* ./
COPY server/src ./src
RUN cargo build --release

# ---- 运行时 ----
FROM debian:stable-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
COPY --from=server /server/target/release/admincloudai-server /usr/local/bin/admincloudai-server
COPY --from=web /web/dist /web-dist
ENV BIND=0.0.0.0:8080 DATA_DIR=/data WEB_DIR=/web-dist
VOLUME /data
EXPOSE 8080
CMD ["admincloudai-server"]
