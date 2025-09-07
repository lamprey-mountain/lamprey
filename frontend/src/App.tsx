import {
	type Component,
	createEffect,
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
} from "./context.ts";
import { type Dispatcher } from "./dispatch/types.ts";
import { createStore } from "solid-js/store";
import { createDispatcher } from "./dispatch/mod.ts";
import { createClient } from "sdk";
import { createApi, useApi } from "./api.tsx";
import { createEmitter } from "@solid-primitives/event-bus";
import { ReactiveMap } from "@solid-primitives/map";
import { createSignal } from "solid-js";
import { useMouseTracking } from "./hooks/useMouseTracking";
import { flags } from "./flags.ts";
import { Portal } from "solid-js/web";
import { Route, Router, type RouteSectionProps } from "@solidjs/router";
import { useFloating } from "solid-floating-ui";
import { UserSettings } from "./UserSettings.tsx";
import { getModal } from "./modal/mod.tsx";
import {
	type ClientRectObject,
	type ReferenceElement,
	shift,
} from "@floating-ui/dom";
import { Debug } from "./Debug.tsx";
import * as i18n from "@solid-primitives/i18n";
import { createResource } from "solid-js";
import type en from "./i18n/en.ts";
import {
	MessageMenu,
	RoomMemberMenu,
	RoomMenu,
	ThreadMemberMenu,
	ThreadMenu,
	UserMenu,
} from "./menu/mod.ts";
import {
	RoomNav,
	RouteAdminSettings,
	RouteFeed,
	RouteHome,
	RouteInvite,
	RouteRoom,
	RouteRoomSettings,
	RouteThread,
	RouteThreadSettings,
} from "./routes.tsx";
import { RouteVerifyEmail } from "./VerifyEmail.tsx";
import { UserProfile } from "./UserProfile.tsx";
import { useContextMenu } from "./hooks/useContextMenu.ts";
import { generateNickname } from "./nick.ts";
import { Inbox } from "./Inbox.tsx";
import { ThreadNav } from "./Nav.tsx";
import { useVoice, VoiceProvider } from "./voice.tsx";
import { VoiceTray } from "./Voice.tsx";

export const BASE_URL = localStorage.getItem("api_url") ??
	"https://chat.celery.eu.org";
export const CDN_URL = localStorage.getItem("cdn_url") ??
	"https://chat-cdn.celery.eu.org";

const App: Component = () => {
	return (
		<Router root={Root}>
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
			<Route path="/debug" component={Debug} />
			<Route path="/feed" component={RouteFeed} />
			<Route path="/invite/:code" component={RouteInvite} />
			<Route path="/verify-email" component={RouteVerifyEmail} />
			<Route path="/user/:user_id" component={UserProfile} />
			<Route path="/admin" component={RouteAdminSettings} />
			<Route path="*404" component={RouteNotFound} />
		</Router>
	);
};

// TODO: refactor bootstrap code?
export const Root: Component = (props: ParentProps) => {
	const events = createEmitter<Events>();
	const client = createClient({
		apiUrl: BASE_URL,
		onSync(msg) {
			console.log("recv", msg);
			events.emit("sync", msg);
		},
		onReady(msg) {
			events.emit("ready", msg);
		},
	});

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

	const ctx: ChatCtx = {
		client,
		data,
		dispatch: (() => {
			throw new Error("Dispatch not initialized");
		}) as Dispatcher,

		t: i18n.translator(() => dict()),
		events,
		menu,
		thread_anchor: new ReactiveMap(),
		thread_attachments: new ReactiveMap(),
		thread_editor_state: new Map(),
		thread_highlight: new ReactiveMap(),
		thread_read_marker_id: new ReactiveMap(),
		thread_reply_id: new ReactiveMap(),
		thread_scroll_pos: new Map(),
		uploads: new ReactiveMap(),

		currentMedia,
		setCurrentMedia,

		settings: new ReactiveMap(
			JSON.parse(localStorage.getItem("settings") ?? "[]"),
		),
		scrollToChatList: (pos: number) => {
			// TODO: Implement actual scroll logic if needed
			console.log("scrollToChatList called with position:", pos);
		},
	};

	const api = createApi(client, events);
	const dispatch = createDispatcher(ctx, api, update);
	ctx.dispatch = dispatch;

	useMouseTracking(update);

	onCleanup(() => client.stop());

	createEffect(() => {
		localStorage.setItem(
			"settings",
			JSON.stringify([...ctx.settings.entries()]),
		);
	});

	// TODO: sync settings to server
	// needs a new event to receive config updates
	// api.client.http.GET("/api/v1/user/{user_id}/config", {
	// 	params: {path: {user_id: "@self"}},
	// });

	// createEffect(() => {
	// 	api.client.http.PUT("/api/v1/user/{user_id}/config", {
	// 		params: {path: {user_id: "@self"}},
	// 		body: {

	// 			frontend: Object.fromEntries (ctx.settings.entries())
	// 		}
	// 	})
	// })

	// TEMP: debugging
	(globalThis as any).ctx = ctx;
	(globalThis as any).client = client;
	(globalThis as any).api = api;
	(globalThis as any).flags = flags;

	const TOKEN = localStorage.getItem("token")!;
	if (TOKEN) {
		client.start(TOKEN);
	} else {
		queueMicrotask(() => {
			ctx.dispatch({ do: "server.init_session" });
		});
	}

	return (
		<api.Provider>
			<chatctx.Provider value={ctx}>
				<VoiceProvider>
					<Root2 setMenu={setMenu} dispatch={dispatch}>{props.children}</Root2>
				</VoiceProvider>
			</chatctx.Provider>
		</api.Provider>
	);
};

export const Root2: Component = (props: any) => {
	const ctx = useCtx();
	const [voice] = useVoice();
	const api = useApi();
	const thread = () =>
		voice.threadId ? api.threads.fetch(() => voice.threadId!)() : null;

	const state = from(ctx.client.state);

	const { handleContextMenu } = useContextMenu(props.setMenu);

	const handleClick = (e: MouseEvent) => {
		props.setMenu(null);
		if (!e.isTrusted) return;
		const target = e.target as HTMLElement;
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
		}
	};

	return (
		<div
			id="root"
			classList={{
				"underline-links": ctx.settings.get("underline_links") === "yes",
			}}
			onClick={handleClick}
			onKeyDown={handleKeypress}
			onContextMenu={handleContextMenu}
		>
			<Show when={api.users.cache.get("@self")}>
				{props.children}
			</Show>
			<Portal mount={document.getElementById("overlay")!}>
				<Overlay />
			</Portal>
			<VoiceTray />
			<div style="visibility:hidden">
				<For each={[...voice.rtc?.streams.values() ?? []]}>
					{(stream) => {
						let audioRef!: HTMLAudioElement;
						createEffect(() => {
							console.log("listening to stream", stream);
							if (audioRef) audioRef.srcObject = stream.media;
						});
						return (
							<audio
								autoplay
								ref={audioRef!}
								muted={voice.deafened}
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
	console.log(ctx);

	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const menuFloating = useFloating(() => menuParentRef(), () => menuRef(), {
		middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
		placement: "right-start",
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
			case "member_room": {
				return <RoomMemberMenu room_id={menu.room_id} user_id={menu.user_id} />;
			}
			case "member_thread": {
				return (
					<ThreadMemberMenu thread_id={menu.thread_id} user_id={menu.user_id} />
				);
			}
			case "user": {
				return <UserMenu user_id={menu.user_id} />;
			}
		}
	}

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
							translate: `${menuFloating.x}px ${menuFloating.y}px`,
						}}
					>
						{getMenu(ctx.menu()!)}
					</div>
				</div>
			</Show>
		</>
	);
}

export default App;
