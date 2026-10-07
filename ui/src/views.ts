export type ViewId = "dashboard" | "import" | "library" | "settings";

export const VIEWS: { id: ViewId; label: string; key: string }[] = [
  { id: "dashboard", label: "Dashboard", key: "1" },
  { id: "import", label: "Import", key: "2" },
  { id: "library", label: "Library", key: "3" },
  { id: "settings", label: "Settings", key: "4" },
];
