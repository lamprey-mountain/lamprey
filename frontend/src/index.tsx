/// <reference lib="dom" />

/* @refresh reload */
import "./styles/index.scss";
import { render } from "solid-js/web";
import App from "./App.tsx";

if ("serviceWorker" in navigator) {
	try {
		console.log("registering service worker");
		navigator.serviceWorker?.register("/sw.js", { scope: "/" });
	} catch (error) {
		console.error(error);
	}
}

render(() => <App />, document.getElementById("mount") as HTMLElement);
