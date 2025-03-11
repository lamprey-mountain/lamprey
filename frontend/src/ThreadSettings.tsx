import { For, Show } from "solid-js";
import type { ThreadT } from "./types.ts";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import { Info } from "./thread_settings/mod.tsx";

const tabs = [
	{ name: "info", path: "", component: Info },
	// TODO: { name: "invites", path: "invites", component: Invites },
	// TODO: { name: "roles", path: "roles", component: Roles },
	// TODO: { name: "members", path: "members", component: Members },
];

export const ThreadSettings = (props: { thread: ThreadT; page: string }) => {
	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	return (
		<div class="settings">
			<header>
				thread settings: {currentTab()?.name}
			</header>
			<nav>
				<ul>
					<For each={tabs}>
						{(tab) => (
							<li>
								<A href={`/thread/${props.thread.id}/settings/${tab.path}`}>
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
						thread={props.thread}
					/>
				</Show>
			</main>
		</div>
	);
};
