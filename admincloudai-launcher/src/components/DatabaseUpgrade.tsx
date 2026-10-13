import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { exit } from "@tauri-apps/plugin-process";
import {
  AlertTriangle,
  ExternalLink,
  FolderOpen,
  FileText,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { CUSTOM_UPDATE_URL } from "@/lib/updater";

interface DatabaseUpgradeProps {
  payload: {
    path?: string;
    error?: string;
    kind?: string;
    db_version?: number;
    supported_version?: number;
  };
}

export function DatabaseUpgrade({ payload }: DatabaseUpgradeProps) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);
  const perform = async (action: () => Promise<unknown>) => {
    try {
      setError(null);
      await action();
    } catch (e) {
      setError(String(e));
    }
  };
  return (
    <div className="flex min-h-screen items-center justify-center bg-surface p-6 text-fg-1">
      <div className="w-full max-w-lg space-y-5 rounded-lg border border-border bg-surface p-6">
        <div className="flex items-start gap-3">
          <AlertTriangle className="h-6 w-6 shrink-0 text-warning" />
          <div className="space-y-2">
            <h1 className="text-lg font-semibold">
              {t("dbUpgrade.title", "数据库版本过新")}
            </h1>
            <p className="text-sm text-fg-2">
              {t(
                "dbUpgrade.description",
                "当前数据库版本超出本启动器支持范围，暂时无法打开。",
              )}
            </p>
            {payload.db_version != null &&
              payload.supported_version != null && (
                <p className="text-xs text-fg-2">
                  {t("dbUpgrade.versionInfo", {
                    db: payload.db_version,
                    supported: payload.supported_version,
                    defaultValue:
                      "数据库版本 v{{db}} · 应用支持 v{{supported}}",
                  })}
                </p>
              )}
          </div>
        </div>
        <p className="text-sm leading-relaxed text-fg-2">
          {t(
            "dbUpgrade.manualRecovery",
            "请先备份整个配置目录，再联系 AdminCloudAI 支持确认兼容版本并手动更新。本定制版尚无自动更新服务，无法判断是否有新版本；请勿安装 CC Switch 或 003 Launcher 更新包覆盖本程序。",
          )}
        </p>
        <div className="space-y-1 rounded-lg bg-subtle p-3 text-xs text-fg-2">
          {payload.error && (
            <p className="break-words font-mono">{payload.error}</p>
          )}
          {payload.path && (
            <p className="break-all">
              {t("dbUpgrade.dbPath", "数据库文件")}：{payload.path}
            </p>
          )}
        </div>
        {error && (
          <p role="alert" className="break-words text-sm text-danger-text">
            {error}
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            onClick={() =>
              void perform(() =>
                invoke("open_external", { url: CUSTOM_UPDATE_URL }),
              )
            }
          >
            <ExternalLink className="h-4 w-4" />
            {t("dbUpgrade.openWebsite", "官网与支持")}
          </Button>
          <Button
            variant="outline"
            onClick={() => void perform(() => invoke("open_app_config_folder"))}
          >
            <FolderOpen className="h-4 w-4" />
            {t("dbUpgrade.openConfigDir", "打开配置目录")}
          </Button>
          <Button
            variant="outline"
            onClick={() => void perform(() => invoke("open_app_log_folder"))}
          >
            <FileText className="h-4 w-4" />
            {t("dbUpgrade.openLogs", "打开日志")}
          </Button>
          <Button variant="ghost" onClick={() => void perform(() => exit(0))}>
            {t("dbUpgrade.quit", "退出")}
          </Button>
        </div>
      </div>
    </div>
  );
}

export default DatabaseUpgrade;
