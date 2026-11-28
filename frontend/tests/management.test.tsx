import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { beforeEach, afterEach, expect, test, vi } from "vitest";
import { MatterManagement } from "../src/features/matters/MatterManagement";
import { apiGet, apiRequest } from "../src/lib/api";
vi.mock("../src/lib/api", async () => ({
  ...(await vi.importActual("../src/lib/api")),
  apiGet: vi.fn(),
  apiRequest: vi.fn(),
}));
let root: Root;
let container: HTMLDivElement;
const saved = vi.fn(async () => {});
const deleted = vi.fn();
const matter = {
  id: "matter-13",
  reference: "MF-013",
  title: "Business document reviewe",
  client_label: "Demo Client",
  service_category: "business",
  assigned_staff_id: "staff-1",
  status: "intake",
  version: 2,
};
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  vi.mocked(apiGet).mockResolvedValue([
    { id: "staff-1", display_name: "Alex" },
  ]);
  vi.mocked(apiRequest).mockResolvedValue({});
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
  vi.clearAllMocks();
});
async function render(canDelete = true) {
  await act(async () =>
    root.render(
      <MatterManagement
        matter={matter}
        canDelete={canDelete}
        role="staff"
        token="test-token"
        busy={false}
        onBusy={() => {}}
        onSaved={saved}
        onDeleted={deleted}
        onExpired={() => {}}
      />,
    ),
  );
}
async function click(text: string) {
  await act(async () =>
    Array.from(container.querySelectorAll("button"))
      .find((b) => b.textContent === text)!
      .click(),
  );
}
async function input(el: HTMLInputElement, value: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )!.set!.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
async function submit() {
  await act(async () =>
    container
      .querySelector("form")!
      .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })),
  );
}
test("corrects a title while preserving the reference and sends the current record version", async () => {
  await render();
  await click("Edit matter");
  const title = container.querySelector("input")!;
  expect(title.value).toBe(matter.title);
  await input(title, "Business document review");
  await submit();
  expect(apiRequest).toHaveBeenCalledWith(
    "/matters/matter-13",
    expect.objectContaining({
      method: "PATCH",
      body: expect.objectContaining({
        title: "Business document review",
        expected_version: 2,
        assigned_staff_id: "staff-1",
      }),
    }),
  );
  expect(saved).toHaveBeenCalledOnce();
  expect(container.textContent).toContain("Matter updated.");
});
test("requires an exact reference before deleting and supports cancelling", async () => {
  await render();
  await click("Delete draft");
  expect(
    container.querySelector("button[type=submit]")?.hasAttribute("disabled"),
  ).toBe(true);
  await input(container.querySelector("input")!, "MF-WRONG");
  expect(
    container.querySelector("button[type=submit]")?.hasAttribute("disabled"),
  ).toBe(true);
  await click("Cancel");
  expect(apiRequest).not.toHaveBeenCalled();
  await click("Delete draft");
  await input(container.querySelector("input")!, "MF-013");
  await submit();
  expect(apiRequest).toHaveBeenCalledWith(
    "/matters/matter-13",
    expect.objectContaining({
      method: "DELETE",
      body: { expected_version: 2, reference: "MF-013" },
    }),
  );
  expect(deleted).toHaveBeenCalledOnce();
});
test("staff cannot archive and workflow matters have no delete control", async () => {
  await render(false);
  expect(container.textContent).not.toContain("Delete draft");
  expect(container.textContent).not.toContain("Archive matter");
});
