import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  FolderOpen,
  HelpCircle,
  Languages,
  Loader2,
  Play,
  RefreshCw,
  RotateCcw,
} from "lucide-react";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { useLocalizedClient } from "@/hooks/useLocalizedClient";
import { localizedClientApi } from "@/lib/api/localizedClient";
import { extractErrorMessage } from "@/utils/errorUtils";
import { SupportDialog } from "./SupportDialog";
import appIcon from "@/assets/admincloud/app-icon.png";

function DetailRow({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="grid grid-cols-[110px_minmax(0,1fr)] gap-4 border-b border-border py-3 text-body">
      <dt className="text-fg-3">{label}</dt>
      <dd className="min-w-0 break-all text-fg-1">{children}</dd>
    </div>
  );
}

export function LocalizedClientPage() {
  const { t } = useTranslation();
  const client = useLocalizedClient();
  const { status, loading, busy, error, subscriptionError } = client;
  const [restartOpen, setRestartOpen] = useState(false);
  const [supportOpen, setSupportOpen] = useState(false);
  const [logError, setLogError] = useState<string | null>(null);
  const selected = status?.client;
  const enabled = !loading && !busy && status?.supported === true;
  const canLaunch = enabled && Boolean(selected) && status?.state !== "pending";
  const state =
    status?.state === "active" && !status.active ? "pending" : status?.state;
  const hintKey =
    !status?.supported && status
      ? "unsupported"
      : state === "not_found"
        ? "missingHint"
        : status?.reason === "restart-required"
          ? "restartHint"
          : status?.reason === "operation-in-progress"
            ? "busyHint"
            : status?.reason === "client-already-running"
              ? "originalHint"
              : state === "pending"
                ? "pendingHint"
                : state === "error"
                  ? "errorHint"
                  : null;

  const openLogs = async () => {
    setLogError(null);
    try {
      await localizedClientApi.openLogs();
    } catch (cause) {
      setLogError(extractErrorMessage(cause) || String(cause));
    }
  };

  return (
    <>
      <AppPageHeader
        title={t("localizedClient.title")}
        icon={<Languages className="h-5 w-5" />}
        actions={
          <>
            <Button
              variant="solid"
              size="regular"
              disabled={!canLaunch}
              onClick={() => void client.launch()}
            >
              <Play className="h-4 w-4" />
              {t("localizedClient.launch")}
            </Button>
            <Button
              variant="outline"
              size="regular"
              disabled={!enabled || !selected}
              onClick={() => setRestartOpen(true)}
            >
              <RotateCcw className="h-4 w-4" />
              {t("localizedClient.restart")}
            </Button>
          </>
        }
      />
      <div
        id="main-content"
        className="min-h-0 flex-1 overflow-y-auto scroll-stable"
      >
        <div className="mx-auto max-w-4xl px-6 pb-8 pt-5">
          <div className="flex items-center gap-3 border-b border-border pb-4">
            <img src={appIcon} alt="" className="h-9 w-9 shrink-0" />
            <div className="min-w-0">
              <h2 className="text-title font-semibold">AdminCloudAI</h2>
              <p className="text-caption text-fg-3">
                {t("localizedClient.clientName")}
              </p>
            </div>
            <Button
              variant="quiet"
              size="compact"
              className="ms-auto shrink-0"
              onClick={() => setSupportOpen(true)}
            >
              <HelpCircle className="h-4 w-4" />
              {t("localizedClient.support")}
            </Button>
          </div>
          <div
            className="border-b border-border py-4"
            role="status"
            aria-live="polite"
          >
            <div className="flex items-center gap-2 font-medium text-strong">
              {(loading || busy) && (
                <Loader2 aria-hidden="true" className="h-4 w-4 animate-spin" />
              )}
              <span
                className={
                  state === "active" && status?.active
                    ? "text-success"
                    : state === "error"
                      ? "text-danger"
                      : "text-fg-1"
                }
              >
                {t(`localizedClient.states.${state ?? "loading"}`)}
              </span>
            </div>
            {status?.message && (
              <p className="mt-1 break-words text-body text-fg-2">
                {status.message}
              </p>
            )}
            {hintKey && (
              <p className="mt-1 break-words text-body text-fg-3">
                {t(`localizedClient.${hintKey}`)}
              </p>
            )}
          </div>
          {(error || logError) && (
            <p role="alert" className="mt-3 break-words text-body text-danger">
              {error || logError}
            </p>
          )}
          {subscriptionError && (
            <p role="alert" className="mt-3 break-words text-body text-danger">
              {t("localizedClient.subscriptionError", {
                error: subscriptionError,
              })}
            </p>
          )}
          <dl>
            <DetailRow label={t("localizedClient.name")}>
              {selected?.name ?? t("localizedClient.notDetected")}
            </DetailRow>
            <DetailRow label={t("localizedClient.source")}>
              {selected
                ? t(`localizedClient.sources.${selected.source}`, {
                    defaultValue: selected.source,
                  })
                : "—"}
            </DetailRow>
            <DetailRow label={t("localizedClient.path")}>
              {selected?.executable ?? "—"}
            </DetailRow>
            <DetailRow label={t("localizedClient.pathMode")}>
              {t(
                `localizedClient.${status?.configured ? "manualPath" : "autoPath"}`,
              )}
            </DetailRow>
            <DetailRow label={t("localizedClient.version")}>
              {selected?.version ?? "—"}
            </DetailRow>
            <DetailRow label={t("localizedClient.running")}>
              {t(
                `localizedClient.runningStates.${state === "active" || state === "pending" || status?.reason === "restart-required" || status?.reason === "client-already-running" ? "detected" : state === "launching" ? "launching" : "unknown"}`,
              )}
            </DetailRow>
          </dl>
          <div className="flex flex-wrap gap-2 pt-4">
            <Button
              variant="outline"
              size="regular"
              disabled={loading || busy || status?.supported === false}
              onClick={() => void client.scan()}
            >
              <RefreshCw className="h-4 w-4" />
              {t("localizedClient.scan")}
            </Button>
            <Button
              variant="outline"
              size="regular"
              disabled={!enabled}
              onClick={() => void client.choosePath()}
            >
              <FolderOpen className="h-4 w-4" />
              {t("localizedClient.choosePath")}
            </Button>
            <Button
              variant="quiet"
              size="regular"
              disabled={!enabled}
              onClick={() => void client.resetPath()}
            >
              <RotateCcw className="h-4 w-4" />
              {t("localizedClient.resetPath")}
            </Button>
            <Button
              variant="quiet"
              size="regular"
              disabled={!enabled || !selected}
              onClick={() => void client.openOriginal()}
            >
              <Play className="h-4 w-4" />
              {t("localizedClient.original")}
            </Button>
            <Button
              variant="quiet"
              size="regular"
              onClick={() => void openLogs()}
            >
              <FolderOpen className="h-4 w-4" />
              {t("localizedClient.logs")}
            </Button>
          </div>
        </div>
      </div>
      <ConfirmDialog
        isOpen={restartOpen}
        title={t("localizedClient.restart")}
        message={t("localizedClient.restartConfirm", {
          path: selected?.executable ?? "",
        })}
        confirmText={t("localizedClient.confirmRestart")}
        cancelText={t("localizedClient.cancel")}
        pending={busy}
        onCancel={() => setRestartOpen(false)}
        onConfirm={() => {
          setRestartOpen(false);
          void client.restart();
        }}
      />
      <SupportDialog open={supportOpen} onOpenChange={setSupportOpen} />
    </>
  );
}
