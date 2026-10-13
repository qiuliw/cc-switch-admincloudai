import { useEffect, useMemo, useState } from "react";
import { ExternalLink, Github, Info, Loader2, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { settingsApi } from "@/lib/api";
import { Badge } from "@/components/ui/badge";
import { WhatsNewDialog } from "@/components/WhatsNewDialog";
import { WHATS_NEW_ENTRIES, entriesUpTo } from "@/lib/whatsNew";
import { CUSTOM_UPDATE_URL } from "@/lib/updater";
import { toast } from "@/lib/toast";
import appIcon from "@/assets/admincloud/app-icon.png";

export function AboutSection({ isPortable }: { isPortable: boolean }) {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string | null>(null);
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
        <Button variant="neutral" onClick={() => void open(CUSTOM_UPDATE_URL)}>
          <ExternalLink className="h-4 w-4" />
          {t("admincloudAbout.manualUpdate", "手动更新")}
        </Button>
      </div>
      <div className="space-y-2 py-4 text-body text-fg-2">
        <p>
          {t(
            "admincloudAbout.updates",
            "尚未建立定制签名更新服务。请从 AdminCloudAI 官网或联系支持获取兼容安装包；不会自动下载或安装上游发行包。",
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
