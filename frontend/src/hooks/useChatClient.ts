import { createEffect, createSignal, from, onCleanup } from "solid-js";
import { createStore } from "solid-js/store";
import { createClient, type Preferences, Room } from "sdk";
import { createEmitter } from "@solid-primitives/event-bus";
import { ReactiveMap } from "@solid-primitives/map";
import { createResource } from "solid-js";
import * as i18n from "@solid-primitives/i18n";
import type en from "../i18n/en.tsx";
import { useMouseTracking } from "./useMouseTracking.ts";
import { SlashCommands } from "../contexts/slash-commands";
import { registerDefaultSlashCommands } from "../default-slash-commands.ts";
import { useLocation } from "@solidjs/router";
import type { ChatCtx, Data, Events, MediaCtx, Popout } from "../context.ts";
import type { ThreadsViewData } from "../context.ts";
import type { Config } from "../config.tsx";
import { flags } from "../flags.ts";
import { RootStore } from "@/api/core/Store.ts";
import { colors, logger } from "../logger.ts";
import { DBSchema, type IDBPDatabase, openDB } from "idb";
import { ApiDB, migrations } from "../db.ts";

export function useChatClient(config: Config) {
	const events = createEmitter<{
		sync: [import("sdk").MessageSync, import("sdk").MessageEnvelope];
		ready: import("sdk").MessageReady;
	}>();
	const useMsgpack = flags.has("msgpack");
	const useDeflate = flags.has("sync_deflate");
	const recvLog = logger.for("sync").create("debug", colors.blue);
	const sendLog = logger.for("sync").create("debug", colors.teal);
	const syncLog = logger.for("cs");
	const client = createClient({
		apiUrl: config.api_url,
		token: localStorage.getItem("token") || undefined,
		format: useMsgpack ? "msgpack" : "json",
		compress: useDeflate ? "deflate" : undefined,
		onMessage(raw) {
			const op = raw.op === "Sync" ? `Sync (${raw.data.type})` : raw.op;
			recvLog("recv 🢃", `got op ${op}`, raw);
		},
		onSend(data) {
			sendLog("send 🢁", `sent op ${data.type}`, data);
		},
		onError(error) {
			syncLog.error("error", error.message, error);
		},
		onSync(msg, raw) {
			events.emit("sync", [msg, raw as import("sdk").MessageEnvelope]);
		},
		onReady(msg) {
			events.emit("ready", msg);
		},
	});

	const [db, setDb] = createSignal<IDBPDatabase<ApiDB> | undefined>();

	(async () => {
		try {
			const database = await openDB<ApiDB>("api", migrations.length, {
				upgrade(db, oldVersion, _newVersion, txn) {
					const log = logger.for("idb");
					for (let i = oldVersion; i < migrations.length; i++) {
						const m = migrations[i];
						m.migrate(db, txn);
						log.info(m.description, undefined, "migrate");
					}
				},
			});
			setDb(database);
			logger.for("idb").debug("IndexedDB opened successfully");
		} catch (e) {
			logger.for("idb").error("Failed to initialize IndexedDB", e);
		}
	})();

	const store = new RootStore(
		client,
		events,
		() => db() as IDBPDatabase<unknown> | undefined,
	);

	const cs = from(client.state);
	createEffect(() => {
		syncLog.debug("client state", cs() ?? "unknown", null);
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
		preferences: () => store.preferences.useRead(),
		setPreferences: (p: Preferences) => store.preferences.put(p),
		scrollToChatList: (pos: number) => {
			console.log("scrollToChatList called with position:", pos);
		},
		cursorStats,
		setCursorStats,
		slashCommands,
		channel_contexts: new ReactiveMap(),
		room_contexts: new ReactiveMap(),
		document_contexts: new ReactiveMap(),
		store,
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
			store.tempCreateSession();
		}
	});

	if (!client.opts.token) {
		queueMicrotask(() => {
			store.tempCreateSession();
		});
	}

	store.ctx = ctx;

	return { client, ctx, store };
}
