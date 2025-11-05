import { createResource, For } from "solid-js";
import { useApi } from "../api";
import { Time } from "../Time";
import { Copyable } from "../util";
import type { Scope } from "sdk";

export function Connections() {
	const api = useApi();

	const [connections] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/user/{user_id}/connection",
			{ params: { path: { user_id: "@self" } } },
		);
		return data;
	});

	const deauthorize = (id: string) => {
		api.client.http.DELETE("/api/v1/user/{user_id}/connection/{app_id}", {
			params: {
				path: { app_id: id, user_id: "@self" },
			},
		});
	};

	// TODO: search authorized apps
	// TODO: handle ConnectionCreate in api sync
	// TODO: handle ConnectionDelete in api sync

	return (
		<div class="user-settings-connections">
			<h2>connections</h2>
			<For each={connections()?.items}>
				{(c) => (
					<article class="connection">
						<header>
							<div class="name">{c.application.name}</div>
							<div class="dim">
								<button
									onClick={() =>
										navigator.clipboard.writeText(c.application.id)}
								>
									copy id
								</button>
							</div>
							<div class="dim">
								authorized <Time date={new Date(c.created_at)} />
							</div>
						</header>
						<div class="info">
							<div>
								<div class="dim">Description</div>
								<div>{c.application.description}</div>
							</div>
							<div>
								<div class="dim">Permissions</div>
								<ul>{c.scopes.map((s) => <li>{formatScope(s)}</li>)}</ul>
							</div>
							<menu>
								<button
									class="danger"
									onClick={() => deauthorize(c.application.id)}
								>
									deauthorize
								</button>
							</menu>
						</div>
					</article>
				)}
			</For>
		</div>
	);
}

function formatScope(scope: Scope): string {
	switch (scope) {
		case "identify":
			return "Read basic profile information (name, avatar, etc)";
		// case "email": return "Read your email address(es)";
		case "full":
			return "**FULL ACESSS** to your account";
		case "auth":
			return "**FULL ACESSS** to your account, including authentication info";
	}
}
