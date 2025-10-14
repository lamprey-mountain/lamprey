import {
	type Component,
	createEffect,
	createMemo,
	For,
	from,
	onCleanup,
	type ParentProps,
	Show,
} from "solid-js";
import {
	type ChatCtx,
	chatctx,
	type Data,
	type Events,
	type MediaCtx,
	type Menu,
	useCtx,
	type UserViewData,
} from "./context.ts";
import { type Dispatcher } from "./dispatch/types.ts";
import { createStore } from "solid-js/store";
import { createDispatcher } from "./dispatch/mod.ts";
import { createClient, type UserConfig } from "sdk";
import { createApi, useApi } from "./api.tsx";
import { createEmitter } from "@solid-primitives/event-bus";
import { ReactiveMap } from "@solid-primitives/map";
import { createSignal } from "solid-js";
import { useMouseTracking } from "./hooks/useMouseTracking";
import { flags } from "./flags.ts";
import { Portal } from "solid-js/web";
import {
	Route,
	Router,
	type RouteSectionProps,
	useLocation,
	useNavigate,
} from "@solidjs/router";
import { UserSettings } from "./UserSettings.tsx";
import { getModal } from "./modal/mod.tsx";
import {
	autoUpdate,
	type ClientRectObject,
	computePosition,
	type ReferenceElement,
	shift,
} from "@floating-ui/dom";
import { Debug } from "./Debug.tsx";
import * as i18n from "@solid-primitives/i18n";
import { createResource } from "solid-js";
import type en from "./i18n/en.ts";
import {
	MessageMenu,
	RoomMenu,
	ThreadMenu,
	UserAdminMenu,
	UserMenu,
} from "./menu/mod.ts";
import {
	RoomNav,
	RouteAuthorize,
	RouteFeed,
	RouteHome,
	RouteInvite,
	RouteRoom,
	RouteRoomSettings,
	RouteThread,
	RouteThreadSettings,
	RouteUser,
} from "./routes.tsx";
import { RouteVerifyEmail } from "./VerifyEmail.tsx";
import { UserProfile } from "./UserProfile.tsx";
import { useContextMenu } from "./hooks/useContextMenu.ts";
import { Inbox } from "./Inbox.tsx";
import { ThreadNav } from "./Nav.tsx";
import { useVoice, VoiceProvider } from "./voice-provider.tsx";
import { Config, ConfigProvider, useConfig } from "./config.tsx";
import { UserView } from "./User.tsx";

const App: Component = () => {
	return (
		<Router root={Root1}>
			<Route path="/" component={RouteHome} />
			<Route path="/inbox" component={RouteInbox} />
			<Route path="/friends" component={RouteFriends} />
			<Route path="/settings/:page?" component={RouteSettings} />
			<Route path="/room/:room_id" component={RouteRoom} />
			<Route
				path="/room/:room_id/settings/:page?"
				component={RouteRoomSettings}
			/>
			<Route
				path="/thread/:thread_id/settings/:page?"
				component={RouteThreadSettings}
			/>
			<Route path="/thread/:thread_id" component={RouteThread} />
			<Route
				path="/thread/:thread_id/message/:message_id"
				component={RouteThread}
			/>
			<Route path="/debug" component={Debug} />
			<Route path="/feed" component={RouteFeed} />
			<Route path="/invite/:code" component={RouteInvite} />
			<Route path="/verify-email" component={RouteVerifyEmail} />
			<Route path="/user/:user_id" component={RouteUser} />
			<Route path="/authorize" component={RouteAuthorize} />
			<Route path="*404" component={RouteNotFound} />
		</Router>
	);
};

function loadSavedConfig(): Config | null {
	const c = localStorage.getItem("config");
	if (!c) return null;
	return JSON.parse(c);
}

const DEFAULT_USER_CONFIG: UserConfig = {
	frontend: {},
	notifs: {
		messages: "Watching",
		mentions: "Notify",
		threads: "Watching",
		room_public: "Watching",
		room_private: "Watching",
		room_dm: "Watching",
	},
};

function loadSavedUserConfig(): UserConfig | null {
	const c = localStorage.getItem("user_config");
	if (!c) return null;
	return JSON.parse(c);
}

// TODO: refactor bootstrap code?
export const Root1: Component = (props: ParentProps) => {
	const saved = loadSavedConfig();
	const [config, setConfig] = createSignal(saved);
	const [resolved, setResolved] = createSignal(false);
	console.log("[config] temporarily reusing existing config", saved);

	(async () => {
		if (localStorage.dontFetchConfig) return;

		const c: Config = await fetch("/config.json").then(
			(res) => res.json(),
			() => null,
		);
		console.log("[config] fetched new config", c);

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

		console.log("[config] resolved new config", c);
		localStorage.setItem("config", JSON.stringify(c));
		setConfig(c);
		setResolved(true);
	})();

	return (
		<Show when={config()}>
			<ConfigProvider value={config()!}>
				<Root2 resolved={resolved()}>{props.children}</Root2>
			</ConfigProvider>
		</Show>
	);
};

export const Root2 = (props: ParentProps<{ resolved: boolean }>) => {
	const config = useConfig();
	const events = createEmitter<Events>();
	const client = createClient({
		apiUrl: config.api_url,
		token: localStorage.getItem("token") || undefined,
		onSync(msg) {
			console.log("recv", msg);
			events.emit("sync", msg);
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
		modals: [],
		cursor: {
			pos: [],
			vel: 0,
			preview: null,
		},
	});

	type Lang = "en";
	const [lang, _setLang] = createSignal<Lang>("en");
	const [dict] = createResource(lang, async (lang) => {
		const m = await import(`./i18n/${lang}.ts`);
		return i18n.flatten(m.default as typeof en);
	});

	const [currentMedia, setCurrentMedia] = createSignal<MediaCtx | null>(null);
	const [menu, setMenu] = createSignal<Menu | null>(null);
	const [userView, setUserView] = createSignal<UserViewData | null>(null);
	const editingMessage = new ReactiveMap<
		string,
		{ message_id: string; selection?: "start" | "end" }
	>();

	let userConfigLoaded = false;

	(async () => {
		const { data } = await api.client.http.GET("/api/v1/config");
		if (data) setUserConfig(data as UserConfig);
		userConfigLoaded = true;
	})();

	createEffect(() => {
		const config = userConfig();
		if (!userConfigLoaded || !config) return;
		localStorage.setItem("user_config", JSON.stringify(config));
		api.client.http.PUT("/api/v1/config", {
			body: config,
		});
	});

	const [recentThreads, setRecentThreads] = createSignal([] as string[]);

	const ctx: ChatCtx = {
		client,
		data,
		dispatch: (() => {
			throw new Error("Dispatch not initialized");
		}) as Dispatcher,

		t: i18n.translator(() => dict()),
		events,
		menu,
		setMenu,
		userView,
		setUserView,
		thread_anchor: new ReactiveMap(),
		thread_attachments: new ReactiveMap(),
		thread_editor_state: new Map(),
		thread_highlight: new ReactiveMap(),
		thread_read_marker_id: new ReactiveMap(),
		thread_reply_id: new ReactiveMap(),
		thread_scroll_pos: new Map(),
		thread_search: new ReactiveMap(),
		thread_pinned_view: new ReactiveMap(),
		voice_chat_sidebar_open: new ReactiveMap(),
		uploads: new ReactiveMap(),
		thread_edit_drafts: new ReactiveMap(),
		thread_input_focus: new Map(),

		editingMessage,

		recentThreads,
		setRecentThreads,

		currentMedia,
		setCurrentMedia,

		userConfig,
		setUserConfig,

		scrollToChatList: (pos: number) => {
			// TODO: Implement actual scroll logic if needed
			console.log("scrollToChatList called with position:", pos);
		},

		selectMode: new ReactiveMap(),
		selectedMessages: new ReactiveMap(),
	};
	createEffect(() => {
		const loc = useLocation();
		const path = loc.pathname.match(/^\/thread\/([^/]+)/);
		if (!path) return;
		ctx.setRecentThreads((s) =>
			[path[1], ...s.filter((i) => i !== path[1])].slice(0, 11)
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

	// TEMP: debugging
	(globalThis as any).ctx = ctx;
	(globalThis as any).client = client;
	(globalThis as any).api = api;
	(globalThis as any).flags = flags;

	return (
		<api.Provider>
			<chatctx.Provider value={ctx}>
				<VoiceProvider>
					<Root3 setMenu={setMenu} dispatch={dispatch}>{props.children}</Root3>
				</VoiceProvider>
			</chatctx.Provider>
		</api.Provider>
	);
};

export const Root3 = (props: any) => {
	const ctx = useCtx();
	const [voice] = useVoice();

	const state = from(ctx.client.state);

	const { handleContextMenu } = useContextMenu(props.setMenu);

	const handleClick = (e: MouseEvent) => {
		props.setMenu(null);
		ctx.setUserView(null);
		if (!e.isTrusted) return;
		// const target = e.target as HTMLElement;
		// if (target.matches("a[download]")) {
		// 	const a = target as HTMLAnchorElement;
		// 	e.preventDefault();
		// 	// HACK: `download` doesn't work for cross origin links, so manually fetch and create a blob url
		// 	fetch(a.href).then((res) => res.blob()).then((res) => {
		// 		const url = URL.createObjectURL(res);
		// 		const fake = (
		// 			<a download={a.download} href={url} style="display:none"></a>
		// 		) as HTMLElement;
		// 		document.body.append(fake);
		// 		fake.click();
		// 		fake.remove();
		// 		URL.revokeObjectURL(url);
		// 	});
		// }
	};

	const handleKeypress = (e: KeyboardEvent) => {
		if (e.key === "Escape") {
			if (ctx.data.modals.length) {
				props.dispatch({ do: "modal.close" });
			}
		} else if (e.key === "k" && e.ctrlKey) {
			e.preventDefault();
			if (ctx.data.modals.length) {
				props.dispatch({ do: "modal.close" });
			} else {
				props.dispatch({ do: "modal.open", modal: { type: "palette" } });
			}
		} else if (e.key === "f" && e.ctrlKey) {
			e.preventDefault();
			const searchInput = document.querySelector(
				".search-input .ProseMirror",
			) as HTMLElement | null;
			searchInput?.focus();
		}
	};

	window.addEventListener("keydown", handleKeypress);
	onCleanup(() => {
		window.removeEventListener("keydown", handleKeypress);
	});

	return (
		<div
			id="root"
			classList={{
				"underline-links":
					ctx.userConfig().frontend["underline_links"] === "yes",
			}}
			onClick={handleClick}
			onContextMenu={handleContextMenu}
		>
			{props.children}
			<Portal mount={document.getElementById("overlay")!}>
				<Overlay />
			</Portal>
			<div style="visibility:hidden">
				<For each={[...voice.rtc?.streams.values() ?? []]}>
					{(stream) => {
						let audioRef!: HTMLAudioElement;
						createEffect(() => {
							console.log("listening to stream", stream);
							if (audioRef) audioRef.srcObject = stream.media;
						});
						createEffect(() => {
							const c = voice.userConfig.get(stream.user_id) ??
								{ mute: false, mute_video: false, volume: 100 };
							audioRef.volume = c.volume / 100;
						});
						return (
							<audio
								autoplay
								ref={audioRef!}
								muted={voice.deafened ||
									voice.userConfig.get(stream.user_id)?.mute === true}
							/>
						);
					}}
				</For>
			</div>
			<Show when={state() !== "ready"}>
				<div style="position:fixed;top:8px;left:8px;background:#111;padding:8px;border:solid #222 1px;">
					{state()}
				</div>
			</Show>
		</div>
	);
};

const Title = (props: { title?: string }) => {
	createEffect(() => document.title = props.title ?? "");
	return undefined;
};

function RouteSettings(p: RouteSectionProps) {
	const { t } = useCtx();
	const api = useApi();
	const user = () => api.users.cache.get("@self");
	createEffect(() => {
		console.log(user());
	});
	return (
		<>
			<Title title={user() ? t("page.settings_user") : t("loading")} />
			<Show when={user()}>
				<UserSettings user={user()!} page={p.params.page} />
			</Show>
		</>
	);
}

function RouteInbox() {
	return (
		<>
			<Title title="inbox" />
			<RoomNav />
			<ThreadNav />
			<Inbox />
		</>
	);
}

function RouteFriends() {
	const api = useApi();

	const [friends] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/user/{user_id}/friend",
			{ params: { path: { user_id: "@self" } } },
		);
		return data;
	});

	const sendRequest = () => {
		const target_id = prompt("target_id");
		if (!target_id) return;
		api.client.http.PUT("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id } },
		});
	};

	return (
		<>
			<Title title="friends" />
			<RoomNav />
			<div class="friends" style="padding:8px">
				todo!
				<ul>
					<li>foo</li>
					<li>bar</li>
					<li>baz</li>
				</ul>
				<pre>{JSON.stringify(friends())}</pre>
				<button onClick={sendRequest}>send request</button>
			</div>
		</>
	);
}

function RouteNotFound() {
	const { t } = useCtx();
	return (
		<div style="padding:8px">
			{t("not_found")}
		</div>
	);
}

function Overlay() {
	const ctx = useCtx();
	const api = useApi();

	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const [menuFloating, setMenuFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = menuParentRef();
		const floating = menuRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
					placement: "right-start",
				}).then(({ x, y, strategy }) => {
					setMenuFloating({ x, y, strategy });
				});
			},
		);
		onCleanup(cleanup);
	});

	const [userViewRef, setUserViewRef] = createSignal<HTMLElement>();
	const [userViewFloating, setUserViewFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = ctx.userView()?.ref;
		const floating = userViewRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
					placement: ctx.userView()?.source === "message"
						? "right-start"
						: "left-start",
				}).then(({ x, y, strategy }) => {
					setUserViewFloating({ x, y, strategy });
				});
			},
		);
		onCleanup(cleanup);
	});

	createEffect(() => {
		ctx.menu();

		setMenuParentRef({
			getBoundingClientRect(): ClientRectObject {
				const menu = ctx.menu();
				if (!menu) return {} as ClientRectObject;
				return {
					x: menu.x,
					y: menu.y,
					left: menu.x,
					top: menu.y,
					right: menu.x,
					bottom: menu.y,
					width: 0,
					height: 0,
				};
			},
		});
	});

	function getMenu(menu: Menu) {
		switch (menu.type) {
			case "room": {
				return <RoomMenu room_id={menu.room_id} />;
			}
			case "thread": {
				return <ThreadMenu thread_id={menu.thread_id} />;
			}
			case "message": {
				return (
					<MessageMenu
						thread_id={menu.thread_id}
						message_id={menu.message_id}
						version_id={menu.version_id}
					/>
				);
			}
			case "user": {
				return (
					<UserMenu
						user_id={menu.user_id}
						room_id={menu.room_id}
						thread_id={menu.thread_id}
						admin={menu.admin}
					/>
				);
			}
		}
	}

	const userViewData = createMemo(() => {
		const uv = ctx.userView();
		if (!uv) return null;
		const user = api.users.fetch(() => uv.user_id);
		const room_member = uv.room_id
			? api.room_members.fetch(() => uv.room_id!, () => uv.user_id)
			: () => null;
		const thread_member = uv.thread_id
			? api.thread_members.fetch(() => uv.thread_id!, () => uv.user_id)
			: () => null;
		return { user, room_member, thread_member };
	});

	return (
		<>
			<For each={ctx.data.modals}>
				{(modal) => getModal(modal)}
			</For>
			<Show when={ctx.menu()}>
				<div class="contextmenu">
					<div
						ref={setMenuRef}
						class="inner"
						style={{
							position: menuFloating.strategy,
							top: "0px",
							left: "0px",
							translate: `${menuFloating.x}px ${menuFloating.y}px`,
						}}
					>
						{getMenu(ctx.menu()!)}
					</div>
				</div>
			</Show>
			<Show when={userViewData()?.user()}>
				<div
					ref={setUserViewRef}
					style={{
						position: userViewFloating.strategy,
						top: "0px",
						left: "0px",
						translate: `${userViewFloating.x}px ${userViewFloating.y}px`,
						"z-index": 100,
					}}
				>
					<UserView
						user={userViewData()!.user()!}
						room_member={userViewData()!.room_member() ?? undefined}
						thread_member={userViewData()!.thread_member() ?? undefined}
					/>
				</div>
			</Show>
		</>
	);
}

export default App;
