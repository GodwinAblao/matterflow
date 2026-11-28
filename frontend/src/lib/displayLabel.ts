const statusLabels: Record<string, string> = {
  intake: "New",
  collecting_documents: "Waiting for documents",
  under_review: "Being reviewed",
  ready: "Ready",
  closed: "Closed",
  missing: "Not received",
  received: "Received",
  needs_correction: "Needs changes",
  accepted: "Approved",
};

export function displayLabel(value: string) {
  return statusLabels[value] ?? value.replaceAll("_", " ");
}
