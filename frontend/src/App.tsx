import { Component, createEffect, from, onCleanup } from "solid-js";
import { ChatCtx, chatctx, Data, defaultData } from "./context.ts";
import { createStore } from "solid-js/store";
import { Main } from "./Main.tsx";
import { createDispatcher } from "./dispatch/mod.ts";
import { createClient, MessageReady, MessageSync } from "sdk";
import { createApi } from "./api.tsx";
import { createEmitter } from "@solid-primitives/event-bus";
import { ReactiveMap } from "@solid-primitives/map";

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
	const ctx: ChatCtx = {
		client,
		data,
		dispatch: () => {
			throw new Error("oh no!");
		},

		thread_anchor: new ReactiveMap(),
	};
	const dispatch = createDispatcher(ctx, api, update);
	ctx.dispatch = dispatch;

	onCleanup(() => client.stop());

	const handleClick = () => {
		dispatch({ do: "menu", menu: null });
	};

	const handleKeypress = (e: KeyboardEvent) => {
		if (e.key === "Escape") dispatch({ do: "modal.close" });
	};

	const handleMouseMove = (e: MouseEvent) => {
		// TEMP: disable because spammy events
		// dispatch({ do: "window.mouse_move", e });
	};

	globalThis.addEventListener("click", handleClick);
	globalThis.addEventListener("keydown", handleKeypress);
	globalThis.addEventListener("mousemove", handleMouseMove);
	onCleanup(() => {
		globalThis.removeEventListener("click", handleClick);
		globalThis.removeEventListener("keydown", handleKeypress);
		globalThis.removeEventListener("mousemove", handleMouseMove);
	});

	// TEMP: debugging
	(globalThis as any).ctx = ctx;
	(globalThis as any).client = client;

	const TOKEN = localStorage.getItem("token")!;
	if (TOKEN) {
		client.start(TOKEN);
	} else {
		queueMicrotask(() => {
			ctx.dispatch({ do: "server.init_session" });
		});
	}

	events.on("sync", (msg) => {
		ctx.dispatch({
			do: "server",
			msg,
		});
	});

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
