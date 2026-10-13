import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  // 开发时代理到本机 cargo run 的后端
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8080",
      "/i": "http://127.0.0.1:8080",
      "/downloads": "http://127.0.0.1:8080",
    },
  },
});
