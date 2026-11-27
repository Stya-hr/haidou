import { getCurrentWindow } from "@tauri-apps/api/window";
import { render } from "solid-js/web";
import App from "./App";
import Float from "./Float";
import Overlay from "./Overlay";
import "./styles.css";

const label = getCurrentWindow().label;
render(
  () => (label === "overlay" ? <Overlay /> : label === "float" ? <Float /> : <App />),
  document.getElementById("root")!,
);
