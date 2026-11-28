import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { AnalyticsDashboard } from "../src/features/analytics/AnalyticsDashboard";
import { apiGet, ApiError } from "../src/lib/api";
vi.mock("../src/lib/api", async () => ({
  ...(await vi.importActual("../src/lib/api")),
  apiGet: vi.fn(),
}));
const report = {
  as_of: "2026-11-28T06:00:00Z",
  local_date: "2026-11-28",
  metrics: {
    visible_matters: 3,
    active_matters: 2,
    required_documents: 9,
    accepted_documents: 6,
    completeness_percent: (100 * 6) / 9,
    outstanding_requirements: 3,
    overdue_follow_ups: 1,
    median_days_to_readiness: 2.5,
    readiness_cohort: 2,
  },
  matters: [
    {
      id: "m1",
      reference: "MF-001",
      title: '=1+1, "fictional"',
      status: "intake",
      assigned_to: "Alex",
      required_documents: 3,
      accepted_documents: 0,
      days_to_readiness: null,
    },
    {
      id: "m2",
      reference: "MF-002",
      title: "Ready fixture",
      status: "ready",
      assigned_to: "Sam",
      required_documents: 3,
      accepted_documents: 3,
      days_to_readiness: 2,
    },
    {
      id: "m3",
      reference: "MF-003",
      title: "Closed fixture",
      status: "closed",
      assigned_to: "Alex",
      required_documents: 3,
      accepted_documents: 3,
      days_to_readiness: 3,
    },
  ],
  requirements: [7, 14, 15].map((age, index) => ({
    id: "d" + index,
    matter_id: "m1",
    reference: "MF-001",
    title: "Requirement " + age,
    status: "missing",
    age_days: age,
  })),
  follow_ups: [
    {
      id: "f1",
      matter_id: "m1",
      reference: "MF-001",
      title: "Overdue fixture",
      due_on: "2026-11-27",
      assigned_to: "Alex",
      completed: false,
      matter_status: "intake",
      overdue: true,
    },
    {
      id: "f2",
      matter_id: "m2",
      reference: "MF-002",
      title: "Due today fixture",
      due_on: "2026-11-28",
      assigned_to: "Sam",
      completed: false,
      matter_status: "ready",
      overdue: false,
    },
    {
      id: "f3",
      matter_id: "m3",
      reference: "MF-003",
      title: "Closed task fixture",
      due_on: "2026-11-26",
      assigned_to: "Alex",
      completed: false,
      matter_status: "closed",
      overdue: false,
    },
  ],
};
let container: HTMLDivElement;
let root: Root;
const onOpen = vi.fn();
const onExpired = vi.fn();
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  vi.mocked(apiGet).mockResolvedValue(structuredClone(report));
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
  vi.restoreAllMocks();
  vi.clearAllMocks();
});
async function render() {
  await act(async () =>
    root.render(
      <AnalyticsDashboard
        token="test-token"
        onExpired={onExpired}
        onOpenMatter={onOpen}
      />,
    ),
  );
}
async function click(prefix: string) {
  const button = Array.from(container.querySelectorAll("button")).find((b) =>
    b.textContent?.startsWith(prefix),
  );
  expect(button).toBeDefined();
  await act(async () => button!.click());
}
function rows() {
  return container.querySelectorAll(".drilldown tbody tr");
}
function table() {
  return container.querySelector(".drilldown tbody")!.textContent;
}

test("metric drill-downs reconcile with active scope, median cohort and overdue dates", async () => {
  await render();
  expect(container.textContent).toContain("66.7%");
  expect(container.textContent).toContain("3 days");
  expect(rows()).toHaveLength(3);
  await click("Active matters");
  expect(rows()).toHaveLength(2);
  expect(table()).not.toContain("Closed fixture");
  await click("Median time");
  expect(rows()).toHaveLength(2);
  expect(table()).not.toContain("=1+1");
  expect(table()).toContain("Closed fixture");
  await click("Overdue follow-ups");
  expect(rows()).toHaveLength(1);
  expect(table()).toContain("Overdue fixture");
  expect(table()).not.toContain("Due today fixture");
  await click("All follow-ups");
  expect(rows()).toHaveLength(3);
});

test("stage, age and assignment charts filter records and open the correct matter", async () => {
  await render();
  await click("ready");
  expect(rows()).toHaveLength(1);
  expect(table()).toContain("Ready fixture");
  await click("0-7 days");
  expect(rows()).toHaveLength(1);
  expect(table()).toContain("Requirement 7");
  await click("8-14 days");
  expect(rows()).toHaveLength(1);
  expect(table()).toContain("Requirement 14");
  await click("15+ days");
  expect(rows()).toHaveLength(1);
  expect(table()).toContain("Requirement 15");
  await click("MF-001");
  expect(onOpen).toHaveBeenCalledWith("m1");
  await click("Alex");
  expect(rows()).toHaveLength(1);
  expect(table()).not.toContain("Closed fixture");
});

test("category changes reset drill-downs and display empty-denominator N/A values", async () => {
  await render();
  await click("Overdue follow-ups");
  vi.mocked(apiGet).mockResolvedValue({
    ...report,
    metrics: {
      ...report.metrics,
      visible_matters: 0,
      active_matters: 0,
      required_documents: 0,
      accepted_documents: 0,
      completeness_percent: null,
      outstanding_requirements: 0,
      overdue_follow_ups: 0,
      median_days_to_readiness: null,
      readiness_cohort: 0,
    },
    matters: [],
    requirements: [],
    follow_ups: [],
  });
  await act(async () => {
    const select = container.querySelector("select")!;
    select.value = "criminal";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(apiGet).toHaveBeenLastCalledWith(
    "/analytics?category=criminal",
    expect.any(AbortSignal),
    "test-token",
  );
  expect(container.textContent).toContain("All visible matters");
  expect(container.querySelectorAll(".metric-grid strong")[2].textContent).toBe(
    "N/A",
  );
  expect(container.textContent).toContain("No records match this selection.");
  expect(rows()).toHaveLength(0);
});

test("CSV exports the selected records and escapes spreadsheet formulas and quotes", async () => {
  const blobs: Blob[] = [];
  const originalCreate = URL.createObjectURL;
  const originalRevoke = URL.revokeObjectURL;
  URL.createObjectURL = vi.fn((blob: Blob | MediaSource) => {
    blobs.push(blob as Blob);
    return "blob:test";
  });
  URL.revokeObjectURL = vi.fn();
  const anchor = vi
    .spyOn(HTMLAnchorElement.prototype, "click")
    .mockImplementation(() => {});
  try {
    await render();
    await click("Export CSV");
    expect(anchor).toHaveBeenCalledOnce();
    const read = (blob: Blob) =>
      new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result as string);
        reader.onerror = reject;
        reader.readAsText(blob);
      });
    const all = await read(blobs[0]);
    expect(all).toContain(`"'=1+1, ""fictional"""`);
    await click("Overdue follow-ups");
    await click("Export CSV");
    const csv = await read(blobs[1]);
    expect(csv).toContain("Overdue fixture");
    expect(csv).not.toContain("Due today fixture");
    expect(csv.split("\r\n")).toHaveLength(2);
  } finally {
    URL.createObjectURL = originalCreate;
    URL.revokeObjectURL = originalRevoke;
  }
});

test("failed refresh keeps the previous snapshot and recovers; expired sessions are handled", async () => {
  await render();
  vi.mocked(apiGet).mockRejectedValueOnce(new Error("Temporary report error"));
  await click("Refresh report");
  expect(rows()).toHaveLength(3);
  expect(container.querySelector('[role="alert"]')?.textContent).toContain(
    "Temporary report error",
  );
  vi.mocked(apiGet).mockResolvedValue(report);
  await click("Refresh report");
  expect(container.querySelector('[role="alert"]')).toBeNull();
  vi.mocked(apiGet).mockRejectedValueOnce(new ApiError(401, "Expired"));
  await click("Refresh report");
  expect(onExpired).toHaveBeenCalledOnce();
});

test("readiness durations display whole days with singular units where appropriate", async () => {
  vi.mocked(apiGet).mockResolvedValue({
    ...report,
    metrics: { ...report.metrics, median_days_to_readiness: 5 },
    matters: [{ ...report.matters[1], days_to_readiness: 1 }],
  });
  await render();
  expect(container.textContent).toContain("5 days");
  expect(container.textContent).not.toContain("5.0 days");
  await click("Median time");
  expect(table()).toContain("1 day to first readiness");
  expect(table()).not.toContain("1.00 days");
});
