import { createResource, createSignal, For, onCleanup } from "solid-js";
import { useApi2 } from "@/api";
import { Time } from "../../../atoms/Time";
import { Copyable } from "../../../utils/general";
import type { Scope } from "sdk";
import fuzzysort from "fuzzysort";

export function Connections() {
	const api2 = useApi2();
	const [connecting, setConnecting] = createSignal(false);

	const [connections, { refetch }] = createResource(async () => {
		const { data } = await api2.client.http.GET(
			"/api/v1/user/{user_id}/connection",
			{ params: { path: { user_id: "@self" } } },
		);
		return data;
	});

	const deauthorize = async (id: string) => {
		await api2.client.http.DELETE(
			"/api/v1/user/{user_id}/connection/{app_id}",
			{
				params: {
					path: { app_id: id, user_id: "@self" },
				},
			},
		);
		refetch();
	};

	let removed = false;

	const handleMessage = (event: MessageEvent) => {
		if (event.origin !== window.location.origin) return;
		if (event.data?.type === "oauth_success") {
			refetch();
			removed = true;
			window.removeEventListener("message", handleMessage);
		}
	};

	if (typeof window !== "undefined") {
		window.addEventListener("message", handleMessage);
		onCleanup(() => {
			if (!removed) {
				console.warn("connection listener wasn't removed");
				window.removeEventListener("message", handleMessage);
			}
		});
	}

	const [search, setSearch] = createSignal("");

	const filteredConnections = () => {
		const query = search();
		if (!query) return connections()?.items ?? [];
		const results = fuzzysort.go(query, connections()?.items ?? [], {
			key: "application.name",
			threshold: -10000,
		});
		return results.map((r) => r.obj);
	};

	return (
		<div class="user-settings-connections">
			<h2>connections</h2>
			<header class="connections-header">
				<input
					type="search"
					placeholder="search"
					aria-label="search"
					onInput={(e) => setSearch(e.target.value)}
				/>
			</header>
			<For each={filteredConnections()}>
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
			<div class="add-connection">
				<button
					onClick={() => {
						setConnecting(true);
						// TODO: show list of available applications to connect
						// For now, just refetch to show any new connections
						setTimeout(() => {
							refetch();
							setConnecting(false);
						}, 1000);
					}}
				>
					add connection
				</button>
			</div>
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
		default:
			return scope;
	}
}
