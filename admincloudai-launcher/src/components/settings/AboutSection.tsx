import { useEffect, useMemo, useState } from "react";
import {
  Download,
  ExternalLink,
  Github,
  Info,
  Loader2,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { settingsApi } from "@/lib/api";
import { useUpdate } from "@/contexts/UpdateContext";
import { Badge } from "@/components/ui/badge";
import { WhatsNewDialog } from "@/components/WhatsNewDialog";
import { WHATS_NEW_ENTRIES, entriesUpTo } from "@/lib/whatsNew";
import { extractErrorMessage } from "@/utils/errorUtils";
import { toast } from "@/lib/toast";
import appIcon from "@/assets/admincloud/app-icon.png";

export function AboutSection({ isPortable }: { isPortable: boolean }) {
  const { t } = useTranslation();
  const { hasUpdate, updateInfo, isChecking, checkUpdate, resetDismiss } =
    useUpdate();
  const [version, setVersion] = useState<string | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [whatsNewOpen, setWhatsNewOpen] = useState(false);
  const recentEntries = useMemo(
    () => (version ? entriesUpTo(WHATS_NEW_ENTRIES, version) : []),
    [version],
  );
  useEffect(() => {
    let active = true;
    void getVersion()
      .then((v) => {
        if (active) setVersion(v);
      })
      .catch(() => {
        if (active) setVersion(t("common.unknown"));
      });
    return () => {
      active = false;
    };
  }, [t]);
  const open = async (url: string) => {
    try {
      await settingsApi.openExternal(url);
    } catch (e) {
      toast.error(String(e));
    }
  };
  const handleCheckUpdate = async () => {
    if (hasUpdate) {
      if (isPortable) {
        try {
          await settingsApi.checkUpdates();
        } catch (error) {
          console.error("[AboutSection] Portable update failed", error);
        }
        return;
      }

      setIsDownloading(true);
      try {
        resetDismiss();
        const installed = await settingsApi.installUpdateAndRestart();
        if (!installed) {
          toast.success(t("settings.upToDate"), { closeButton: true });
        }
      } catch (error) {
        console.error("[AboutSection] Update failed", error);
        toast.error(t("settings.updateFailed"), {
          description: extractErrorMessage(error) || undefined,
          closeButton: true,
        });
        try {
          await settingsApi.checkUpdates();
        } catch (fallbackError) {
          console.error(
            "[AboutSection] Failed to open fallback updater",
            fallbackError,
          );
        }
      } finally {
        setIsDownloading(false);
      }
      return;
    }

    try {
      const available = await checkUpdate();
      if (!available) {
        toast.success(t("settings.upToDate"), { closeButton: true });
      }
    } catch (error) {
      console.error("[AboutSection] Check update failed", error);
      toast.error(t("settings.checkUpdateFailed"), {
        description: extractErrorMessage(error) || undefined,
        closeButton: true,
      });
    }
  };
  return (
    <div className="divide-y divide-border">
      <div className="flex flex-wrap items-center gap-4 py-4">
        <img src={appIcon} alt="" className="h-10 w-10 shrink-0" />
        <div className="min-w-0 flex-1">
          <div className="text-title text-fg-1">AdminCloudAI 启动器</div>
          <div className="flex flex-wrap items-center gap-2 text-caption text-fg-2">
            <span>beta 0.3.1</span>
            {version ? (
              <span>
                {t("admincloudAbout.core", "核心")} v{version}
              </span>
            ) : (
              <Loader2 className="h-3 w-3 animate-spin" />
            )}
            {isPortable && (
              <Badge variant="secondary">
                <Info className="h-3 w-3" />
                {t("settings.portableMode")}
              </Badge>
            )}
          </div>
        </div>
        <Button
          type="button"
          variant={hasUpdate ? "solid" : "neutral"}
          size="regular"
          onClick={() => void handleCheckUpdate()}
          disabled={isChecking || isDownloading}
        >
          {isDownloading ? (
            <>
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              {t("settings.updating")}
            </>
          ) : hasUpdate ? (
            <>
              <Download className="h-3.5 w-3.5" />
              {t("settings.updateTo", {
                version: updateInfo?.availableVersion ?? "",
              })}
            </>
          ) : isChecking ? (
            <>
              <RefreshCw className="h-3.5 w-3.5 animate-spin" />
              {t("settings.checking")}
            </>
          ) : (
            <>
              <RefreshCw className="h-3.5 w-3.5" />
              {t("settings.checkForUpdates")}
            </>
          )}
        </Button>
      </div>
      {hasUpdate && updateInfo && (
        <div className="space-y-1 py-4 text-body">
          <p className="font-medium text-fg-1">
            {t("settings.updateAvailable", {
              version: updateInfo.availableVersion,
            })}
          </p>
          {updateInfo.notes && (
            <p className="line-clamp-3 text-fg-2">{updateInfo.notes}</p>
          )}
        </div>
      )}
      <div className="space-y-2 py-4 text-body text-fg-2">
        <p>
          {t(
            "admincloudAbout.updates",
            "通过 AdminCloudAI 自建签名升级服务在线更新；不会下载或安装上游发行包。",
          )}
        </p>
        <p>
          {t(
            "admincloudAbout.attribution",
            "AdminCloudAI 定制发行，基于 CC Switch v4.0.9。上游作者 Jason Young，遵循 MIT 许可证；保留上游版权与开源致谢。",
          )}
        </p>
      </div>
      <div className="flex flex-wrap gap-2 py-4">
        <Button
          variant="neutral"
          size="compact"
          onClick={() => void open("https://github.com/farion1231/cc-switch")}
        >
          <Github className="h-3.5 w-3.5" />
          {t("admincloudAbout.upstream", "上游项目与许可证")}
        </Button>
        <Button
          variant="neutral"
          size="compact"
          onClick={() =>
            void open(
              "https://github.com/farion1231/cc-switch/releases/tag/v4.0.9",
            )
          }
        >
          <ExternalLink className="h-3.5 w-3.5" />
          {t("admincloudAbout.upstreamNotes", "上游 v4.0.9 说明")}
        </Button>
        {recentEntries.length > 0 && (
          <Button
            variant="neutral"
            size="compact"
            onClick={() => setWhatsNewOpen(true)}
          >
            <Sparkles className="h-3.5 w-3.5" />
            {t("whatsNew.recentTitle")}
          </Button>
        )}
      </div>
      <WhatsNewDialog
        open={whatsNewOpen}
        onClose={() => setWhatsNewOpen(false)}
        entries={recentEntries}
      />
    </div>
  );
}
