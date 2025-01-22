import { Component, Show, batch as solidBatch } from "solid-js";
import { createEffect, createSignal, onCleanup } from "solid-js";
import { Client } from "sdk";
import { ChatCtx, chatctx, Data, useCtx } from "./context.ts";
import { createStore, produce } from "solid-js/store";
import { InviteT, MemberT, MessageT, RoleT } from "./types.ts";
import { Main } from "./Main.tsx";
import { createDispatcher, createWebsocketHandler } from "./dispatch.ts";
import { createReconnectingWS } from "@solid-primitives/websocket";
import { Router, Route, useLocation } from "@solidjs/router";
// import { PGlite } from "@electric-sql/pglite";
// global.PGlite = PGlite;

// const TOKEN = "0a11b93f-ff19-4c56-9bd2-d25bede776de";
const BASE_URL = localStorage.getItem("base_url") ?? "https://chat.celery.eu.org";
const TOKEN = localStorage.getItem("token")!;

const SLICE_LEN = 100;
const PAGINATE_LEN = 30;

const App: Component = () => {
	// const [hash, setHash] = createSignal(location.hash.slice(1));
	const [title, setTitle] = createSignal(document.title);

	const ws = createReconnectingWS(`${BASE_URL}/api/v1/sync`);
	onCleanup(() => ws.close());
	// const state = createWSState(ws);
	ws.addEventListener("message", (e) => {
		handleMessage(JSON.parse(e.data));
	});
	ws.addEventListener("open", (e) => {
		console.log("opened");
		ws.send(JSON.stringify({ type: "Hello", token: TOKEN }));
	});
	ws.addEventListener("error", (e) => {
		console.error(e);
	});

	const [data, update] = createStore<Data>({
		rooms: {},
		room_members: {},
		room_roles: {},
		threads: {},
		messages: {},
		timelines: {},
		slices: {},
		invites: {},
		users: {},
		thread_state: {},
		modals: [],
		user: null,
		menu: null,
	});

	const client = new Client(TOKEN, BASE_URL);

	(async () => {
		const data = await client.http("GET", `/api/v1/room?dir=f&limit=100`);
		for (const room of data.items) {
			update("rooms", room.id, room);
		}
	})();

	const ctx: ChatCtx = {
		client,
		data,
		dispatch: () => { throw new Error("oh no!"); }
	};
	const dispatch = createDispatcher(ctx, update);
	ctx.dispatch = dispatch;
	const handleMessage = createWebsocketHandler(ws, ctx);

	const handleClick = () => {
		dispatch({ do: "menu", menu: null });
	};

	// const handleHashChange = () => setHash(location.hash.slice(1));
	// globalThis.addEventListener("hashchange", handleHashChange);
	globalThis.addEventListener("click", handleClick);
	onCleanup(() => {
		// globalThis.removeEventListener("hashchange", handleHashChange);
		globalThis.removeEventListener("click", handleClick);
	});
	createEffect(() => document.title = title());
	// createEffect(() => location.hash = hash());
	// createEffect(() => setTitle(parts.get(hash())?.title ?? "unknown"));

	globalThis.addEventListener("keydown", e => {
		if (e.key === "Escape") dispatch({ do: "modal.close" });
	});

	return (
		<div id="root">
			<chatctx.Provider value={{ client, data, dispatch }}>
				<Main />
			</chatctx.Provider>
		</div>
	);
};

export default App;
