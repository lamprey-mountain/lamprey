import { createSignal, Show } from "solid-js";
import { A } from "@solidjs/router";
import { useCtx } from "./context.ts";
import { useApi } from "./api.tsx";
import { flags } from "./flags.ts";

export const Home = () => {
	const ctx = useCtx();
	const api = useApi();
	const [email, setEmail] = createSignal("");
	const [password, setPassword] = createSignal("");
	const [confirmPassword, setConfirmPassword] = createSignal("");

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

	async function handleAuthSubmit(e: SubmitEvent) {
		e.preventDefault();

		if (!email()) {
			ctx.dispatch({
				do: "modal.alert",
				text: "missing email",
			});
		}

		if (!password()) {
			ctx.dispatch({
				do: "modal.alert",
				text: "missing password",
			});
		}

		ctx.client.http.POST("/api/v1/auth/password", {
			body: { type: "Email", email: email(), password: password() },
		});
	}

	async function createGuest() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.POST("/api/v1/guest", { body: { name } }).then(() => {
					location.reload();
				});
			},
		});
	}

	return (
		<div class="home">
			<h2>home</h2>
			<p>work in progress. expect bugs and missing polish.</p>
			<br />
			<Show when={api.session()?.status === "Unauthorized"}>
				<div class="auth border">
					<section class="form-wrapper">
						<form onSubmit={handleAuthSubmit}>
							<label>
								<div class="label-text">email</div>
								<input
									class="input"
									type="email"
									placeholder="noreply@example.com"
									value={email()}
									onInput={(e) => setEmail(e.currentTarget.value)}
								/>
							</label>
							<br />
							<label>
								<div class="label-text">password</div>
								<input
									class="input"
									type="password"
									placeholder="dolphins"
									value={password()}
									onInput={(e) => setPassword(e.currentTarget.value)}
								/>
							</label>
							<br />
							<input class="submit-btn" type="submit" value="login" />
						</form>
					</section>
					<section class="social-wrapper">
						<ul class="social-list">
							<li class="social-item">
								<button class="social-button" onClick={loginDiscord}>
									login with discord
								</button>
							</li>
							<li class="social-item">
								<button class="social-button" onClick={loginGithub}>
									login with github
								</button>
							</li>
						</ul>
					</section>
				</div>
				<br />
				<button onClick={createGuest}>create guest</button>
			</Show>
			<Show when={api.session() && api.session()?.status !== "Unauthorized"}>
				<button onClick={logout}>logout</button>
			</Show>
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
