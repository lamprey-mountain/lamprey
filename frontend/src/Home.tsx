import { Show } from "solid-js";
import { A } from "@solidjs/router";
import { useCtx } from "./context.ts";
import { useApi } from "./api.tsx";
import { flags } from "./flags.ts";

export const Home = () => {
	const ctx = useCtx();

	function createRoom() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont: (name: string | null) => {
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
			cont(invite_code: string | null) {
				if (!invite_code) return;
				ctx.client.http.POST("/api/v1/invite/{invite_code}", {
					params: {
						path: { invite_code },
					},
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

	async function loginGithub() {
		const res = await ctx.client.http.POST("/api/v1/auth/oauth/{provider}", {
			params: {
				path: {
					provider: "github",
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

	async function createSession() {
		const { data } = await ctx.client.http.POST("/api/v1/session", { body: {} });
		localStorage.setItem("token", data?.token!);
		location.reload();
	}

	async function createGuest() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.POST("/api/v1/guest", { body: { name } });
			}
		});
	}

	async function setPassword() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "password?",
			cont(password) {
				if (!password) return;
				ctx.client.http.PUT("/api/v1/auth/password", { body: { password } })
			}
		});
	}

	async function loginWithEmailPassword() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "email?",
			cont(email) {
				if (!email) return;
				ctx.dispatch({
					do: "modal.prompt",
					text: "password?",
					cont(password) {
						if (!password) return;
						ctx.client.http.POST("/api/v1/auth/password", { body: { email, type: "Email", password } })
					}
				});
			}
		});
	}

	const api = useApi();

	return (
		<div class="home">
			<h2>home</h2>
			<p>work in progress. expect bugs and missing polish.</p>
			<button onClick={loginDiscord}>login with discord</button>
			<button onClick={loginGithub}>login with github</button>
			<br />
			<button onClick={logout}>logout</button>
			<br />
			<br />
			<button onClick={createSession}>create session</button>
			<br />
			<button onClick={createGuest}>create guest</button>
			<br />
			<button onClick={setPassword}>set password</button>
			<br />
			<button onClick={loginWithEmailPassword}>
				login with email/password
			</button>
			<br />
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
			<Show when={flags.has("dev")}>
				<A href="/debug">debug</A>
				<br />
			</Show>
			<Show when={flags.has("inbox")}>
				<A href="/inbox">inbox</A>
				<br />
			</Show>
			<Show when={flags.has("friends")}>
				<A href="/friends">friends</A>
				<br />
			</Show>
		</div>
	);
};
