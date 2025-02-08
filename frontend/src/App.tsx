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
import { UserSettings } from "./UserSettings.tsx";
import { MessageMenu } from "./menu/Message.tsx";
import { RoomMenu } from "./menu/Room.tsx";
import { ThreadMenu } from "./menu/Thread.tsx";
import { getModal } from "./modal/mod.tsx";
import { ClientRectObject, ReferenceElement, shift } from "@floating-ui/dom";
import { Debug } from "./Debug.tsx";
import { RoomMemberMenu } from "./menu/RoomMember.tsx";

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
	const ctx: ChatCtx = {
		client,
		data,
		dispatch: () => {
			throw new Error("oh no!");
		},

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
		if (e.key === "Escape") dispatch({ do: "modal.close" });
	};

	const handleMouseMove = (e: MouseEvent) => {
		// TEMP: disable because spammy events
		// dispatch({ do: "window.mouse_move", e });
	};

	// TODO: refactor
	const handleContextMenu = (e: MouseEvent) => {
		const targetEl = e.target as HTMLElement;
		const menuEl = targetEl.closest(".has-menu") as HTMLElement | null;
		const mediaEl = targetEl.closest("a:not(.has-menu), img, video, audio") as
			| HTMLElement
			| null;
		if (!menuEl) return;
		if (mediaEl && targetEl.contains(mediaEl)) return;

		// TODO: refactor?
		const {
			messageId: message_id,
			roomId: room_id,
			threadId: thread_id,
			userId: user_id,
		} = menuEl.dataset;
		let menu: Partial<Menu> | null = null;

		if (message_id) {
			const message = api.messages.cache.get(message_id);
			if (!message) return;
			const thread_id = message.thread_id;
			const version_id = message.version_id;
			menu = {
				type: "message",
				thread_id,
				message_id,
				version_id,
			};
		}

		if (thread_id) {
			menu = {
				type: "thread",
				thread_id,
			};
		}

		if (room_id) {
			menu = {
				type: "room",
				room_id,
			};
		}

		// TODO: member_thread
		if (user_id && thread_id) {
			const thread = api.threads.cache.get(thread_id);
			if (!thread) return;
			menu = {
				type: "member_room",
				room_id: thread.room_id,
				user_id,
			};
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

const Title = (props: { title: string }) => {
	createEffect(() => document.title = props.title);
	return undefined;
};

const RouteHome = () => {
	return (
		<>
			<Title title="Home" />
			<ChatNav />
			<Home />
		</>
	);
};

function RouteSettings(p: RouteSectionProps) {
	const api = useApi();
	const user = () => api.users.cache.get("@self");
	return (
		<>
			<Title title={user() ? "Settings" : "loading..."} />
			<Show when={user()}>
				<UserSettings user={user()!} page={p.params.page} />
			</Show>
		</>
	);
}

function RouteRoom(p: RouteSectionProps) {
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	return (
		<>
			<Title title={room() ? room()!.name : "loading..."} />
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
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	const title = () => room() ? `${room()!.name} settings` : "loading...";
	return (
		<>
			<Title title={title()} />
			<ChatNav />
			<Show when={room()}>
				<RoomSettings room={room()!} page={p.params.page} />
			</Show>
		</>
	);
}

function RouteThread(p: RouteSectionProps) {
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const room = api.rooms.fetch(() => thread()?.room_id!);

	return (
		<>
			<Show when={room()} fallback={<Title title="loading..." />}>
				<Title title={`${thread()!.name} - ${room()!.name}`} />
			</Show>
			<ChatNav />
			<Show when={room()}>
				<ChatMain room={room()!} thread={thread()!} />
			</Show>
		</>
	);
}

function RouteNotFound() {
	return (
		<div style="padding:8px">
			not found
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
