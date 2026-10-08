import ManagedRetroArchPanel from "../components/ManagedRetroArchPanel";
import RetroArchPanel from "../components/RetroArchPanel";

/** Everything RetroArch: the managed RetroArch (launcher base) and the export to an own RetroArch. */
export default function RetroArch() {
  return (
    <div class="grid">
      <div class="wide">
        <ManagedRetroArchPanel />
      </div>
      <div class="wide">
        <RetroArchPanel />
      </div>
    </div>
  );
}
