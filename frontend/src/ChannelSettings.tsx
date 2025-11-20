import { For, Match, Show, Switch } from "solid-js";
import type { Channel } from "sdk";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import {
	Info,
	Invites,
	Permissions,
	Webhooks,
} from "./channel_settings/mod.tsx";
import { useCtx } from "./context.ts";
import { useModals } from "./contexts/modal.tsx";

const tabs = [
	{ name: "info", path: "", component: Info },
	{ name: "invites", path: "invites", component: Invites },
	{
		name: "permissions",
		path: "permissions",
		component: Permissions,
		noPad: true,
	},
	{ name: "webhooks", path: "webhooks", component: Webhooks },
	{ name: "remove channel", action: "remove", style: "danger" },
];

export const ChannelSettings = (props: { channel: Channel; page: string }) => {
	const ctx = useCtx();
	const [, modalCtl] = useModals();
	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	const handleAction = (action: string) => {
		switch (action) {
			case "remove":
				modalCtl.confirm(
					`Are you sure you want to remove "${props.channel.name}"?`,
					(confirmed) => {
						if (confirmed) {
							ctx.client.http.DELETE("/api/v1/channel/{channel_id}/remove", {
								params: { path: { channel_id: props.channel.id } },
							}).then(() => {
								// assuming channel has room_id
								const parentRoomId = props.channel.room_id;
								window.location.href = `/room/${parentRoomId}`;
							}).catch((error) => {
								console.error("Failed to remove channel:", error);
								modalCtl.alert("Failed to remove channel: " + error.message);
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
				channel settings: {currentTab()?.name}{" "}
				<A href={`/channel/${props.channel.id}`}>back</A>
			</header>
			<nav>
				<ul>
					<For each={tabs}>
						{(tab) => (
							<Switch>
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
										<A
											href={`/channel/${props.channel.id}/settings/${tab.path}`}
										>
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
						channel={props.channel}
					/>
				</Show>
			</main>
		</div>
	);
};
