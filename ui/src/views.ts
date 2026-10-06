export type ViewId = "dashboard" | "inbox" | "library" | "plan" | "rules" | "settings";

export const VIEWS: { id: ViewId; label: string; key: string }[] = [
  { id: "dashboard", label: "Dashboard", key: "1" },
  { id: "inbox", label: "Inbox", key: "2" },
  { id: "library", label: "Library", key: "3" },
  { id: "plan", label: "Plan", key: "4" },
  { id: "rules", label: "Rules", key: "5" },
  { id: "settings", label: "Settings", key: "6" },
];
