import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { SessionWorkspace } from "../src/features/matters/DemoWorkspace";
import { apiGet } from "../src/lib/api";
import type { Role } from "../src/features/auth/useDemoSession";

vi.mock("../src/lib/api", async () => {
  const actual =
    await vi.importActual<typeof import("../src/lib/api")>("../src/lib/api");
  return { ...actual, apiGet: vi.fn() };
});
vi.mock("../src/features/documents/MatterDetails", () => ({
  MatterDetails: ({ id }: { id: string }) => (
    <section aria-label="Loaded matter details">{id}</section>
  ),
}));

let container: HTMLDivElement;
let root: Root;
let role: Role;
const base = {
  title: "Fictional document review",
  client_label: "Demo Client",
  service_category: "property",
  status: "collecting_documents",
  assigned_to: "Alex Demo",
  required_documents: 2,
  accepted_documents: 0,
};
const shared = { ...base, id: "shared-matter", reference: "MF-001" };
const reviewerOnly = {
  ...base,
  id: "reviewer-only-matter",
  reference: "MF-002",
  title: "Other staff matter",
};

beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  role = "staff";
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  vi.mocked(apiGet).mockImplementation(async (path) => {
    if (path === "/matters")
      return (role === "staff" ? [shared] : [shared, reviewerOnly]) as never;
    if (path === "/team")
      return [
        { display_name: "Alex Demo", role: "staff", active_matters: 1 },
      ] as never;
    throw new Error("Unexpected API request: " + path);
  });
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
  vi.clearAllMocks();
});
async function render(nextRole: Role, workspace = "workspace-one") {
  role = nextRole;
  await act(async () =>
    root.render(
      <SessionWorkspace
        key={workspace}
        token="demo-token"
        onExpired={() => {
          throw new Error("Unexpected expiry");
        }}
        session={{
          workspace_id: workspace,
          user_id: "user-" + nextRole,
          display_name: "Demo " + nextRole,
          role: nextRole,
          expires_at: "2099-01-01T00:00:00Z",
        }}
      />,
    ),
  );
}
async function open(reference: string) {
  const button = container.querySelector<HTMLButtonElement>(
    'button[aria-label="Open ' + reference + '"]',
  );
  expect(button).not.toBeNull();
  await act(async () => button!.click());
}
function details() {
  return container.querySelector('section[aria-label="Loaded matter details"]');
}

test("keeps the selected assigned matter through staff and reviewer switches", async () => {
  await render("staff");
  await open("MF-001");
  expect(details()?.textContent).toBe("shared-matter");
  await render("reviewer");
  expect(details()?.textContent).toBe("shared-matter");
  await render("staff");
  expect(details()?.textContent).toBe("shared-matter");
});
test("hides an inaccessible selection and restores it only in an authorized role", async () => {
  await render("reviewer");
  await open("MF-002");
  expect(details()?.textContent).toBe("reviewer-only-matter");
  await render("staff");
  expect(details()).toBeNull();
  expect(container.textContent).not.toContain("Other staff matter");
  expect(container.textContent).toContain("not available in this role");
  await render("reviewer");
  expect(details()?.textContent).toBe("reviewer-only-matter");
});
test("clears selection when a new workspace starts", async () => {
  await render("staff");
  await open("MF-001");
  await render("staff", "workspace-two");
  expect(details()).toBeNull();
  expect(container.textContent).toContain("A closer look");
});

function mockPagedMatters() {
  const rows = Array.from({ length: 12 }, (_, index) => ({
    ...base,
    id: "matter-" + (index + 1),
    reference: "MF-" + String(index + 1).padStart(3, "0"),
  }));
  vi.mocked(apiGet).mockImplementation(async (path) => {
    if (path === "/matters")
      return (
        role === "reviewer" ? rows : rows.filter((_, index) => index % 2 === 0)
      ) as never;
    if (path === "/team") return [] as never;
    throw new Error("Unexpected API request: " + path);
  });
}
async function clickPage(direction: "Next" | "Previous") {
  const button = container.querySelector<HTMLButtonElement>(
    'button[aria-label="' + direction + ' matter page"]',
  );
  expect(button).not.toBeNull();
  await act(async () => button!.click());
}
test("paginates the register, keeps detail selection, and resets paging for size and search changes", async () => {
  mockPagedMatters();
  await render("reviewer");
  expect(
    container.querySelectorAll('button[aria-label^="Open MF-"]'),
  ).toHaveLength(5);
  expect(container.textContent).toContain("Showing 1-5 of 12 matters");
  await clickPage("Next");
  await open("MF-008");
  expect(details()?.textContent).toBe("matter-8");
  await clickPage("Next");
  expect(container.textContent).toContain("Showing 11-12 of 12 matters");
  expect(details()?.textContent).toBe("matter-8");
  expect(
    container.querySelector<HTMLButtonElement>(
      'button[aria-label="Next matter page"]',
    )?.disabled,
  ).toBe(true);
  const size = container.querySelector<HTMLSelectElement>(
    'select[aria-label="Matters per page"]',
  )!;
  await act(async () => {
    size.value = "10";
    size.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(
    container.querySelectorAll('button[aria-label^="Open MF-"]'),
  ).toHaveLength(10);
  expect(container.textContent).toContain("Page 1 of 2");
  await clickPage("Next");
  const search = container.querySelector<HTMLInputElement>(
    'input[type="search"]',
  )!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )!.set!.call(search, "MF-012");
    search.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(
    container.querySelectorAll('button[aria-label^="Open MF-"]'),
  ).toHaveLength(1);
  expect(container.textContent).toContain("Showing 1-1 of 1 matters");
  expect(container.textContent).toContain("Page 1 of 1");
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )!.set!.call(search, "no-matching-reference");
    search.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(
    container.querySelectorAll('button[aria-label^="Open MF-"]'),
  ).toHaveLength(0);
  expect(container.textContent).toContain("No matters match your search.");
  expect(
    container.querySelector<HTMLButtonElement>(
      'button[aria-label="Previous matter page"]',
    )?.disabled,
  ).toBe(true);
  expect(
    container.querySelector<HTMLButtonElement>(
      'button[aria-label="Next matter page"]',
    )?.disabled,
  ).toBe(true);
});
test("opens the correct page for a preserved selection after a role switch", async () => {
  mockPagedMatters();
  await render("reviewer");
  await clickPage("Next");
  await clickPage("Next");
  await open("MF-011");
  await render("staff");
  expect(details()?.textContent).toBe("matter-11");
  expect(container.textContent).toContain("Page 2 of 2");
  expect(
    container.querySelector('button[aria-label="Open MF-011"]'),
  ).not.toBeNull();
  await render("reviewer");
  expect(details()?.textContent).toBe("matter-11");
  expect(container.textContent).toContain("Page 3 of 3");
});
