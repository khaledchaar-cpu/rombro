import CorePicker from "../components/CorePicker";
import ManagedRetroArchPanel from "../components/ManagedRetroArchPanel";
import Panel from "../components/Panel";

/** Everything RetroArch: the managed RetroArch and the core per system. */
export default function RetroArch() {
  return (
    <div class="grid">
      <div class="wide">
        <ManagedRetroArchPanel />
      </div>
      <Panel title="Cores per system" class="wide">
        <CorePicker />
      </Panel>
    </div>
  );
}
