import { createSignal } from "solid-js";
import type { Config } from "../config.tsx";
import { logger } from "../logger.ts";

const log = logger.for("config");

function loadSavedConfig(): Config | null {
	const c = localStorage.getItem("config");
	if (!c) return null;
	return JSON.parse(c);
}

async function fetchConfig(): Promise<Config> {
	const env = (globalThis as any).ENV;
	if (env) {
		log.info("using server defined env", env);
		return env;
	} else {
		const c: Config = await fetch("/config.json").then(
			(res) => res.json(),
			() => null,
		);
		log.info("using server defined env", c);
		return c;
	}
}

export function useAppConfig() {
	const saved = loadSavedConfig();
	const [config, setConfig] = createSignal<Config | null>(saved);
	const [resolved, setResolved] = createSignal(false);

	log.info("temporarily reusing existing config", saved);

	(async () => {
		if (localStorage.dontFetchConfig) return;

		const c = await fetchConfig();

		if (c.api_url && typeof c?.api_url !== "string") {
			throw new Error("config.api_url is not a string");
		}

		if (c.cdn_url && typeof c?.cdn_url !== "string") {
			throw new Error("config.cdn_url is not a string");
		}

		c.api_url ??= localStorage.getItem("api_url") ??
			"https://chat.celery.eu.org";
		c.cdn_url ??= localStorage.getItem("cdn_url") ??
			"https://chat-cdn.celery.eu.org";

		log.info("resolved new config", c);
		localStorage.setItem("config", JSON.stringify(c));
		setConfig(c);
		setResolved(true);
	})();

	return { config, resolved, setConfig, setResolved };
}
