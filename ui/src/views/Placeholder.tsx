import Panel from "../components/Panel";

export default function Placeholder(props: { title: string }) {
  return (
    <Panel title={props.title}>
      <p class="dim mono">// not implemented yet (M8)</p>
    </Panel>
  );
}
