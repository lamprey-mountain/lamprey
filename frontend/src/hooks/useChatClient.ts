import { createEmitter } from "@solid-primitives/event-bus";
import * as i18n from "@solid-primitives/i18n";
import { ReactiveMap } from "@solid-primitives/map";
import { useLocation } from "@solidjs/router";
import { type IDBPDatabase, openDB } from "idb";
import {
	createClient,
	type MessageEnvelope,
	type MessageReady,
	type MessageSync,
	type Preferences,
} from "sdk";
import {
	createEffect,
	createResource,
	createSignal,
	from,
	onCleanup,
} from "solid-js";
import { createStore } from "solid-js/store";
import { RootStore } from "@/api/core/Store.ts";
import type {
	ChatCtx,
	Data,
	MediaCtx,
	Popout,
	ThreadsViewData,
} from "@/app/context";
import type en from "@/i18n/en.tsx";
import { registerDefaultSlashCommands } from "@/lib/commands/builtin.ts";
import type { Config } from "@/lib/config";
import { flags } from "@/lib/flags";
import { type ApiDB, migrations } from "@/lib/sync/db";
import { colors, logger } from "@/utils/logger";
import { useMouseTracking } from "./useMouseTracking.ts";
import { SlashCommands } from "@/lib/commands/registry.ts";

export function useChatClient(config: Config) {
	const events = createEmitter<{
		sync: [MessageSync, MessageEnvelope];
		ready: MessageReady;
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
			if (data && typeof data === "object" && "type" in data) {
				sendLog("send 🢁", `sent op ${(data as { type: string }).type}`, data);
			}
		},
		onError(error) {
			syncLog.error("error", error.message, error);
		},
		onSync(msg, raw) {
			events.emit("sync", [msg, raw as MessageEnvelope]);
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

	const store = new RootStore(client, events, db);

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
	const [headerThreadsButtonRef, setHeaderThreadsButtonRef] =
		createSignal<HTMLElement | null>(null);

	const slashCommands = new SlashCommands();

	const [recentChannels, setRecentChannels] = createSignal([] as string[]);

	const [cursorStats, setCursorStats] = createSignal<
		import("@/app/context").CursorStats | null
	>(null);

	const ctx: ChatCtx = {
		client,
		data,
		dataUpdate: update,
		t: i18n.translator(() => dict()) as i18n.Translator<
			i18n.Flatten<typeof en>
		>,
		events: events as any,
		popout,
		setPopout,
		threadsView,
		setThreadsView,
		headerThreadsButtonRef,
		setHeaderThreadsButtonRef,
		uploads: new ReactiveMap(),
		recentChannels,
		setRecentChannels,
		currentMedia,
		setCurrentMedia,
		preferences: () => store.preferences.useRead(),
		setPreferences: ((p: Preferences) => store.preferences.put(p)) as any,
		scrollToChatList: (pos: number) => {
			console.log("scrollToChatList called with position:", pos);
		},
		cursorStats,
		setCursorStats,
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
			[path[2], ...s.filter((i) => i !== path[2])].slice(0, 11),
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
			store.initSession().catch((err) => {
				logger.for("auth").error("Failed to create temp session", err);
				alert("oh no :(\nsomething went VERY wrong");
			});
		}
	});

	(store as any).ctx = ctx;

	registerDefaultSlashCommands(ctx, store, slashCommands);

	return { client, ctx, store };
}
