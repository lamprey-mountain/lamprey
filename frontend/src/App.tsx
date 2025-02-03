import { Component, createEffect, from, onCleanup } from "solid-js";
import { ChatCtx, chatctx, Data, defaultData } from "./context.ts";
import { createStore } from "solid-js/store";
import { Main } from "./Main.tsx";
import { createDispatcher } from "./dispatch/mod.ts";
import { createClient, MessageReady, MessageSync } from "sdk";
import { ApiProvider, useApi } from "./api.tsx";
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

	return (
		<div id="root">
			<ApiProvider client={client} temp_events={events}>
				<App2 client={client} events={events} />
			</ApiProvider>
		</div>
	);
};

// HACK: this exists so the api context exists
const App2 = (props: any) => {
	console.log("API", useApi());

	const [data, update] = createStore<Data>(defaultData);

	const ctx: ChatCtx = {
		client: props.client,
		data,
		dispatch: () => {
			throw new Error("oh no!");
		},
		
		thread_anchor: new ReactiveMap(),
	};
	const dispatch = createDispatcher(ctx, useApi(), update);
	ctx.dispatch = dispatch;

	onCleanup(() => props.client.stop());

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
	(globalThis as any).client = props.client;

	const TOKEN = localStorage.getItem("token")!;
	if (TOKEN) {
		props.client.start(TOKEN);
	} else {
		queueMicrotask(() => {
			ctx.dispatch({ do: "server.init_session" });
		});
	}

	props.events.on("sync", (msg) => {
		ctx.dispatch({
			do: "server",
			msg,
		});
	});

	return (
		<chatctx.Provider value={ctx}>
			<Main />
		</chatctx.Provider>
	);
};

export default App;
