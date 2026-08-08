import { createSignal, For } from "solid-js";
import { useApi, useAuth } from "@/api";
import { useModals } from "@/contexts/modal";

export const Authenticate = () => {
	const api = useApi();
	const auth = useAuth();
	const [email, setEmail] = createSignal("");
	const [password, setPassword] = createSignal("");
	const [, modalctl] = useModals();

	async function loginOauth(provider: string) {
		const url = await auth.oauthUrl(provider);
		globalThis.open(url);
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
			api.users.createGuest(name);
		});
	}

	const oauthProviders = () => [
		{ id: "discord", label: "discord" },
		{ id: "github", label: "github" },
	];

	return (
		<div class="authenticate">
			<section class="section-email-password">
				<form class="form" onSubmit={handleAuthSubmit}>
					<label>
						<h3 class="dim">email</h3>
						<input
							class="input"
							type="email"
							placeholder="noreply@example.com"
							value={email()}
							onInput={(e) => setEmail(e.currentTarget.value)}
						/>
					</label>
					<label>
						<h3 class="dim">password</h3>
						<input
							class="input"
							type="password"
							placeholder="dolphins"
							value={password()}
							onInput={(e) => setPassword(e.currentTarget.value)}
						/>
					</label>
					<input class="button submit" type="submit" value="login" />
				</form>
			</section>
			<section class="section-oauth">
				<button type="button" class="button primary" onClick={createGuest}>
					create guest
				</button>
				<For each={oauthProviders()}>
					{(p) => (
						<button
							type="button"
							class="button oauth"
							onClick={[loginOauth, p.id]}
						>
							login with {p.label}
						</button>
					)}
				</For>
			</section>
		</div>
	);
};
