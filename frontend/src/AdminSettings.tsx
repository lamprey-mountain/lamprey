import { createEffect } from "solid-js";
import { Info, Todo } from "./admin_settings/mod.tsx";
import { For, Show } from "solid-js";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";

const tabs = [
	{ name: "info", path: "", component: Info },
];

export const AdminSettings = (props: { page: string }) => {
	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	return (
		<div class="settings">
			<header>
				admin settings: {currentTab()?.name}
				{" "}
			</header>
			<nav>
				<ul>
					<For each={tabs}>
						{(tab) => (
							<li>
								<A href={`/admin/${tab.path}`}>
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
					/>
				</Show>
			</main>
		</div>
	);
};
