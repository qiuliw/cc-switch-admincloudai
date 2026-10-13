import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  LOCALIZED_CLIENT_EVENT,
  localizedClientApi,
  type LocalizedClientStatus,
} from "@/lib/api/localizedClient";
import { extractErrorMessage } from "@/utils/errorUtils";

export function useLocalizedClient() {
  const [status, setStatus] = useState<LocalizedClientStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [subscriptionError, setSubscriptionError] = useState<string | null>(
    null,
  );
  const mounted = useRef(false);
  const inFlight = useRef(false);
  const request = useRef(0);
  const eventRevision = useRef(0);

  const execute = useCallback(
    async (operation: () => Promise<LocalizedClientStatus | null>) => {
      if (!mounted.current || inFlight.current) return;
      inFlight.current = true;
      const id = ++request.current;
      const revision = eventRevision.current;
      setWorking(true);
      setError(null);
      try {
        const next = await operation();
        // Native events may have delivered newer progress or proof while this request awaited.
        if (
          mounted.current &&
          id === request.current &&
          revision === eventRevision.current &&
          next
        ) {
          setStatus(next);
        }
      } catch (cause) {
        if (mounted.current && id === request.current) {
          setError(extractErrorMessage(cause) || String(cause));
        }
      } finally {
        if (mounted.current && id === request.current) {
          inFlight.current = false;
          setWorking(false);
          setLoading(false);
        }
      }
    },
    [],
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    mounted.current = true;
    inFlight.current = false;
    void (async () => {
      try {
        const off = await listen<LocalizedClientStatus>(
          LOCALIZED_CLIENT_EVENT,
          ({ payload }) => {
            if (disposed) return;
            eventRevision.current += 1;
            setStatus(payload);
            setError(null);
          },
        );
        if (disposed) {
          off();
          return;
        }
        unlisten = off;
        setSubscriptionError(null);
      } catch (cause) {
        if (!disposed)
          setSubscriptionError(extractErrorMessage(cause) || String(cause));
      }
      if (!disposed) await execute(localizedClientApi.getStatus);
    })();
    return () => {
      disposed = true;
      mounted.current = false;
      request.current += 1;
      unlisten?.();
    };
  }, [execute]);

  const busy = working || status?.state === "launching";
  return {
    status,
    loading,
    busy,
    error,
    subscriptionError,
    scan: () => execute(localizedClientApi.scan),
    launch: () => execute(localizedClientApi.launch),
    restart: () => execute(localizedClientApi.restart),
    openOriginal: () => execute(localizedClientApi.openOriginal),
    choosePath: () =>
      execute(async () => {
        const path = await localizedClientApi.pickPath();
        return path === null ? null : localizedClientApi.setPath(path);
      }),
    resetPath: () => execute(() => localizedClientApi.setPath(null)),
  };
}
