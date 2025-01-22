import { Component, onCleanup } from "solid-js";
import { ChatCtx, chatctx, Data } from "./context.ts";
import { createStore } from "solid-js/store";
import { Main } from "./Main.tsx";
import { createDispatcher } from "./dispatch.ts";
import { createClient } from "sdk";

const BASE_URL = localStorage.getItem("base_url") ??
	"https://chat.celery.eu.org";
const TOKEN = localStorage.getItem("token")!;

const App: Component = () => {
	const client = createClient({
		baseUrl: BASE_URL,
		token: TOKEN,
		onMessage(msg) {
			console.log("recv", msg);
			ctx.dispatch({
				do: "server",
				msg,
			});
		},
	});

	client.start();
	onCleanup(() => client.stop());

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

	(async () => {
		const { data, error } = await client.http.GET("/api/v1/room", {
			params: {
				query: {
					dir: "f",
					limit: 100,
				},
			},
		});
		if (error) {
			console.error(error);
			return;
		}
		for (const room of data.items) {
			update("rooms", room.id, room);
		}
	})();

	const ctx: ChatCtx = {
		client,
		data,
		dispatch: () => {
			throw new Error("oh no!");
		},
	};
	const dispatch = createDispatcher(ctx, update);
	ctx.dispatch = dispatch;

	const handleClick = () => {
		dispatch({ do: "menu", menu: null });
	};

	const handleKeypress = (e: KeyboardEvent) => {
		if (e.key === "Escape") dispatch({ do: "modal.close" });
	};

	globalThis.addEventListener("click", handleClick);
	globalThis.addEventListener("keydown", handleKeypress);
	onCleanup(() => {
		globalThis.removeEventListener("click", handleClick);
		globalThis.removeEventListener("keydown", handleKeypress);
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
