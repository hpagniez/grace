import "./index.css";
import { render } from "@solidjs/web";
import { App } from "./renderer/App";

render(() => <App api={window.api} />, document.getElementById("root")!);
