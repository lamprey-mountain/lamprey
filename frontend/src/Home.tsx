import { Show } from "solid-js";
import { A } from "@solidjs/router";
import { useCtx } from "./context.ts";

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
			cont(code) {
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

	return (
		<div class="home">
			<h2>home</h2>
			<p>work in progress. expect bugs and missing polish.</p>
			<button onClick={createRoom}>
				create room
			</button>
			<br />
			<button onClick={useInvite}>use invite</button>
			<br />
			<Show when={ctx.data.user}>
				<A href="/settings">settings</A>
				<br />
			</Show>
			<button onClick={loginDiscord}>login with discord</button>
			<br />
			<button onClick={logout}>logout</button>
			<br />
			<A target="_self" href="/api/docs">api docs</A>
			<br />
		</div>
	);
};

