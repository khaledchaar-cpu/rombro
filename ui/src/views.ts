export type ViewId = "dashboard" | "import" | "library" | "stats" | "retroarch" | "rules" | "system";

/** Sidebar entries; `group: "foot"` sits at the bottom. `icon`: SVG path data (24×24, stroked). */
export const VIEWS: { id: ViewId; label: string; key: string; icon: string; group?: "foot" }[] = [
  { id: "dashboard", label: "Dashboard", key: "1", icon: "M3 3h8v8H3zM13 3h8v5h-8zM13 10h8v11h-8zM3 13h8v8H3z" },
  { id: "import", label: "Import", key: "2", icon: "M12 3v12M7 10l5 5 5-5M4 17v4h16v-4" },
  { id: "library", label: "Library", key: "3", icon: "M4 4h4v16H4zM10 4h4v16h-4zM16 5l4-1 3 15-4 1z" },
  { id: "stats", label: "Stats", key: "7", icon: "M4 20V10M10 20V4M16 20v-7M22 20H2" },
  { id: "retroarch", label: "RetroArch", key: "4", icon: "M6 9h12a4 4 0 0 1 4 4v1a4 4 0 0 1-7 2.6L14 15h-4l-1 1.6A4 4 0 0 1 2 14v-1a4 4 0 0 1 4-4zM7 11v4M5 13h4M16 12h.01M18 14h.01" },
  { id: "rules", label: "Rules", key: "5", icon: "M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0M14 4v4M8 10v4M16 16v4" },
  { id: "system", label: "System", key: "6", icon: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1", group: "foot" },
];
