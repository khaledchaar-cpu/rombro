import { createSignal } from "solid-js";

export type SettingsSection = "rules" | "databases" | "retroarch" | "appearance";

export const SECTIONS: { id: SettingsSection; label: string }[] = [
  { id: "rules", label: "Rules" },
  { id: "databases", label: "Databases" },
  { id: "retroarch", label: "RetroArch" },
  { id: "appearance", label: "Appearance" },
];

/** Section shown in the Settings view (other views link into it). */
export const [settingsSection, setSettingsSection] = createSignal<SettingsSection>("rules");
