import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { ApiStatus } from "../src/features/health/ApiStatus";
import { DemoEntry } from "../src/features/auth/DemoEntry";
import { apiGet } from "../src/lib/api";

vi.mock("../src/lib/api", () => ({ apiGet: vi.fn() }));
let root: Root;
let container: HTMLDivElement;
const onEnter = vi.fn(async () => {});
const healthy = {
  status: "ready",
  service: "matterflow-api",
  version: "0.1.0",
};

beforeEach(() => {
  vi.useFakeTimers();
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
  vi.clearAllMocks();
  vi.useRealTimers();
});
async function render() {
  await act(async () =>
    root.render(
      <ApiStatus>
        {(ready) => <DemoEntry busy={false} ready={ready} onEnter={onEnter} />}
      </ApiStatus>,
    ),
  );
}

test("retries a sleeping database automatically and enables entry only when ready", async () => {
  vi.mocked(apiGet)
    .mockRejectedValueOnce(new Error("503"))
    .mockResolvedValue(healthy);
  await render();
  expect(container.querySelector("button")?.disabled).toBe(true);
  await act(async () =>
    container
      .querySelector("form")!
      .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })),
  );
  expect(onEnter).not.toHaveBeenCalled();
  await act(async () => vi.advanceTimersByTimeAsync(3_000));
  expect(apiGet).toHaveBeenLastCalledWith("/ready", expect.any(AbortSignal));
  expect(container.querySelector("button")?.disabled).toBe(false);
  expect(container.textContent).toContain("Demo service is ready");
  const calls = vi.mocked(apiGet).mock.calls.length;
  await act(async () => vi.advanceTimersByTimeAsync(90_000));
  expect(apiGet).toHaveBeenCalledTimes(calls);
  expect(container.textContent).not.toContain("still getting ready");
});

test("keeps reconnecting through prolonged failures without requesting manual retry", async () => {
  vi.mocked(apiGet).mockRejectedValue(new Error("offline"));
  await render();
  await act(async () => vi.advanceTimersByTimeAsync(135_000));
  expect(container.textContent).toContain("reconnecting automatically");
  expect(container.textContent).not.toContain("Please retry");
  expect(container.querySelector("button")?.disabled).toBe(true);
  vi.mocked(apiGet).mockResolvedValue(healthy);
  await act(async () => vi.advanceTimersByTimeAsync(15_000));
  expect(container.querySelector("button")?.disabled).toBe(false);
});

test("aborts an outstanding request on unmount without scheduling more checks", async () => {
  let resolve!: (value: typeof healthy) => void;
  vi.mocked(apiGet).mockReturnValue(
    new Promise((done) => {
      resolve = done;
    }),
  );
  await render();
  const signal = vi.mocked(apiGet).mock.calls[0][1]!;
  await act(async () => root.render(null));
  expect(signal.aborted).toBe(true);
  await act(async () => {
    resolve(healthy);
    await vi.advanceTimersByTimeAsync(150_000);
  });
  expect(apiGet).toHaveBeenCalledTimes(1);
});
