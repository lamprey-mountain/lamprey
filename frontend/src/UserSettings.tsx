import { For, Show } from "solid-js";
import { UserT } from "./types.ts";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import { Info, Sessions } from "./user_settings/mod.tsx";

const tabs = [
	{ name: "info", path: "", component: Info },
	{ name: "sessions", path: "sessions", component: Sessions },
];

export const UserSettings = (props: { user: UserT; page: string }) => {
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
