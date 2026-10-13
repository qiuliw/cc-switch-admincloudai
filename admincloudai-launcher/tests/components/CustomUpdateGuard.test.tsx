import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { checkForUpdate } from "@/lib/updater";
import { UpdateProvider, useUpdate } from "@/contexts/UpdateContext";
import { DatabaseUpgrade } from "@/components/DatabaseUpgrade";
import { AboutSection } from "@/components/settings/AboutSection";
import { settingsApi } from "@/lib/api/settings";

const boundary = vi.hoisted(() => ({ check: vi.fn(), invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: boundary.check }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: boundary.invoke }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: async () => "4.0.9" }));
vi.mock("@tauri-apps/plugin-process", () => ({ exit: vi.fn() }));

describe("custom application update guard", () => {
  beforeEach(() => {
    boundary.check.mockResolvedValue({ version: "99.0.0" });
    boundary.invoke.mockResolvedValue(null);
  });

  it("returns manual status without querying a foreign updater", async () => {
    expect(await checkForUpdate()).toEqual({ status: "manual" });
    expect(boundary.check).not.toHaveBeenCalled();
  });

  it("never checks upstream on mount or an explicit context check", async () => {
    function Consumer() {
      const update = useUpdate();
      return (
        <button onClick={() => void update.checkUpdate()}>
          {String(update.hasUpdate)}
        </button>
      );
    }
    vi.useFakeTimers();
    try {
      render(
        <UpdateProvider>
          <Consumer />
        </UpdateProvider>,
      );
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1500);
      });
      await act(async () => {
        fireEvent.click(screen.getByRole("button"));
      });
      expect(screen.getByRole("button")).toHaveTextContent("false");
      expect(boundary.check).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it("provides honest manual database recovery and preserves diagnostic actions", async () => {
    render(
      <DatabaseUpgrade
        payload={{
          path: "isolated/cc-switch.db",
          db_version: 21,
          supported_version: 20,
        }}
      />,
    );
    expect(screen.getByText(/手动更新/)).toBeInTheDocument();
    expect(screen.queryByText(/已是最新/)).not.toBeInTheDocument();
    expect(boundary.invoke).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /官网/ }));
    expect(boundary.invoke).toHaveBeenCalledWith("open_external", {
      url: "https://api.admincloudai.com/home",
    });
    fireEvent.click(screen.getByRole("button", { name: /日志/ }));
    expect(boundary.invoke).toHaveBeenCalledWith("open_app_log_folder");
  });

  it("brands About and links manual updates without install or latest claims", async () => {
    render(
      <UpdateProvider>
        <AboutSection isPortable={false} />
      </UpdateProvider>,
    );
    expect(await screen.findByText(/beta 0.3/)).toBeInTheDocument();
    expect(screen.getByText("AdminCloudAI 启动器")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /手动更新/ }));
    expect(boundary.invoke).toHaveBeenCalledWith("open_external", {
      url: "https://api.admincloudai.com/home",
    });
    expect(boundary.check).not.toHaveBeenCalled();
    expect(boundary.invoke).not.toHaveBeenCalledWith(
      "install_update_and_restart",
    );
  });

  it("rejects legacy install callers and redirects legacy update links to the custom site", async () => {
    await expect(settingsApi.installUpdateAndRestart()).rejects.toThrow(
      /手动更新/,
    );
    expect(boundary.invoke).not.toHaveBeenCalled();
    await settingsApi.checkUpdates();
    expect(boundary.invoke).toHaveBeenCalledWith("open_external", {
      url: "https://api.admincloudai.com/home",
    });
  });
});
