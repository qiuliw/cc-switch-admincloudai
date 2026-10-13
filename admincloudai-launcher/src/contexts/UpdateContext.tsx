import React, { createContext, useContext } from "react";
import type { UpdateInfo } from "../lib/updater";
import { checkForUpdate } from "../lib/updater";

interface UpdateContextValue {
  hasUpdate: boolean;
  updateInfo: UpdateInfo | null;
  isChecking: boolean;
  error: string | null;
  isDismissed: boolean;
  dismissUpdate: () => void;
  checkUpdate: () => Promise<boolean>;
  resetDismiss: () => void;
}

const manualUpdate: UpdateContextValue = {
  hasUpdate: false,
  updateInfo: null,
  isChecking: false,
  error: null,
  isDismissed: false,
  dismissUpdate: () => {},
  resetDismiss: () => {},
  checkUpdate: async () => {
    await checkForUpdate();
    return false;
  },
};
const UpdateContext = createContext<UpdateContextValue | undefined>(undefined);

export function UpdateProvider({ children }: { children: React.ReactNode }) {
  return (
    <UpdateContext.Provider value={manualUpdate}>
      {children}
    </UpdateContext.Provider>
  );
}

export function useUpdate() {
  const context = useContext(UpdateContext);
  if (!context) throw new Error("useUpdate must be used within UpdateProvider");
  return context;
}
