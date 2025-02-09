import { For, Show } from "solid-js";
import { RoomT } from "./types.ts";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import {
	AuditLog,
	Info,
	Invites,
	Members,
	Roles,
} from "./room_settings/mod.tsx";

const tabs = [
	{ name: "info", path: "", component: Info },
	{ name: "invites", path: "invites", component: Invites },
	{ name: "roles", path: "roles", component: Roles },
	{ name: "members", path: "members", component: Members },
	{ name: "audit log", path: "logs", component: AuditLog },
];

export const RoomSettings = (props: { room: RoomT; page: string }) => {
	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	return (
		<div class="settings">
			<header>
				room settings: {currentTab()?.name}
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
			<main>
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
