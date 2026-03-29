import type { Preferences } from "sdk";
import { BaseService } from "../core/Service";
import { createEffect } from "solid-js";
import { logger } from "../../logger";

const log = logger.for("api/preferences");

const DEFAULT_PREFERENCES: Preferences = {
	frontend: {
		desktop_notifs: "yes",
		push_notifs: "yes",
		tts_notifs: "no",
		message_style: "cozy",
	},
	notifs: {
		messages: "Nothing",
		threads: "Nothing",
		reactions: "Dms",
		tts: "Nothing",
	},
	privacy: {
		friends: {
			pause_until: null,
			allow_everyone: true,
			allow_mutual_friend: true,
			allow_mutual_room: true,
		},
		dms: true,
		rpc: true,
		exif: true,
	},
};

// TODO: store preferences in db instead of localstorage
function loadSavedPreferences(): Preferences | null {
	const c = localStorage.getItem("preferences");
	if (!c) return null;
	try {
		return JSON.parse(c);
	} catch {
		return null;
	}
}

export class PreferencesService extends BaseService<Preferences> {
	protected cacheName = "preferences";
	public _loaded = false; // TEMP: public so that store can edit it

	constructor(store: import("../core/Store").RootStore) {
		super(store);
		this.cache.set("@self", DEFAULT_PREFERENCES);
	}

	getKey(_item: Preferences): string {
		return "@self";
	}

	/** fetch preferences via http */
	async fetch(_id: string): Promise<Preferences> {
		const data = await this.retryWithBackoff<Preferences>(() =>
			this.client.http.GET("/api/v1/preferences")
		);
		return data;
	}

	/** set preferences via http */
	async setPreferences(preferences: Preferences): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.PUT("/api/v1/preferences", { body: preferences })
		);
	}

	/** reactively read permissions */
	useRead() {
		return this.read();
	}

	read() {
		return this.cache.get("@self") ?? DEFAULT_PREFERENCES;
	}

	/** set permissions, updating localstorage and via http if necessary */
	put(prefs: Preferences) {
		localStorage.setItem("preferences", JSON.stringify(prefs));
		this.cache.set("@self", prefs);

		if (this._loaded) {
			this.setPreferences(prefs).catch((e) => {
				log.error("Failed to sync preferences to server", e);
			});
		}
	}
}
