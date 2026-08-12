import { createSignal, For, Show } from "solid-js";
import type { ApiError } from "ts-sdk";
import { useApi, useAuth } from "@/api";
import { useCtx } from "@/app/context";
import { useModals2 } from "@/contexts/modal";

// TODO: come up with more placeholder emails/passwords

const placeholderEmails = ["noreply@example.com"];

const placeholderPasswords = [
	"12345678",
	"hunter2",
	"dolphins",
	"********",
	"password",
	"admin",
];

function rnd<T>(arr: T[]) {
	return arr[Math.floor(Math.random() * arr.length)];
}

export const Authenticate = () => {
	const ctx = useCtx();
	const api = useApi();
	const auth = useAuth();
	const [email, setEmail] = createSignal("");
	const [password, setPassword] = createSignal("");
	const [emailError, setEmailError] = createSignal("");
	const [passwordError, setPasswordError] = createSignal("");
	const [loginError, setLoginError] = createSignal("");
	const [loggingIn, setLoggingIn] = createSignal(false);
	const [oauthing, setOauthing] = createSignal("");
	const modals = useModals2();

	async function loginOauth(provider: string) {
		try {
			setOauthing(provider);
			const url = await auth.oauthUrl(provider);
			globalThis.open(url);
		} finally {
			setOauthing("");
		}
	}

	async function handleAuthSubmit(e: SubmitEvent) {
		e.preventDefault();
		let valid = true;

		if (email().includes("@")) {
			setEmailError("");
		} else if (email()) {
			setEmailError("invalid email address");
			valid = false;
		} else {
			setEmailError("missing email");
			valid = false;
		}

		if (password()) {
			setPasswordError("");
		} else {
			setPasswordError("missing password");
			valid = false;
		}

		setLoginError("");

		if (!valid) return;

		setLoggingIn(true);
		auth
			.passwordLogin({
				type: "Email",
				email: email(),
				password: password(),
			})
			.then(() => {
				ctx.setPopout(null);
			})
			.catch((err: ApiError) => {
				setLoginError(getErrorMessage(err));
				setLoggingIn(false);
			});
	}

	async function createGuest() {
		ctx.setPopout(null);
		modals.prompt("name?").then((name) => {
			if (!name) return;
			api.users.createGuest(name);
		});
	}

	const oauthProviders = () => [
		{ id: "discord", label: "discord" },
		{ id: "github", label: "github" },
	];

	const loading = () => !!(loggingIn() || oauthing());

	return (
		<div class="authenticate">
			<section class="section-email-password">
				<form class="form" onSubmit={handleAuthSubmit}>
					<label>
						<h3 class="dim">email</h3>
						<input
							class="input"
							type="email"
							placeholder={rnd(placeholderEmails)}
							value={email()}
							onInput={(e) => setEmail(e.currentTarget.value)}
							ref={(el) => queueMicrotask(() => el.focus())}
						/>
						<Show when={emailError()}>
							{(err) => <div class="error">{err()}</div>}
						</Show>
					</label>
					<label>
						<h3 class="dim">password</h3>
						<input
							class="input"
							type="password"
							placeholder={rnd(placeholderPasswords)}
							value={password()}
							onInput={(e) => setPassword(e.currentTarget.value)}
						/>
						<Show when={passwordError()}>
							{(err) => <div class="error">{err()}</div>}
						</Show>
					</label>
					<Show when={loginError()}>
						{(err) => <div class="error">{err()}</div>}
					</Show>
					<button type="submit" class="button submit" disabled={loading()}>
						{loggingIn() ? "Logging in..." : "Login"}
					</button>
				</form>
			</section>
			<section class="section-oauth">
				<h3 class="dim">new user</h3>
				<button type="button" class="button primary" onClick={createGuest}>
					Create guest
				</button>

				<h3 class="dim" style="margin-top:8px">
					oauth
				</h3>
				<For each={oauthProviders()}>
					{(p) => (
						<button
							type="button"
							class="button oauth"
							onClick={[loginOauth, p.id]}
							disabled={loading()}
						>
							{oauthing() === p.id
								? `Requesting ${p.label}...`
								: `Login with ${p.label}`}
						</button>
					)}
				</For>
			</section>
		</div>
	);
};

function getErrorMessage(err: ApiError) {
	switch (err.code) {
		// FIXME: return the same error code for both cases
		case "UnknownUserEmail":
			return "Invalid email or password";
		case "InvalidPassword":
			return "Invalid email or password";
		default:
			return err.code;
	}
}
