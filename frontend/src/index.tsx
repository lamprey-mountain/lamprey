/* @refresh reload */
import "./index.scss";
import { render } from "solid-js/web";
import App from "./App.tsx";

render(() => <App />, document.getElementById("mount") as HTMLElement);
