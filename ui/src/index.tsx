import { render } from "solid-js/web";
import "@fontsource/orbitron/600.css";
import "@fontsource/rajdhani/500.css";
import "@fontsource/rajdhani/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/layout.css";
import "./styles/views.css";
import App from "./App";

const root = document.getElementById("root");
if (root) render(() => <App />, root);
