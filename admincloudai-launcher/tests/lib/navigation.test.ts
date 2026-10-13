import { beforeEach, describe, expect, it, vi } from "vitest";
import { parseView, readStoredView, storeView } from "@/lib/navigation";

describe("localized client navigation", () => {
  beforeEach(() => localStorage.clear());

  it("recognizes and persists the Chinese client global page", () => {
    expect(parseView("localizedClient")).toBe("localizedClient");
    storeView("localizedClient" as ReturnType<typeof readStoredView>);
    expect(readStoredView()).toBe("localizedClient");
  });

  it("opens the Chinese client page for a fresh or invalid preference", () => {
    expect(readStoredView()).toBe("localizedClient");
    localStorage.setItem("cc-switch-last-view", "unknown-page");
    expect(readStoredView()).toBe("localizedClient");
  });

  it("preserves valid saved views including migrated app views", () => {
    for (const [stored, expected] of [
      ["providers", "providers"],
      ["apps", "apps"],
      ["settings", "settings"],
      ["openclawTools", "openclawConfig"],
    ]) {
      localStorage.setItem("cc-switch-last-view", stored);
      expect(readStoredView()).toBe(expected);
    }
  });

  it("uses the Chinese client page when preference storage is unavailable", () => {
    const read = vi
      .spyOn(Storage.prototype, "getItem")
      .mockImplementation(() => {
        throw new Error("Storage unavailable");
      });
    expect(readStoredView()).toBe("localizedClient");
    read.mockRestore();
  });
});
