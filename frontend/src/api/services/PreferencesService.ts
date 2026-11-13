import type { Preferences, PreferencesChannel, PreferencesRoom } from "sdk";
import { logger } from "@/utils/logger";
import { BaseService } from "../core/Service";

const log = logger.for("api/preferences");

// NOTE: mirrors the rust defaults in crate-common/src/v1/types/preferences
// (and notifications/preferences.rs). keep in sync when adding fields.
export const DEFAULT_PREFERENCES: Preferences = {
	frontend: {
		room_sidebar: [],

		underline_links: false,
		show_send_button: false,
		message_style: "cozy",
		show_compact_avatars: false,
		message_spacing: 8,
		chat_font_scale: 100,
		app_scale: 100,
		saturation: 100,
		reduced_motion: false,
		reduced_motion_sync: true,
		autoplay_emoji: true,
		autoplay_gifs: true,
		theme: "auto",

		preview_attachments: true,
		preview_attachments_descriptions: false,
		link_previews: true,
		typing_indicators: true,
		spoilers: "click",
		threads_sidebar_text: true,
		threads_sidebar_document: true,
		threads_sidebar_forum: true,

		date_format: "auto",
		time_format: "auto",

		desktop_notifs: false,
		push_notifs: false,
		tts_notifs: false,

		input_device: null,
		output_device: null,
		video_device: null,
		mic_volume: 50,
		speaker_volume: 75,
		voice_echo_cancellation: false,
		voice_noise_suppression: false,
		voice_input_mode: "vad",
		voice_activity_threshold: 30,
		voice_activity_timeout: 1000,

		calendar_day_starts_at: 0,

		favorite_emoji: [],
	},
	notifs: {
		mute: null,
		messages: "Mentions",
		replies: "Watching",
		threads: "Inbox",
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
// NOTE: when this is implemented, make sure to clear preferences on log out
function _loadSavedPreferences(): Preferences | null {
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
	private _syncTimeout: ReturnType<typeof setTimeout> | null = null;

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
			this.client.http.GET("/api/v1/preferences"),
		);
		return data;
	}

	/** set preferences via http */
	async setPreferences(preferences: Preferences): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.PUT("/api/v1/preferences", { body: preferences }),
		);
	}

	/** set room notification/privacy preferences via http */
	async setRoomPreferences(
		room_id: string,
		preferences: PreferencesRoom,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.PUT("/api/v1/preferences/room/{room_id}", {
				params: { path: { room_id } },
				body: preferences,
			} as any),
		);
	}

	/** set channel (thread) notification preferences via http */
	async setChannelPreferences(
		channel_id: string,
		preferences: PreferencesChannel,
	): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.PUT("/api/v1/preferences/channel/{channel_id}", {
				params: { path: { channel_id } },
				body: preferences,
			} as any),
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

		if (!this._loaded) return;

		// debounce so that dragging a slider doesn't flood the server
		if (this._syncTimeout) clearTimeout(this._syncTimeout);
		this._syncTimeout = setTimeout(() => {
			this._syncTimeout = null;
			this.setPreferences(this.read()).catch((e) => {
				log.error("Failed to sync preferences to server", e);
			});
		}, 500);
	}
}
