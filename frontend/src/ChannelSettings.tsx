import { For, Show } from "solid-js";
import type { Channel } from "sdk";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import { Info, Permissions, Todo } from "./channel_settings/mod.tsx";

const tabs = [
	{ name: "info", path: "", component: Info },
	{ name: "invites", path: "invites", component: Todo },
	{ name: "permissions", path: "permissions", component: Permissions },
	{ name: "members", path: "members", component: Todo },
];

export const ChannelSettings = (props: { channel: Channel; page: string }) => {
	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	return (
		<div class="settings">
			<header>
				channel settings: {currentTab()?.name}{" "}
				<A href={`/channel/${props.channel.id}`}>back</A>
			</header>
			<nav>
				<ul>
					<For each={tabs}>
						{(tab) => (
							<li>
								<A href={`/channel/${props.channel.id}/settings/${tab.path}`}>
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
						channel={props.channel}
					/>
				</Show>
			</main>
		</div>
	);
};
