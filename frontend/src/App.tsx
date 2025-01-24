import { Component, onCleanup } from "solid-js";
import { ChatCtx, chatctx, Data } from "./context.ts";
import { createStore } from "solid-js/store";
import { Main } from "./Main.tsx";
import { createDispatcher } from "./dispatch.ts";
import { createClient } from "sdk";

const BASE_URL = localStorage.getItem("base_url") ??
	"https://chat.celery.eu.org";

const App: Component = () => {
	const TOKEN = localStorage.getItem("token")!;
	const client = createClient({
		baseUrl: BASE_URL,
		onState(state) {
			console.log({ state });
		},
		onSync(msg) {
			console.log("recv", msg);
			ctx.dispatch({
				do: "server",
				msg,
			});
		},
		onReady(msg) {
			ctx.dispatch({ do: "server.ready", msg });
		},
	});

	if (TOKEN) {
		client.start(TOKEN);
	} else {
		client.http.POST("/api/v1/session", {
			body: {},
		}).then((res) => {
			if (!res.data) {
				console.log("failed to init session", res.response);
				throw new Error("failed to init session");
			}
			const { token } = res.data;
			localStorage.setItem("token", token);
			client.start(token);
		});
	}

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
		uploads: {},
	});

	const ctx: ChatCtx = {
		client,
		data,
		dispatch: () => {
			throw new Error("oh no!");
		},
	};
	const dispatch = createDispatcher(ctx, update);
	ctx.dispatch = dispatch;
	ctx.dispatch({ do: "init" });

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
