// The real Grace renderer (src/renderer), unchanged, with the Tauri Api instead of the preload one.
import "./index.css";
import { render } from "@solidjs/web";
import { App } from "../../../../src/renderer/App";
import { api } from "./api";

render(() => <App api={api} />, document.getElementById("root")!);
