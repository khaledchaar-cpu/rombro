import Panel from "../components/Panel";
import RetroArchPanel from "../components/RetroArchPanel";

/** Everything RetroArch: export of playlists, BIOS files and cores; later the launcher (v2). */
export default function RetroArch() {
  return (
    <div class="grid">
      <div class="wide">
        <RetroArchPanel />
      </div>
      <Panel title="Launcher" class="wide">
        <p class="dim">
          Coming in v2: start games straight from RomBro, play time, favorites and a gamepad mode.
        </p>
      </Panel>
    </div>
  );
}
