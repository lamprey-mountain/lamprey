import { createSignal, For, Show } from "solid-js";
import { A } from "@solidjs/router";
import { useCtx } from "./context.ts";
import { useApi } from "./api.tsx";
import { createScheduled, leadingAndTrailing, throttle, } from "@solid-primitives/scheduled";
import { createResource } from "solid-js";
import { MessageView } from "./Message.tsx";
import { Message } from "sdk";
import { flags } from "./flags.ts";

export const Home = () => {
	const ctx = useCtx();

	function createRoom() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.POST("/api/v1/room", {
					body: { name },
				});
			},
		});
	}

	function useInvite() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "invite code?",
			cont(_code) {
				// TODO: fix
				// ctx.client.http.POST("/api/v1/invite")
				// ctx.client.http("POST", `/api/v1/invites/${code}`, {});
				queueMicrotask(() => {
					ctx.dispatch({ do: "modal.alert", text: "todo!" });
				});
			},
		});
	}

	async function loginDiscord() {
		const res = await ctx.client.http.POST("/api/v1/auth/oauth/{provider}", {
			params: {
				path: {
					provider: "discord",
				},
			},
		});
		if (res.error) {
			ctx.dispatch({ do: "modal.alert", text: "failed to create login url" });
			return;
		}
		globalThis.open(res.data.url);
	}

	async function logout() {
		await ctx.client.http.DELETE("/api/v1/session/{session_id}", {
			params: {
				path: {
					session_id: "@self",
				},
			},
		});
		localStorage.clear();
		location.reload(); // TODO: less hacky logout
	}

	const api = useApi();

	return (
		<div class="home">
			<h2>home</h2>
			<p>work in progress. expect bugs and missing polish.</p>
			<button onClick={loginDiscord}>login with discord</button>
			<br />
			<button onClick={logout}>logout</button>
			<br />
			<Show when={api.users.cache.get("@self")}>
				<button onClick={createRoom}>
					create room
				</button>
				<br />
				<button onClick={useInvite}>use invite</button>
				<br />
				<A href="/settings">settings</A>
				<br />
			</Show>
			<A target="_self" href="/api/docs">api docs</A>
			<br />
			<Show when={flags.has("message_search")}>
				<Search />
			</Show>
		</div>
	);
};

const Search = () => {
	const ctx = useCtx();
	const [searchQuery, setSearchQueryRaw] = createSignal<string>("");
	const setSearchQuery = leadingAndTrailing(throttle, setSearchQueryRaw, 300);
	const [searchResults] = createResource(
		searchQuery as any,
		(async (query: string, { value }: { value?: Array<Message> }) => {
			if (!query) return;
			const { data, error } = await ctx.client.http.POST(
				"/api/v1/search/message",
				{
					body: { query },
				},
			);
			if (error) throw new Error(error);
			return data.items;
		}) as any,
	);

	return (
		<>
			<h3>experiments</h3>
			<label>
				search messages:{" "}
				<input type="text" onInput={(e) => setSearchQuery(e.target.value)} />
			</label>
			<br />
			<Show when={searchResults.loading}>loading...</Show>
			<For each={searchResults() as any}>
				{(m: Message) => (
					<li class="message">
						<MessageView message={m} />
					</li>
				)}
			</For>
		</>
	);
};
