/// <reference lib="dom" />

/* @refresh reload */
import "./styles/index.scss";
import { render } from "solid-js/web";
import App from "./App.tsx";
import { logger } from "./logger.ts";

const log = logger.for("sw");

// @ts-expect-error
const gitCommit = __VITE_GIT_COMMIT__;

(async () => {
	if (!("serviceWorker" in navigator)) return;

	try {
		log.info("host", "registering service worker", {});

		const registration = await navigator.serviceWorker.register("/sw.js", {
			scope: "/",
			type: "module",
		});

		const existingSwCommit = localStorage.getItem("swCommit");

		if (existingSwCommit && existingSwCommit !== gitCommit) {
			log.info("host", "new version detected, updating service worker", {});
			await registration.update();
			log.info("host", "updated", {});
		} else {
			log.info("host", "registered", {});
		}

		localStorage.setItem("swCommit", gitCommit);
	} catch (error) {
		log.error("host", "registration failed", { error });
	}
})();

render(() => <App />, document.getElementById("mount") as HTMLElement);
