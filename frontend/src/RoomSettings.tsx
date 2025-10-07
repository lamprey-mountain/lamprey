import { For, Show } from "solid-js";
import type { RoomT } from "./types.ts";
import { Dynamic } from "solid-js/web";
import { AuditLog } from "./room_settings/AuditLog.tsx";
import { Emoji } from "./room_settings/Emoji.tsx";
import { Info } from "./room_settings/Info.tsx";
import { Invites } from "./room_settings/Invites.tsx";
import { Members } from "./room_settings/Members.tsx";
import { Bans } from "./room_settings/Bans.tsx";
import { Metrics } from "./room_settings/Metrics.tsx";
import { Roles } from "./room_settings/Roles.tsx";
// import { Todo } from "./room_settings/Todo.tsx";
import { Integrations } from "./room_settings/Integrations.tsx";
import * as Admin from "./admin_settings/mod.tsx";
import { SERVER_ROOM_ID } from "sdk";
import { A } from "@solidjs/router";

const tabs = [
	{ name: "info", path: "", component: Info },
	{ name: "invites", path: "invites", component: Invites },
	{ name: "roles", path: "roles", component: Roles, noPad: true },
	{ name: "members", path: "members", component: Members },
	{ name: "bans", path: "bans", component: Bans },
	{ name: "integrations", path: "integrations", component: Integrations },
	// { name: "tags", path: "tags", component: Todo },
	{ name: "emoji", path: "emoji", component: Emoji },
	{ name: "audit log", path: "logs", component: AuditLog },
	{ name: "metrics", path: "metrics", component: Metrics },
];

const adminTabs = [
	{ name: "info", path: "", component: Admin.ServerInfo },
	{ name: "users", path: "users", component: Admin.Users },
	{ name: "rooms", path: "rooms", component: Admin.Rooms },
	{ name: "invites", path: "invites", component: Admin.Invites },
	{ name: "roles", path: "roles", component: Roles, noPad: true },
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
						{(tab) => (
							<li>
								<A href={`/room/${props.room.id}/settings/${tab.path}`}>
									{tab.name}
								</A>
							</li>
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
