import { A } from "@solidjs/router";
import { createEffect, createSignal, Show } from "solid-js";
import {
	useApi,
	useAuth,
	useInvites,
	useRooms,
	useSessions,
	useUsers,
} from "@/api";
import { UnicodeEmoji } from "@/atoms/UnicodeEmoji";
import { useCurrentUser } from "@/contexts/currentUser";
import { useModals } from "@/contexts/modal";
import { flags } from "@/lib/flags";

export const Home = () => {
	const api = useApi();
	const auth = useAuth();
	const rooms = useRooms();
	const invites = useInvites();
	const users = useUsers();
	const user = useCurrentUser();
	const [email, setEmail] = createSignal("");
	const [password, setPassword] = createSignal("");
	const [, modalctl] = useModals();

	function openRoomModal() {
		modalctl.open({
			type: "room_create_or_join",
			onCreate: (data: { name: string; public: boolean } | null) => {
				if (!data) return;
				rooms.create({ name: data.name, public: data.public });
			},
			onInvite: (invite_code: string | null) => {
				if (!invite_code) return;
				invites.accept(invite_code);
			},
		});
	}

	function useInvite() {
		modalctl.prompt("invite code?", (invite_code: string | null) => {
			if (!invite_code) return;
			invites.accept(invite_code);
		});
	}

	async function loginDiscord() {
		const url = await auth.oauthUrl("discord");
		globalThis.open(url);
	}

	async function loginGithub() {
		const url = await auth.oauthUrl("github");
		globalThis.open(url);
	}

	async function logout() {
		await api.logout();
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

		auth.passwordLogin({
			type: "Email",
			email: email(),
			password: password(),
		});
	}

	async function createGuest() {
		modalctl.prompt("name?", (name) => {
			if (!name) return;
			users.createGuest(name);
		});
	}

	const isUnauthorized = () =>
		api.session() === null || api.session()?.status === "Unauthorized";
	const isAuthorized = () => api.session()?.status === "Authorized";

	return (
		<div class="home">
			<h2>home</h2>
			<p>welcome to lamprey mountain, the internet's finest asylum</p>
			<p>work in progress. expect bugs and missing polish.</p>
			<UnicodeEmoji hex="1F345" />
			<Show when={isUnauthorized()}>
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
							<input class="button submit-btn" type="submit" value="login" />
						</form>
					</section>
					<section class="social-wrapper">
						<ul class="social-list">
							<li class="social-item">
								<button
									type="button"
									class="button social-button"
									onClick={loginDiscord}
								>
									login with discord
								</button>
							</li>
							<li class="social-item">
								<button
									type="button"
									class="button social-button"
									onClick={loginGithub}
								>
									login with github
								</button>
							</li>
						</ul>
					</section>
				</div>
				<br />
				<button type="button" class="button" onClick={createGuest}>
					create guest
				</button>
				<br />
			</Show>

			<Show when={isAuthorized()}>
				<button type="button" class="button" onClick={logout}>
					logout
				</button>
				<br />
				<br />
				<Show when={user()}>
					<button type="button" class="button" onClick={openRoomModal}>
						create or join room
					</button>
					<br />
				</Show>
			</Show>

			<A target="_self" href="/api/docs">
				api docs
			</A>
			<br />
			<Show when={flags.has("dev")}>
				<A href="/debug">debug</A>
				<br />
			</Show>
		</div>
	);
};
