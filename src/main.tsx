import { getCurrentWindow } from "@tauri-apps/api/window";
import { render } from "solid-js/web";
import App from "./App";
import Overlay from "./Overlay";
import "./styles.css";

const isOverlay = getCurrentWindow().label === "overlay";
render(() => (isOverlay ? <Overlay /> : <App />), document.getElementById("root")!);
