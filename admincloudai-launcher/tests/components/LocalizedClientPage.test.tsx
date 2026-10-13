import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "i18next";
import zh from "@/i18n/locales/zh.json";

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  unlisten: vi.fn(),
  handler: undefined as undefined | ((event: { payload: unknown }) => void),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: native.listen }));

const ready = {
  supported: true,
  configured: false,
  active: false,
  state: "ready",
  client: {
    name: "Codex",
    executable: "C:\\Apps\\Codex.exe",
    source: "Microsoft Store",
    version: "1.0",
    appId: null,
  },
  message: "已找到客户端，可以启动中文界面",
  reason: null,
  debugPort: null,
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

async function mountPage() {
  const { LocalizedClientPage } = await import(
    "@/components/localized-client/LocalizedClientPage"
  );
  let view!: ReturnType<typeof render>;
  await act(async () => {
    view = render(<LocalizedClientPage />);
  });
  return view;
}

function emit(payload: unknown) {
  act(() => native.handler?.({ payload }));
}

describe("LocalizedClientPage native workflow", () => {
  beforeEach(() => {
    i18n.addResourceBundle(
      "zh",
      "translation",
      { localizedClient: (zh as any).localizedClient },
      true,
      true,
    );
    native.handler = undefined;
    native.unlisten.mockReset();
    native.listen.mockReset().mockImplementation(async (_event, handler) => {
      native.handler = handler;
      return native.unlisten;
    });
    native.invoke.mockReset().mockResolvedValue(ready);
  });

  it("shows pending verification without success and locks launch until the native request settles", async () => {
    const opening = deferred<typeof ready>();
    native.invoke.mockImplementation((command) =>
      command === "open_chinese_chatgpt"
        ? opening.promise
        : Promise.resolve(ready),
    );
    await mountPage();
    const launch = await screen.findByRole("button", {
      name: "启动中文 ChatGPT",
    });
    await waitFor(() => expect(launch).toBeEnabled());
    fireEvent.click(launch);
    emit({ ...ready, state: "pending", message: "正在等待客户端中文界面验证" });
    expect(screen.getByRole("status")).toHaveTextContent("等待中文验证");
    expect(screen.getByRole("status")).not.toHaveTextContent("中文已启用");
    expect(launch).toBeDisabled();
    await act(async () => opening.resolve({ ...ready, state: "pending" }));
    expect(screen.getByRole("status")).not.toHaveTextContent("中文已启用");
  });

  it("keeps subscription failure visible while manual rescan still updates the status snapshot", async () => {
    native.listen.mockRejectedValue(new Error("事件通道连接失败"));
    native.invoke.mockImplementation((command) =>
      Promise.resolve(
        command === "scan_chinese_chatgpt"
          ? {
              ...ready,
              active: true,
              state: "active",
              message: "客户端中文界面已验证",
            }
          : ready,
      ),
    );
    await mountPage();
    const warning = screen.getByRole("alert");
    expect(warning).toHaveTextContent("事件通道连接失败");
    expect(warning).toHaveTextContent("手动重新扫描");
    expect(screen.getByRole("status")).toHaveTextContent("可以启动");
    const scan = screen.getByRole("button", { name: "重新扫描" });
    expect(scan).toBeEnabled();
    fireEvent.click(scan);
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent("中文已启用"),
    );
    expect(screen.getByRole("alert")).toHaveTextContent("事件通道连接失败");
  });

  it("keeps restart visible at the top and requires confirmation before invoking it", async () => {
    await mountPage();
    const header = screen.getByRole("banner");
    const restart = within(header).getByRole("button", {
      name: "重启中文 ChatGPT",
    });
    await waitFor(() => expect(restart).toBeEnabled());
    expect(native.invoke).not.toHaveBeenCalledWith("restart_chinese_chatgpt");
    fireEvent.click(restart);
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent("关闭所选客户端");
    expect(dialog).toHaveTextContent("C:\\Apps\\Codex.exe");
    expect(native.invoke).not.toHaveBeenCalledWith("restart_chinese_chatgpt");
    fireEvent.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(native.invoke).not.toHaveBeenCalledWith("restart_chinese_chatgpt");
    fireEvent.click(restart);
    fireEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "确认重启",
      }),
    );
    await waitFor(() =>
      expect(native.invoke).toHaveBeenCalledWith("restart_chinese_chatgpt"),
    );
  });

  it("does not allow a stale initial response to replace newer native evidence", async () => {
    const initial = deferred<typeof ready>();
    native.invoke.mockReturnValue(initial.promise);
    await mountPage();
    await waitFor(() => expect(native.handler).toBeDefined());
    emit({
      ...ready,
      state: "active",
      active: true,
      message: "客户端中文界面已验证",
    });
    await act(async () => initial.resolve(ready));
    expect(screen.getByRole("status")).toHaveTextContent("中文已启用");
  });

  it("ignores a stale operation response after a newer terminal event and unsubscribes on unmount", async () => {
    const scanning = deferred<typeof ready>();
    native.invoke.mockImplementation((command) =>
      command === "scan_chinese_chatgpt"
        ? scanning.promise
        : Promise.resolve(ready),
    );
    const view = await mountPage();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "重新扫描" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "重新扫描" }));
    emit({
      ...ready,
      state: "active",
      active: true,
      message: "客户端中文界面已验证",
    });
    await act(async () => scanning.resolve(ready));
    expect(screen.getByRole("status")).toHaveTextContent("中文已启用");
    view.unmount();
    expect(native.unlisten).toHaveBeenCalledOnce();
  });

  it("requires native active proof even when an invocation resolves successfully", async () => {
    native.invoke.mockResolvedValue({
      ...ready,
      state: "active",
      active: false,
    });
    await mountPage();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "重新扫描" })).toBeEnabled(),
    );
    expect(screen.getByRole("status")).not.toHaveTextContent("中文已启用");
  });

  it("offers path selection for a missing client and leaves selection cancellation untouched", async () => {
    native.invoke.mockImplementation((command) =>
      Promise.resolve(
        command === "pick_chinese_chatgpt_path"
          ? null
          : {
              ...ready,
              state: "not_found",
              client: null,
              message: "未找到官方客户端",
            },
      ),
    );
    await mountPage();
    const pick = screen.getByRole("button", { name: "选择路径" });
    await waitFor(() => expect(pick).toBeEnabled());
    expect(
      screen.getByRole("button", { name: "启动中文 ChatGPT" }),
    ).toBeDisabled();
    fireEvent.click(pick);
    await waitFor(() => expect(pick).toBeEnabled());
    expect(native.invoke).not.toHaveBeenCalledWith(
      "set_chinese_chatgpt_path",
      expect.anything(),
    );
    expect(screen.getByText(/安装官方 Codex 或 ChatGPT/)).toBeInTheDocument();
  });

  it("passes selected and reset paths to native validation and can launch the original client", async () => {
    native.invoke.mockImplementation((command) =>
      Promise.resolve(
        command === "pick_chinese_chatgpt_path"
          ? "C:\\Apps\\ChatGPT.exe"
          : ready,
      ),
    );
    await mountPage();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "选择路径" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "选择路径" }));
    await waitFor(() =>
      expect(native.invoke).toHaveBeenCalledWith("set_chinese_chatgpt_path", {
        path: "C:\\Apps\\ChatGPT.exe",
      }),
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "重置路径" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "重置路径" }));
    await waitFor(() =>
      expect(native.invoke).toHaveBeenCalledWith("set_chinese_chatgpt_path", {
        path: null,
      }),
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "原版启动" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "原版启动" }));
    await waitFor(() =>
      expect(native.invoke).toHaveBeenCalledWith("open_chatgpt_client"),
    );
  });

  it.each([
    ["restart-required", "请确认重启"],
    ["operation-in-progress", "正在进行"],
    ["client-already-running", "自行关闭"],
  ])("shows an actionable blocked result for %s", async (reason, hint) => {
    native.invoke.mockResolvedValue({
      ...ready,
      state: "blocked",
      reason,
      message: "客户端操作受阻",
    });
    await mountPage();
    await screen.findByText(new RegExp(hint));
    expect(screen.getByRole("status")).not.toHaveTextContent("中文已启用");
  });

  it("displays timeout details and allows retry after a failed command", async () => {
    native.invoke.mockImplementation((command) =>
      command === "scan_chinese_chatgpt"
        ? Promise.reject(new Error("客户端操作超时"))
        : Promise.resolve(ready),
    );
    await mountPage();
    const scan = screen.getByRole("button", { name: "重新扫描" });
    await waitFor(() => expect(scan).toBeEnabled());
    fireEvent.click(scan);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "客户端操作超时",
    );
    expect(scan).toBeEnabled();
  });

  it("shows the recovered support notice and QR with logs and without an empty WeChat row", async () => {
    await mountPage();
    fireEvent.click(screen.getByRole("button", { name: "使用帮助" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent("ChatGPT/中转站/国产模型接入，联系客服。");
    expect(
      within(dialog).getByRole("img", { name: "使用帮助客服二维码" }),
    ).toHaveAttribute("src", expect.stringContaining("support-qr.png"));
    expect(within(dialog).queryByText("客服微信")).not.toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "打开日志" }));
    await waitFor(() =>
      expect(native.invoke).toHaveBeenCalledWith("open_app_log_folder"),
    );
  });
});
