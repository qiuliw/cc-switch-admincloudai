import { invoke } from "@tauri-apps/api/core";

export type ClientState =
  | "not_found"
  | "ready"
  | "launching"
  | "pending"
  | "active"
  | "blocked"
  | "error";

export interface LocalizedClient {
  name: string;
  executable: string;
  source: string;
  version?: string | null;
  appId?: string | null;
}

export interface LocalizedClientStatus {
  supported: boolean;
  configured: boolean;
  active: boolean;
  state: ClientState;
  client: LocalizedClient | null;
  message: string;
  reason?: string | null;
  debugPort?: number | null;
}

export const ADMINCLOUD_HOME = "https://api.admincloudai.com/home";
export const LOCALIZED_CLIENT_EVENT = "chinese-chatgpt-status-updated";

export const localizedClientApi = {
  getStatus: () => invoke<LocalizedClientStatus>("get_chinese_chatgpt_status"),
  scan: () => invoke<LocalizedClientStatus>("scan_chinese_chatgpt"),
  launch: () => invoke<LocalizedClientStatus>("open_chinese_chatgpt"),
  restart: () => invoke<LocalizedClientStatus>("restart_chinese_chatgpt"),
  openOriginal: () => invoke<LocalizedClientStatus>("open_chatgpt_client"),
  pickPath: () => invoke<string | null>("pick_chinese_chatgpt_path"),
  setPath: (path: string | null) =>
    invoke<LocalizedClientStatus>("set_chinese_chatgpt_path", { path }),
  openLogs: () => invoke<void>("open_app_log_folder"),
  /** 在线客服二维码（data URL）；服务不可达返回 null */
  getSupportQr: () => invoke<string | null>("get_online_support_qr"),
};
