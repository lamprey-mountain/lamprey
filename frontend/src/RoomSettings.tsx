import { For, Match, Show, Switch } from "solid-js";
import type { RoomT } from "./types.ts";
import { Dynamic } from "solid-js/web";
import {
	AuditLog,
	Bans,
	Emoji,
	Info,
	Integrations,
	Invites,
	Members,
	Metrics,
	Roles,
} from "./room_settings/mod.tsx";
import * as Admin from "./admin_settings/mod.tsx";
import { SERVER_ROOM_ID } from "sdk";
import { A } from "@solidjs/router";

const tabs = [
	{ category: "overview" },
	{ name: "info", path: "", component: Info },
	{ name: "emoji", path: "emoji", component: Emoji },
	{ name: "metrics", path: "metrics", component: Metrics },
	{ category: "access" },
	{ name: "invites", path: "invites", component: Invites },
	{ name: "roles", path: "roles", component: Roles, noPad: true },
	{ name: "members", path: "members", component: Members },
	{ name: "integrations", path: "integrations", component: Integrations },
	{ category: "moderation" },
	{ name: "bans", path: "bans", component: Bans },
	{ name: "audit log", path: "logs", component: AuditLog },
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
	const currentTabs = () => props.room.id === SERVER_ROOM_ID ? adminTabs : tabs;
	const currentTab = () =>
		currentTabs().find((i) => i.path === (props.page ?? ""))!;

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
								<Match when={tab.name}>
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
