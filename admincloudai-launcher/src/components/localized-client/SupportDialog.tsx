import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { localizedClientApi } from "@/lib/api/localizedClient";
import { extractErrorMessage } from "@/utils/errorUtils";
import supportQr from "@/assets/admincloud/support-qr.png";

export function SupportDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);
  // 在线二维码优先（服务端可轮换），拉取失败回落内置图片
  const [qrSrc, setQrSrc] = useState<string>(supportQr);

  useEffect(() => {
    if (!open) {
      return;
    }
    let active = true;
    void localizedClientApi
      .getSupportQr()
      .then((online) => {
        if (active && online) {
          setQrSrc(online);
        }
      })
      .catch(() => {
        // 保留内置二维码
      });
    return () => {
      active = false;
    };
  }, [open]);

  const openLogs = async () => {
    setError(null);
    try {
      await localizedClientApi.openLogs();
    } catch (cause) {
      setError(extractErrorMessage(cause) || String(cause));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>{t("localizedClient.support")}</DialogTitle>
          <DialogDescription>
            {t("localizedClient.supportNotice")}
          </DialogDescription>
        </DialogHeader>
        <div className="flex justify-center px-6 py-5">
          <img
            src={qrSrc}
            alt={t("localizedClient.supportQr")}
            className="h-44 w-44 bg-white p-2 object-contain"
          />
        </div>
        {error && (
          <p
            role="alert"
            className="break-words px-6 pb-3 text-body text-danger"
          >
            {error}
          </p>
        )}
        <DialogFooter>
          <Button
            variant="quiet"
            size="regular"
            onClick={() => void openLogs()}
          >
            <FolderOpen className="h-4 w-4" />
            {t("localizedClient.logs")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
