import {
	Component,
	createEffect,
	For,
	from,
	onCleanup,
	ParentProps,
	Show,
} from "solid-js";
import {
	ChatCtx,
	chatctx,
	Data,
	defaultData,
	Events,
	Menu,
	useCtx,
} from "./context.ts";
import { createStore } from "solid-js/store";
import { createDispatcher } from "./dispatch/mod.ts";
import { createClient } from "sdk";
import { createApi, useApi } from "./api.tsx";
import { createEmitter } from "@solid-primitives/event-bus";
import { ReactiveMap } from "@solid-primitives/map";
import { createSignal } from "solid-js";
import { flags } from "./flags.ts";
import { Portal } from "solid-js/web";
import { Route, Router, RouteSectionProps } from "@solidjs/router";
import { useFloating } from "solid-floating-ui";
import { ChatMain } from "./Chat.tsx";
import { Home } from "./Home.tsx";
import { ChatNav } from "./Nav.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { RoomSettings } from "./RoomSettings.tsx";
import { ThreadSettings } from "./ThreadSettings.tsx";
import { UserSettings } from "./UserSettings.tsx";
import { getModal } from "./modal/mod.tsx";
import { ClientRectObject, ReferenceElement, shift } from "@floating-ui/dom";
import { Debug } from "./Debug.tsx";
import * as i18n from "@solid-primitives/i18n";
import { createResource } from "solid-js";
import type en from "./i18n/en.ts";
import { ThreadMembers } from "./Thread.tsx";
import {
	MessageMenu,
	RoomMemberMenu,
	RoomMenu,
	ThreadMemberMenu,
	ThreadMenu,
	UserMenu,
} from "./menu/mod.ts";

const BASE_URL = localStorage.getItem("base_url") ??
	"https://chat.celery.eu.org";

const App: Component = () => {
	return (
		<Router root={Root}>
			<Route path="/" component={RouteHome} />
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
			<Route path="*404" component={RouteNotFound} />
		</Router>
	);
};

// TODO: refactor bootstrap code?
export const Root: Component = (props: ParentProps) => {
	const events = createEmitter<Events>();
	const client = createClient({
		baseUrl: BASE_URL,
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

	const api = createApi(client, events);
	const [data, update] = createStore<Data>(defaultData);
	const [menu, setMenu] = createSignal<Menu | null>(null);

	type Lang = "en";
	const [lang, _setLang] = createSignal<Lang>("en");
	const [dict] = createResource(lang, async (lang) => {
		const m = await import(`./i18n/${lang}.ts`);
		return i18n.flatten(m.default as typeof en);
	});

	const ctx: ChatCtx = {
		client,
		data,
		dispatch: () => {
			throw new Error("oh no!");
		},

		t: i18n.translator(dict),
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
	};
	const dispatch = createDispatcher(ctx, api, update);
	ctx.dispatch = dispatch;

	onCleanup(() => client.stop());

	const handleClick = (e: MouseEvent) => {
		setMenu(null);
		if (!e.isTrusted) return;
		const target = e.target as HTMLElement;
		if (target.matches("a[download]")) {
			const a = target as HTMLAnchorElement;
			e.preventDefault();
			// HACK: `download` doesn't work for cross origin links, so manually fetch and create a blob url
			fetch(a.href).then((res) => res.blob()).then((res) => {
				const url = URL.createObjectURL(res);
				const fake = (
					<a download={a.download} href={url} style="display:none"></a>
				) as HTMLElement;
				document.body.append(fake);
				fake.click();
				fake.remove();
				URL.revokeObjectURL(url);
			});
		}
	};

	const handleKeypress = (e: KeyboardEvent) => {
		if (e.key === "Escape") {
			const thread_id = (document.querySelector(".chat") as HTMLElement)
				?.dataset.threadId;
			if (ctx.data.modals.length) {
				dispatch({ do: "modal.close" });
			} else if (thread_id) {
				// messages are approx. 20 px high, show 3 pages of messages
				const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

				ctx.thread_anchor.set(thread_id, {
					type: "backwards",
					limit: SLICE_LEN,
				});
				// version_id may be undefined
				ctx.dispatch({
					do: "thread.mark_read",
					thread_id: thread_id,
					delay: false,
					also_local: true,
					version_id: api.threads.cache.get(thread_id)?.last_version_id!,
				});
				// HACK: i need to make the update order less jank
				setTimeout(() => {
					const listEl = document.querySelector(".chat > .list") as HTMLElement;
					listEl.scrollTo(0, 99999999);
				});
			}
		}
	};

	const handleMouseMove = (e: MouseEvent) => {
		// TEMP: disable because spammy events
		// dispatch({ do: "window.mouse_move", e });
	};

	// TODO: refactor
	const handleContextMenu = (e: MouseEvent) => {
		const targetEl = e.target as HTMLElement;

		const menuEl = targetEl.closest(
			".menu-room, .menu-thread, .menu-message, .menu-user",
		) as HTMLElement | null;
		const mediaEl = targetEl.closest("a, img, video, audio") as
			| HTMLElement
			| null;
		if (!menuEl) return;
		if (mediaEl && targetEl !== menuEl) return;

		const getData = (key: string) => {
			const target = menuEl.closest(`[${key}]`) as HTMLElement | null;
			return target
				?.dataset[
					key.slice("data-".length).replace(
						/-([a-z])/g,
						(_, c) => c.toUpperCase(),
					)
				];
		};

		let menu: Partial<Menu> | null = null;
		const room_id = getData("data-room-id");
		const thread_id = getData("data-thread-id");
		const message_id = getData("data-message-id");
		const user_id = getData("data-user-id");

		if (menuEl.classList.contains("menu-room")) {
			if (!room_id) return;
			menu = {
				type: "room",
				room_id,
			};
		} else if (menuEl.classList.contains("menu-thread")) {
			if (!thread_id) return;
			menu = {
				type: "thread",
				thread_id,
			};
		} else if (menuEl.classList.contains("menu-message")) {
			const message = api.messages.cache.get(message_id!);
			if (!message) return;
			const thread_id = message.thread_id;
			const version_id = message.version_id;
			menu = {
				type: "message",
				thread_id,
				message_id,
				version_id,
			};
		} else if (menuEl.classList.contains("menu-user")) {
			if (!user_id) return;
			if (thread_id) {
				const thread = api.threads.cache.get(thread_id);
				if (!thread) return;
				menu = {
					type: "member_thread",
					thread_id: thread.id,
					user_id,
				};
			} else if (room_id) {
				const room = api.rooms.cache.get(room_id);
				if (!room) return;
				menu = {
					type: "member_room",
					room_id: room.id,
					user_id,
				};
			} else {
				menu = {
					type: "user",
					user_id,
				};
			}
		}

		if (menu) {
			e.preventDefault();
			setMenu({
				x: e.clientX,
				y: e.clientY,
				...menu,
			} as Menu);
		}
	};

	globalThis.addEventListener("click", handleClick);
	globalThis.addEventListener("keydown", handleKeypress);
	globalThis.addEventListener("mousemove", handleMouseMove);
	globalThis.addEventListener("contextmenu", handleContextMenu);
	onCleanup(() => {
		globalThis.removeEventListener("click", handleClick);
		globalThis.removeEventListener("keydown", handleKeypress);
		globalThis.removeEventListener("mousemove", handleMouseMove);
		globalThis.removeEventListener("contextmenu", handleContextMenu);
	});

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

	createEffect(() => {
		// FIXME: don't fetch all threads every time room cache changes
		// fine for now, but will be massively less efficient the more rooms/threads there are
		for (const room_id of api.rooms.cache.keys()) {
			api.threads.list(() => room_id);
		}
	});

	return (
		<div id="root">
			<api.Provider>
				<chatctx.Provider value={ctx}>
					{props.children}
					<Portal mount={document.getElementById("overlay")!}>
						<Overlay />
					</Portal>
				</chatctx.Provider>
			</api.Provider>
		</div>
	);
};

const Title = (props: { title?: string }) => {
	createEffect(() => document.title = props.title ?? "");
	return undefined;
};

const RouteHome = () => {
	const { t } = useCtx();
	return (
		<>
			<Title title={t("page.home")} />
			<ChatNav />
			<Home />
		</>
	);
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

function RouteRoom(p: RouteSectionProps) {
	const { t } = useCtx();
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	return (
		<>
			<Title title={room() ? room()!.name : t("loading")} />
			<ChatNav />
			<Show when={room()}>
				<RoomHome room={room()!} />
				<Show when={flags.has("room_member_list")}>
					<RoomMembers room={room()!} />
				</Show>
			</Show>
		</>
	);
}

function RouteRoomSettings(p: RouteSectionProps) {
	const { t } = useCtx();
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	const title = () =>
		room() ? t("page.settings_room", room()!.name) : t("loading");
	return (
		<>
			<Title title={title()} />
			<Show when={room()}>
				<RoomSettings room={room()!} page={p.params.page} />
			</Show>
		</>
	);
}

function RouteThreadSettings(p: RouteSectionProps) {
	const { t } = useCtx();
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const title = () =>
		thread() ? t("page.settings_thread", thread()!.name) : t("loading");
	return (
		<>
			<Title title={title()} />
			<ChatNav />
			<Show when={thread()}>
				<ThreadSettings thread={thread()!} page={p.params.page} />
			</Show>
		</>
	);
}

function RouteThread(p: RouteSectionProps) {
	const { t } = useCtx();
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const room = api.rooms.fetch(() => thread()?.room_id!);

	return (
		<>
			<Show when={room() && thread()} fallback={<Title title={t("loading")} />}>
				<Title title={`${thread()!.name} - ${room()!.name}`} />
			</Show>
			<ChatNav />
			<Show when={room() && thread()}>
				<ChatMain room={room()!} thread={thread()!} />
				<Show when={flags.has("thread_member_list")}>
					<ThreadMembers thread={thread()!} />
				</Show>
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
	console.log(ctx);

	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const menuFloating = useFloating(menuParentRef, menuRef, {
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
