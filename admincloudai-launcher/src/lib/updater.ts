import { getVersion } from "@tauri-apps/api/app";

export const CUSTOM_UPDATE_URL = "https://api.admincloudai.com/home";
export const MANUAL_UPDATE_MESSAGE =
  "AdminCloudAI 定制版仅支持手动更新，请从官网或联系支持获取兼容的定制安装包。";

export interface UpdateInfo {
  currentVersion: string;
  availableVersion: string;
  notes?: string;
  pubDate?: string;
}

export async function getCurrentVersion(): Promise<string> {
  try {
    return await getVersion();
  } catch {
    return "";
  }
}

// No signed custom update service exists. Never query an upstream feed.
export async function checkForUpdate(): Promise<{ status: "manual" }> {
  return { status: "manual" };
}
