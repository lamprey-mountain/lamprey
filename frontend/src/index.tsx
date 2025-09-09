/// <reference lib="dom" />

/* @refresh reload */
import "./styles/index.scss";
import { render } from "solid-js/web";
import App from "./App.tsx";

// @ts-ignore
const gitCommit = __VITE_GIT_COMMIT__;

if ("serviceWorker" in navigator) {
	try {
		console.log("[sw:host] registering service worker");
		navigator.serviceWorker?.register("/sw.js", { scope: "/" }).then(() => {
			localStorage.setItem("swCommit", gitCommit);
		});

		const existingSwCommit = localStorage.getItem("swCommit");
		if (existingSwCommit && existingSwCommit !== gitCommit) {
			console.log("[sw:host] updating service worker");
			navigator.serviceWorker.getRegistration().then((sw) => {
				if (!sw) return;
				sw.update();
			});
		}
	} catch (error) {
		console.error(error);
	}
}

render(() => <App />, document.getElementById("mount") as HTMLElement);
