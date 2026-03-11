import { createEffect, createSignal, from, onCleanup } from "solid-js";
import { createStore } from "solid-js/store";
import { createClient, type Preferences } from "sdk";
import { createEmitter } from "@solid-primitives/event-bus";
import { ReactiveMap } from "@solid-primitives/map";
import { createResource } from "solid-js";
import * as i18n from "@solid-primitives/i18n";
import type en from "../i18n/en.tsx";
import { createApi } from "../api.tsx";
import { useMouseTracking } from "./useMouseTracking.ts";
import { SlashCommands } from "../contexts/slash-commands";
import { registerDefaultSlashCommands } from "../default-slash-commands.ts";
import { useLocation } from "@solidjs/router";
import type { ChatCtx, Data, Events, MediaCtx } from "../context.ts";
import type { ThreadsViewData } from "../context.ts";
import type { Config } from "../config.tsx";
import { flags } from "../flags.ts";

function loadSavedPreferences(): Preferences | null {
	const c = localStorage.getItem("preferences");
	if (!c) return null;
	return JSON.parse(c);
}

const DEFAULT_PREFERENCES: Preferences = {
	frontend: {
		desktop_notifs: "yes",
		push_notifs: "yes",
		tts_notifs: "no",
		message_style: "cozy",
	},
	notifs: {
		messages: "Nothing",
		mentions: "Notify",
		threads: "Nothing",
		room_public: "Nothing",
		room_private: "Nothing",
		room_dm: "Nothing",
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

import { RootStore } from "../api/core/Store.ts";

export function useChatClient(config: Config) {
	const events = createEmitter<{
		sync: [import("sdk").MessageSync, import("sdk").MessageEnvelope];
		ready: import("sdk").MessageReady;
	}>();
	const useMsgpack = flags.has("msgpack");
	const client = createClient({
		apiUrl: config.api_url,
		token: localStorage.getItem("token") || undefined,
		format: useMsgpack ? "msgpack" : "json",
		onSync(msg, raw) {
			console.log("recv", msg, raw);
			events.emit("sync", [msg, raw as import("sdk").MessageEnvelope]);
		},
		onReady(msg) {
			events.emit("ready", msg);
		},
	});

	const [preferences, setPreferences] = createSignal<Preferences>(
		loadSavedPreferences() ?? DEFAULT_PREFERENCES,
	);
	const [serverPreferences, setServerPreferences] = createSignal<
		Preferences | null
	>(null);
	const store = new RootStore(
		client,
		events,
		preferences,
		setPreferences,
		setServerPreferences,
	);
	const api = createApi(client, events, { preferences, setPreferences, store });

	const cs = from(client.state);
	createEffect(() => {
		console.log("client state", cs());
	});

	const [data, update] = createStore<Data>({
		cursor: {
			pos: [],
			vel: 0,
		},
		channels: {},
	});

	type Lang = "en";
	const [lang, _setLang] = createSignal<Lang>("en");
	const [dict] = createResource(lang, async (lang) => {
		const m = await import(`../i18n/${lang}.tsx`);
		return i18n.flatten(m.default as typeof en);
	});

	const [currentMedia, setCurrentMedia] = createSignal<MediaCtx | null>(null);
	const [popout, setPopout] = createSignal<Popout | null>(null);
	const [threadsView, setThreadsView] = createSignal<ThreadsViewData | null>(
		null,
	);

	const slashCommands = new SlashCommands();
	registerDefaultSlashCommands(slashCommands);

	createEffect(() => {
		const config = preferences();
		if (!api.preferencesLoaded() || !config) return;
		if (api.session()?.status !== "Authorized") return;

		localStorage.setItem("preferences", JSON.stringify(config));

		// Only send to server if preferences differ from what server has
		const serverPrefs = serverPreferences();
		if (
			!serverPrefs || JSON.stringify(config) !== JSON.stringify(serverPrefs)
		) {
			api.users.setPreferences(config);
		}
	});

	const [recentChannels, setRecentChannels] = createSignal([] as string[]);

	const [cursorStats, setCursorStats] = createSignal<
		import("../context.ts").CursorStats | null
	>(null);

	const ctx: ChatCtx = {
		client,
		data,
		dataUpdate: update,

		t: i18n.translator(() => dict()) as i18n.Translator<
			i18n.Flatten<typeof en>
		>,
		events,
		popout,
		setPopout,
		threadsView,
		setThreadsView,
		uploads: new ReactiveMap(),
		recentChannels,
		setRecentChannels,
		currentMedia,
		setCurrentMedia,
		preferences,
		setPreferences,
		scrollToChatList: (pos: number) => {
			console.log("scrollToChatList called with position:", pos);
		},
		cursorStats,
		setCursorStats,
		slashCommands,
		channel_contexts: new ReactiveMap(),
		room_contexts: new ReactiveMap(),
		document_contexts: new ReactiveMap(),
		api,
	};

	createEffect(() => {
		const loc = useLocation();
		const path = loc.pathname.match(/^\/(channel)\/([^/]+)/);
		if (!path) return;
		ctx.setRecentChannels((s) =>
			[path[2], ...s.filter((i) => i !== path[2])].slice(0, 11)
		);
	});

	useMouseTracking(update);

	onCleanup(() => client.stop());

	createEffect(() => {
		client.opts.apiUrl = config.api_url;
		const TOKEN = localStorage.getItem("token");
		if (TOKEN) {
			client.start(TOKEN);
		} else {
			api.tempCreateSession();
		}
	});

	if (!client.opts.token) {
		queueMicrotask(() => {
			api.tempCreateSession();
		});
	}

	api.ctx = ctx;

	return { client, api, ctx, preferences, setPreferences, store };
}
