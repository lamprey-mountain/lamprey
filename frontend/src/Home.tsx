import { createSignal, Show } from "solid-js";
import { A } from "@solidjs/router";
import { useCtx } from "./context.ts";
import { useApi } from "./api.tsx";
import { useModals } from "./contexts/modal";
import { flags } from "./flags.ts";

export const Home = () => {
	const api = useApi();
	const [email, setEmail] = createSignal("");
	const [password, setPassword] = createSignal("");
	const [confirmPassword, setConfirmPassword] = createSignal("");
	const [, modalctl] = useModals();

	function createRoom() {
		modalctl.prompt("name?", (name: string | null) => {
			if (!name) return;
			api.rooms.create({ name });
		});
	}

	function useInvite() {
		modalctl.prompt("invite code?", (invite_code: string | null) => {
			if (!invite_code) return;
			api.invites.use(invite_code);
		});
	}

	async function loginDiscord() {
		const url = await api.auth.oauthUrl("discord");
		globalThis.open(url);
	}

	async function loginGithub() {
		const url = await api.auth.oauthUrl("github");
		globalThis.open(url);
	}

	async function logout() {
		await api.sessions.delete("@self");
		localStorage.clear();
		location.reload(); // TODO: less hacky logout
	}

	async function handleAuthSubmit(e: SubmitEvent) {
		e.preventDefault();

		if (!email()) {
			modalctl.alert("missing email");
			return;
		}

		if (!password()) {
			modalctl.alert("missing password");
			return;
		}

		api.auth.passwordLogin({
			type: "Email",
			email: email(),
			password: password(),
		});
	}

	async function createGuest() {
		modalctl.prompt("name?", (name) => {
			if (!name) return;
			api.users.createGuest(name).then(() => {
				location.reload();
			});
		});
	}

	return (
		<div class="home">
			<h2>home</h2>
			<p>welcome to lamprey mountain, the internet's finest asylum</p>
			<p>work in progress. expect bugs and missing polish.</p>
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
			<Show when={flags.has("friends")}>
				<A href="/friends">friends</A>
				<br />
			</Show>
		</div>
	);
};
