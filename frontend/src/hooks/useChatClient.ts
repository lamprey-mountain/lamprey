import { createEffect, createSignal, from, onCleanup } from "solid-js";
import { createStore } from "solid-js/store";
import { createClient, type UserConfig } from "sdk";
import { createEmitter } from "@solid-primitives/event-bus";
import { ReactiveMap } from "@solid-primitives/map";
import { createResource } from "solid-js";
import * as i18n from "@solid-primitives/i18n";
import type en from "../i18n/en.tsx";
import { createApi } from "../api.tsx";
import { createDispatcher } from "../dispatch/mod.ts";
import { useMouseTracking } from "./useMouseTracking.ts";
import { SlashCommands } from "../slash-commands.ts";
import { registerDefaultSlashCommands } from "../default-slash-commands.ts";
import { useLocation } from "@solidjs/router";
import type { ChatCtx, Data, Events, MediaCtx, Menu } from "../context.ts";
import type { AutocompleteState, UserViewData } from "../context.ts";
import type { ThreadsViewData } from "../context.ts";
import type { Config } from "../config.tsx";

function loadSavedUserConfig(): UserConfig | null {
	const c = localStorage.getItem("user_config");
	if (!c) return null;
	return JSON.parse(c);
}

const DEFAULT_USER_CONFIG: UserConfig = {
	frontend: {
		desktop_notifs: "yes",
		push_notifs: "yes",
		tts_notifs: "no",
		message_style: "cozy",
	},
	notifs: {
		messages: "Watching",
		mentions: "Notify",
		threads: "Watching",
		room_public: "Watching",
		room_private: "Watching",
		room_dm: "Watching",
	},
};

export function useChatClient(config: Config) {
	const events = createEmitter<Events>();
	const client = createClient({
		apiUrl: config.api_url,
		token: localStorage.getItem("token") || undefined,
		onSync(msg, raw) {
			console.log("recv", msg, raw);
			events.emit("sync", [msg, raw]);
		},
		onReady(msg) {
			events.emit("ready", msg);
		},
	});

	const [userConfig, setUserConfig] = createSignal<UserConfig>(
		loadSavedUserConfig() ?? DEFAULT_USER_CONFIG,
	);
	const api = createApi(client, events, { userConfig, setUserConfig });

	const cs = from(client.state);
	createEffect(() => {
		console.log("client state", cs());
	});

	const [data, update] = createStore<Data>({
		cursor: {
			pos: [],
			vel: 0,
			preview: null,
		},
	});

	type Lang = "en";
	const [lang, _setLang] = createSignal<Lang>("en");
	const [dict] = createResource(lang, async (lang) => {
		const m = await import(`../i18n/${lang}.tsx`);
		return i18n.flatten(m.default as typeof en);
	});

	const [currentMedia, setCurrentMedia] = createSignal<MediaCtx | null>(null);
	const [menu, setMenu] = createSignal<Menu | null>(null);
	const [popout, setPopout] = createSignal({});
	const [autocomplete, setAutocomplete] = createSignal<AutocompleteState>(null);
	const [userView, setUserView] = createSignal<UserViewData | null>(null);
	const [threadsView, setThreadsView] = createSignal<ThreadsViewData | null>(
		null,
	);

	const slashCommands = new SlashCommands();
	registerDefaultSlashCommands(slashCommands);

	let userConfigLoaded = false;

	(async () => {
		const data = await api.users.getConfig();
		if (data) setUserConfig(data as UserConfig);
		userConfigLoaded = true;
	})();

	createEffect(() => {
		const config = userConfig();
		if (!userConfigLoaded || !config) return;
		localStorage.setItem("user_config", JSON.stringify(config));
		api.users.setConfig(config);
	});

	const [recentChannels, setRecentChannels] = createSignal([] as string[]);

	const ctx: ChatCtx = {
		client,
		data,
		dispatch: (() => {
			throw new Error("Dispatch not initialized");
		}) as ChatCtx["dispatch"],

		t: i18n.translator(() => dict()),
		events,
		menu,
		setMenu,
		popout,
		setPopout,
		autocomplete,
		setAutocomplete,
		userView,
		setUserView,
		threadsView,
		setThreadsView,
		uploads: new ReactiveMap(),
		recentChannels,
		setRecentChannels,
		currentMedia,
		setCurrentMedia,
		userConfig,
		setUserConfig,
		scrollToChatList: (pos: number) => {
			console.log("scrollToChatList called with position:", pos);
		},
		slashCommands,
		channel_contexts: new ReactiveMap(),
		room_contexts: new ReactiveMap(),
		document_contexts: new ReactiveMap(),
	};

	createEffect(() => {
		const loc = useLocation();
		const path = loc.pathname.match(/^\/(channel)\/([^/]+)/);
		if (!path) return;
		ctx.setRecentChannels((s) =>
			[path[2], ...s.filter((i) => i !== path[2])].slice(0, 11)
		);
	});

	const dispatch = createDispatcher(ctx, api, update);
	ctx.dispatch = dispatch;

	useMouseTracking(update);

	onCleanup(() => client.stop());

	createEffect(() => {
		client.opts.apiUrl = config.api_url;
		const TOKEN = localStorage.getItem("token");
		if (TOKEN) {
			client.start(TOKEN);
		} else {
			ctx.dispatch({ do: "server.init_session" });
		}
	});

	if (!client.opts.token) {
		queueMicrotask(() => {
			ctx.dispatch({ do: "server.init_session" });
		});
	}

	api.ctx = ctx;

	return { client, api, ctx, userConfig, setUserConfig };
}
