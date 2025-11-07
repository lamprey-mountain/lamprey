import {
	createResource,
	createSignal,
	For,
	Show,
	type VoidProps,
} from "solid-js";
import { type User } from "sdk";
import { useCtx } from "../context";
import { useApi } from "../api";
import { Modal } from "../modal/mod";

export function Authentication(props: VoidProps<{ user: User }>) {
	const ctx = useCtx();

	return (
		<div class="user-settings-auth">
			<h2>authentication</h2>
			<div>authentication settings, words go here, etc etc</div>
			<br />
			<h3>email addreses</h3>
			<Email user={props.user} />
			<h3>oauth</h3>
			<Oauth />
			<br />
			<h3>totp</h3>
			<div>todo</div>
			<br />
			<h3>webauthn</h3>
			<div>todo</div>
			<br />
			<div class="danger">
				<h3>danger zone</h3>
				<div style="height: 4px"></div>
				<label>
					<button
						onClick={() =>
							ctx.dispatch({
								do: "modal.open",
								modal: { type: "settings", user_id: props.user.id },
							})}
					>
						change password
					</button>
					<span style="margin-left:8px">change your password</span>
				</label>
				<div style="height: 4px"></div>
				<label>
					<button onClick={() => alert("todo")}>disable</button>
					<span style="margin-left:8px">this will disable your account</span>
				</label>
				<div style="height: 4px"></div>
				<label>
					<button onClick={() => alert("todo")}>self destruct</button>
					<span style="margin-left:8px">this will delete your account</span>
				</label>
			</div>
		</div>
	);
}

function Email(_props: VoidProps<{ user: User }>) {
	const api = useApi();
	const ctx = useCtx();

	// TODO: use props.user.emails when sync events are implemented
	const [emails, { refetch }] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/user/{user_id}/email", {
			params: { path: { user_id: "@self" } },
		});
		return data;
	});

	function addEmail() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "email?",
			cont(email: string | null) {
				if (!email) return;
				api.client.http.PUT("/api/v1/user/{user_id}/email/{addr}", {
					params: { path: { user_id: "@self", addr: email } },
				}).then(refetch);
			},
		});
	}

	function deleteEmail(email: string) {
		ctx.dispatch({
			do: "modal.confirm",
			text: "delete email?",
			cont(conf: boolean) {
				if (!conf) return;
				api.client.http.DELETE("/api/v1/user/{user_id}/email/{addr}", {
					params: { path: { user_id: "@self", addr: email } },
				}).then(refetch);
			},
		});
	}

	function resendVerification(email: string) {
		api.client.http.POST(
			"/api/v1/user/{user_id}/email/{addr}/resend-verification",
			{
				params: { path: { user_id: "@self", addr: email } },
			},
		);
	}

	// TODO: button to make email primary

	return (
		<>
			<div class="email-list">
				<For each={emails()}>
					{(email) => (
						<div class="email">
							<div style="flex:1">
								<b>{email.email}</b>
								{email.is_verified ? " (verified)" : " (unverified)"}
								{email.is_primary && " (primary)"}
							</div>
							<menu>
								<Show when={!email.is_verified}>
									<button
										type="button"
										onClick={() => resendVerification(email.email)}
									>
										resend verification
									</button>
								</Show>
								<button
									class="danger"
									type="button"
									onClick={() => deleteEmail(email.email)}
								>
									delete
								</button>
							</menu>
						</div>
					)}
				</For>
			</div>
			<div class="email-add">
				<button class="primary" type="button" onClick={addEmail}>
					add email
				</button>
			</div>
		</>
	);
}

function Oauth() {
	const api = useApi();

	// TODO: dont use debug route for this
	// add something to sync i guess
	const [oauthProviders] = createResource(async () => {
		const { data } = await api.client.http.GET("/api/v1/debug/info");
		return data?.features.oauth.providers;
	});

	const [enabledOauthProviders, { refetch: refetchOauthProviders }] =
		createResource(async () => {
			const { data } = await api.client.http.GET("/api/v1/auth");
			return data?.oauth_providers;
		});

	const connectOauth = async (id: string) => {
		const url = await api.auth.oauthUrl(id);
		globalThis.open(url);
		// FIXME(#751): auth state update sync event
		// refetchOauthProviders();
	};

	const disconnectOauth = async (id: string) => {
		await api.client.http.DELETE("/api/v1/auth/oauth/{provider}", {
			params: { path: { provider: id } },
		});
		refetchOauthProviders();
	};

	return (
		<div class="oauth">
			<For each={oauthProviders()}>
				{(provider) => {
					const connected = () =>
						enabledOauthProviders()?.includes(provider.id);
					return (
						<div class="provider">
							<div style="flex:1">{provider.name}</div>
							<button
								onClick={() =>
									connected()
										? disconnectOauth(provider.id)
										: connectOauth(provider.id)}
								classList={{ danger: connected() }}
							>
								{connected() ? "disconnect" : "connect"}
							</button>
						</div>
					);
				}}
			</For>
		</div>
	);
}

export const ModalResetPassword = () => {
	const [password, setPassword] = createSignal("");
	const [confirmPassword, setConfirmPassword] = createSignal("");
	const ctx = useCtx();

	async function handlePasswordSet(e: SubmitEvent) {
		e.preventDefault();

		if (!password()) {
			ctx.dispatch({
				do: "modal.alert",
				text: "missing password",
			});
		}
		if (!confirmPassword()) {
			ctx.dispatch({
				do: "modal.alert",
				text: "missing confirmPassword",
			});
		}

		if (password() !== confirmPassword()) {
			ctx.dispatch({
				do: "modal.alert",
				text: "password !== confirmPassword",
			});
		}

		ctx.client.http.PUT("/api/v1/auth/password", {
			body: { password: password() },
		});
	}

	return (
		<Modal>
			<div class="auth">
				<section class="form-wrapper">
					reset password
					<form onSubmit={handlePasswordSet}>
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
						<label>
							<div class="label-text">confirm password</div>
							<input
								class="input"
								type="password"
								placeholder="dolphins"
								value={confirmPassword()}
								onInput={(e) => setConfirmPassword(e.currentTarget.value)}
							/>
						</label>
						<br />
						<br />
						<input class="submit-btn" type="submit" value={"set password"} />
					</form>
				</section>
			</div>
		</Modal>
	);
};
