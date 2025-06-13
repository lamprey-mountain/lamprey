import { For, Show } from "solid-js";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import { Info, Sessions } from "./user_settings/mod.tsx";
import type { User } from "sdk";
import { Todo } from "./user_settings/Todo.tsx";
import { Applications } from "./user_settings/Applications.tsx";

const tabs = [
	{ name: "info", path: "", component: Info },
	{ name: "sessions", path: "sessions", component: Sessions },
	{ name: "audit log", path: "audit-log", component: Todo },
	{ name: "notifications", path: "notifications", component: Todo },
	{ name: "blocked users", path: "blocks", component: Todo },
	{ name: "email", path: "email", component: Todo },
	{ name: "applications", path: "applications", component: Applications },
];

export const UserSettings = (props: { user: User; page: string }) => {
	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	return (
		<div class="settings">
			<header>
				user settings <A href="/">home</A>
			</header>
			<nav>
				<ul>
					<For each={tabs}>
						{(tab) => (
							<li>
								<A href={`/settings/${tab.path}`}>
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
						user={props.user}
					/>
				</Show>
			</main>
		</div>
	);
};
