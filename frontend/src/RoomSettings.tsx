import { For, Match, Show, Switch } from "solid-js";
import type { RoomT } from "./types.ts";
import { Dynamic } from "solid-js/web";
import {
	AuditLog,
	Bans,
	Bots,
	Emoji,
	Info,
	Invites,
	Members,
	Metrics,
	Roles,
	Webhooks,
} from "./room_settings/mod.tsx";
import * as Admin from "./admin_settings/mod.tsx";
import { SERVER_ROOM_ID } from "sdk";
import { A } from "@solidjs/router";
import { useCtx } from "./context.ts";
import { useApi } from "./api.tsx";
import { useModals } from "./contexts/modal.tsx";

const tabs = [
	{ category: "overview" },
	{ name: "info", path: "", component: Info },
	{ name: "emoji", path: "emoji", component: Emoji },
	{ name: "metrics", path: "metrics", component: Metrics },
	{ category: "integrations" },
	{ name: "bots", path: "bots", component: Bots },
	{ name: "webhooks", path: "webhooks", component: Webhooks },
	{ category: "access" },
	{ name: "invites", path: "invites", component: Invites },
	{ name: "roles", path: "roles", component: Roles, noPad: true },
	{ name: "members", path: "members", component: Members },
	{ category: "moderation" },
	{ name: "bans", path: "bans", component: Bans },
	{ name: "audit log", path: "logs", component: AuditLog },
	{ name: "delete room", action: "delete", style: "danger" },
];

const adminTabs = [
	{ category: "overview" },
	{ name: "info", path: "", component: Admin.ServerInfo },
	{ category: "access" },
	{ name: "invites", path: "invites", component: Admin.Invites },
	{ name: "roles", path: "roles", component: Roles, noPad: true },
	{ category: "content" },
	{ name: "users", path: "users", component: Admin.Users },
	{ name: "rooms", path: "rooms", component: Admin.Rooms },
	{ category: "moderation" },
	{ name: "audit log", path: "logs", component: Admin.AuditLog },
];

export const RoomSettings = (props: { room: RoomT; page: string }) => {
	const ctx = useCtx();
	const api = useApi();
	const [, modalCtl] = useModals();
	const currentTabs = () => props.room.id === SERVER_ROOM_ID ? adminTabs : tabs;
	const currentTab = () =>
		currentTabs().find((i) => i.path === (props.page ?? ""))!;

	const handleAction = (action: string) => {
		switch (action) {
			case "delete":
				modalCtl.confirm(
					`Are you sure you want to delete "${props.room.name}"?`,
					(confirmed) => {
						if (confirmed) {
							ctx.client.http.DELETE("/api/v1/room/{room_id}", {
								params: { path: { room_id: props.room.id } },
							}).then(() => {
								window.location.href = "/";
							}).catch((error) => {
								console.error("Failed to delete room:", error);
								modalCtl.alert("Failed to delete room: " + error.message);
							});
						}
					},
				);
				break;
			default:
				console.warn(`Unknown action: ${action}`);
		}
	};

	return (
		<div class="settings">
			<header>
				{props.room.id === SERVER_ROOM_ID ? "admin settings" : "room settings"}
				: {currentTab()?.name} <A href={`/room/${props.room.id}`}>back</A>
			</header>
			<nav>
				<ul>
					<For each={currentTabs()}>
						{(tab, idx) => (
							<Switch>
								<Match when={tab.category}>
									<div
										class="dim"
										style={{
											"margin-top": idx() === 0 ? "" : "12px",
											"margin": "2px 8px",
										}}
									>
										{tab.category}
									</div>
								</Match>
								<Match when={tab.action}>
									<li>
										<button
											class="action"
											onClick={() => handleAction(tab.action)}
											style={{
												color: tab.style === "danger"
													? "oklch(var(--color-red))"
													: "inherit",
											}}
										>
											{tab.name}
										</button>
									</li>
								</Match>
								<Match when={true}>
									<li>
										<A href={`/room/${props.room.id}/settings/${tab.path}`}>
											{tab.name}
										</A>
									</li>
								</Match>
							</Switch>
						)}
					</For>
				</ul>
			</nav>
			<main classList={{ padded: !currentTab()?.noPad }}>
				<Show when={currentTab()} fallback="unknown page">
					<Dynamic
						component={currentTab()?.component}
						room={props.room}
					/>
				</Show>
			</main>
		</div>
	);
};
