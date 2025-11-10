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
	Popout,
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
	ChannelMenu,
	MessageMenu,
	RoomMenu,
	UserAdminMenu,
	UserMenu,
} from "./menu/mod.ts";
import {
	RouteAuthorize,
	RouteChannel,
	RouteChannelSettings,
	RouteFeed,
	RouteFriends,
	RouteHome,
	RouteInbox,
	RouteInvite,
	RouteRoom,
	RouteRoomSettings,
	RouteUser,
} from "./routes.tsx";
import { RoomNav } from "./RoomNav.tsx";
import { RouteVerifyEmail } from "./VerifyEmail.tsx";
import { UserProfile } from "./UserProfile.tsx";
import { useContextMenu } from "./hooks/useContextMenu.ts";
import { Inbox } from "./Inbox.tsx";
import { ChannelNav } from "./ChannelNav.tsx";
import { useVoice, VoiceProvider } from "./voice-provider.tsx";
import { Config, ConfigProvider, useConfig } from "./config.tsx";
import { UserView } from "./User.tsx";
import { EmojiPicker } from "./EmojiPicker.tsx";
import { Autocomplete } from "./Autocomplete.tsx";
import { AutocompleteState } from "./context.ts";
import { Resizable } from "./Resizable.tsx";
import { SlashCommands, SlashCommandsContext } from "./slash-commands.ts";
import { registerDefaultSlashCommands } from "./default-slash-commands.ts";
import { ChannelContext, createInitialChannelState } from "./channelctx.tsx";

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
				path="/channel/:channel_id/settings/:page?"
				component={RouteChannelSettings}
			/>
			<Route path="/channel/:channel_id" component={RouteChannel} />
			<Route
				path="/channel/:channel_id/message/:message_id"
				component={RouteChannel}
			/>
			<Route
				path="/thread/:channel_id/settings/:page?"
				component={RouteChannelSettings}
			/>
			<Route path="/thread/:channel_id" component={RouteChannel} />
			<Route
				path="/thread/:channel_id/message/:message_id"
				component={RouteChannel}
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
	const [popout, setPopout] = createSignal<Popout>({});
	const [autocomplete, setAutocomplete] = createSignal<AutocompleteState>(null);
	const [userView, setUserView] = createSignal<UserViewData | null>(null);
	const editingMessage = new ReactiveMap<
		string,
		{ message_id: string; selection?: "start" | "end" }
	>();

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
		}) as Dispatcher,

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
		channel_anchor: new ReactiveMap(),
		channel_attachments: new ReactiveMap(),
		channel_editor_state: new Map(),
		channel_highlight: new ReactiveMap(),
		channel_read_marker_id: new ReactiveMap(),
		channel_reply_id: new ReactiveMap(),
		channel_scroll_pos: new Map(),
		channel_search: new ReactiveMap(),
		channel_pinned_view: new ReactiveMap(),
		voice_chat_sidebar_open: new ReactiveMap(),
		uploads: new ReactiveMap(),
		channel_edit_drafts: new ReactiveMap(),
		channel_input_focus: new Map(),
		channel_slowmode_expire_at: new ReactiveMap(),

		editingMessage,

		recentChannels,
		setRecentChannels,

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

		slashCommands,
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

	// TEMP: debugging
	(globalThis as any).ctx = ctx;
	(globalThis as any).client = client;
	(globalThis as any).api = api;
	(globalThis as any).flags = flags;

	const channelContexts = new Map();

	const channelContext = () => {
		const channelId = "todo";
		const c = channelContexts.get(channelId)
		if (c) return c;

		const ctx = createInitialChannelState();
		channelContexts.set(channelId, ctx);
		return ctx;
	}

	return (
		<api.Provider>
			<chatctx.Provider value={ctx}>
				<ChannelContext.Provider value={channelContext()}>
					<VoiceProvider>
						<SlashCommandsContext.Provider value={slashCommands}>
							<Root3 setMenu={setMenu} dispatch={dispatch}>
								{props.children}
							</Root3>
						</SlashCommandsContext.Provider>
					</VoiceProvider>
				</ChannelContext.Provider>
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

	const [autocompleteRef, setAutocompleteRef] = createSignal<HTMLElement>();
	const [autocompleteFloating, setAutocompleteFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = ctx.autocomplete()?.ref;
		const floating = autocompleteRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					// middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
					placement: "top-start",
				}).then(({ x, y, strategy }) => {
					setAutocompleteFloating({ x, y, strategy });
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

	const [popoutRef, setPopoutRef] = createSignal<HTMLElement>();
	const [popoutFloating, setPopoutFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});

	createEffect(() => {
		const reference = ctx.popout()?.ref;
		const floating = popoutRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
					placement: ctx.popout()?.placement ?? "top",
				}).then(({ x, y, strategy }) => {
					setPopoutFloating({ x, y, strategy });
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
			case "channel": {
				return <ChannelMenu channel_id={menu.channel_id} />;
			}
			case "message": {
				return (
					<MessageMenu
						channel_id={menu.channel_id}
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
						channel_id={menu.channel_id}
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
		const thread_member = uv.channel_id
			? api.thread_members.fetch(() => uv.channel_id!, () => uv.user_id)
			: () => null;
		return { user, room_member, thread_member };
	});

	return (
		<>
			<For each={ctx.data.modals}>{(modal) => getModal(modal)}</For>
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
			<Show when={ctx.popout()?.id === "emoji" && ctx.popout().ref}>
				<div
					ref={setPopoutRef}
					style={{
						position: popoutFloating.strategy,
						top: "0px",
						left: "0px",
						translate: `${popoutFloating.x}px ${popoutFloating.y}px`,
						"z-index": 100,
					}}
				>
					<EmojiPicker {...ctx.popout().props} />
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
			<Show when={ctx.autocomplete()}>
				<div
					ref={setAutocompleteRef}
					style={{
						position: autocompleteFloating.strategy,
						top: "0px",
						left: "0px",
						translate:
							`${autocompleteFloating.x}px ${autocompleteFloating.y}px`,
						"z-index": 100,
					}}
				>
					<Autocomplete />
				</div>
			</Show>
		</>
	);
}

export default App;
