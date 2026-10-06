export type ViewId = "dashboard" | "import" | "library" | "rules" | "settings";

export const VIEWS: { id: ViewId; label: string; key: string }[] = [
  { id: "dashboard", label: "Dashboard", key: "1" },
  { id: "import", label: "Import", key: "2" },
  { id: "library", label: "Library", key: "3" },
  { id: "rules", label: "Rules", key: "4" },
  { id: "settings", label: "Settings", key: "5" },
];
