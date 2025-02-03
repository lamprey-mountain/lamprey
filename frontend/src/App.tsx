import { Component, createEffect, from, onCleanup } from "solid-js";
import { ChatCtx, chatctx, Data, defaultData, Menu } from "./context.ts";
import { createStore } from "solid-js/store";
import { Main } from "./Main.tsx";
import { createDispatcher } from "./dispatch/mod.ts";
import { createClient, MessageReady, MessageSync } from "sdk";
import { createApi } from "./api.tsx";
import { createEmitter } from "@solid-primitives/event-bus";
import { ReactiveMap } from "@solid-primitives/map";
import { createSignal } from "solid-js";

const BASE_URL = localStorage.getItem("base_url") ??
	"https://chat.celery.eu.org";

// TODO: refactor bootstrap code?
const App: Component = () => {
	const events = createEmitter<{
		sync: MessageSync;
		ready: MessageReady;
	}>();
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

		menu,
		thread_anchor: new ReactiveMap(),
	};
	const dispatch = createDispatcher(ctx, api, update);
	ctx.dispatch = dispatch;

	onCleanup(() => client.stop());

	const handleClick = () => {
		setMenu(null);
	};

	const handleKeypress = (e: KeyboardEvent) => {
		if (e.key === "Escape") dispatch({ do: "modal.close" });
	};

	const handleMouseMove = (e: MouseEvent) => {
		// TEMP: disable because spammy events
		// dispatch({ do: "window.mouse_move", e });
	};

	const handleContextMenu = (e: MouseEvent) => {
		const targetEl = e.target as HTMLElement;
		const menuEl = targetEl.closest(".has-menu") as HTMLElement | null;
		if (!menuEl) return;

		// TODO: refactor?
		// TODO: load targets instead of returning
		const { messageId, roomId, threadId } = menuEl.dataset;
		let menu: Partial<Menu> | null = null;

		if (messageId) {
			const threadId = api.messages.cache.get(messageId)?.thread_id;
			if (!threadId) return;
			menu = {
				type: "message",
				thread_id: threadId,
				message_id: messageId,
			};
		}

		if (threadId) {
			menu = {
				type: "thread",
				thread_id: threadId,
			};
		}

		if (roomId) {
			menu = {
				type: "room",
				room_id: roomId,
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
					<Main />
				</chatctx.Provider>
			</api.Provider>
		</div>
	);
};

export default App;
