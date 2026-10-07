import { createSignal } from "solid-js";

export type SystemSection = "databases" | "appearance";

export const SECTIONS: { id: SystemSection; label: string }[] = [
  { id: "databases", label: "Databases" },
  { id: "appearance", label: "Appearance" },
];

/** Section shown in the System view (other views link into it). */
export const [systemSection, setSystemSection] = createSignal<SystemSection>("databases");
