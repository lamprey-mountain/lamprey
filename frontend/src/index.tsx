/// <reference lib="dom" />

/* @refresh reload */
import "./styles/index.scss";
import { render } from "solid-js/web";
import App from "./App.tsx";

// @ts-ignore
const gitCommit = __VITE_GIT_COMMIT__;

(async () => {
	if (!("serviceWorker" in navigator)) return;

	try {
		console.log("[sw:host] registering service worker");

		const registration = await navigator.serviceWorker.register("/sw.js", {
			scope: "/",
			type: "module",
		});

		const existingSwCommit = localStorage.getItem("swCommit");

		if (existingSwCommit && existingSwCommit !== gitCommit) {
			console.log("[sw:host] new version detected, updating service worker");
			await registration.update();
			console.log("[sw:host] updated");
		} else {
			console.log("[sw:host] registered");
		}

		localStorage.setItem("swCommit", gitCommit);
	} catch (error) {
		console.error("[sw:host] registration failed", error);
	}
})();

render(() => <App />, document.getElementById("mount") as HTMLElement);
