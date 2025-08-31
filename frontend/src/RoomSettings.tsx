import { For, Show } from "solid-js";
import type { RoomT } from "./types.ts";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import { AuditLog } from "./room_settings/AuditLog.tsx";
import { Emoji } from "./room_settings/Emoji.tsx";
import { Info } from "./room_settings/Info.tsx";
import { Invites } from "./room_settings/Invites.tsx";
import { Members } from "./room_settings/Members.tsx";
import { Metrics } from "./room_settings/Metrics.tsx";
import { Roles } from "./room_settings/Roles.tsx";
import { Todo } from "./room_settings/Todo.tsx";

const tabs = [
	{ name: "info", path: "", component: Info },
	{ name: "invites", path: "invites", component: Invites },
	{ name: "roles", path: "roles", component: Roles, noPad: true },
	{ name: "members", path: "members", component: Members },
	{ name: "tags", path: "tags", component: Todo },
	{ name: "emoji", path: "emoji", component: Emoji },
	{ name: "audit log", path: "logs", component: AuditLog },
	{ name: "metrics", path: "metrics", component: Metrics },
];

export const RoomSettings = (props: { room: RoomT; page: string }) => {
	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	return (
		<div class="settings">
			<header>
				room settings: {currentTab()?.name}{" "}
				<A href={`/room/${props.room.id}`}>back</A>
			</header>
			<nav>
				<ul>
					<For each={tabs}>
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
