import { Component, For, Match, Show, Switch } from "solid-js";
import type { Channel, Permission } from "sdk";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import {
	Info,
	Invites,
	Permissions,
	Tags,
	Webhooks,
} from "./components/features/channel_settings/mod.tsx";
import { useCtx } from "./context.ts";
import { useModals } from "./contexts/modal.tsx";
import { usePermissions } from "./hooks/usePermissions.ts";
import { useApi2 } from "@/api";
import { useCurrentUser } from "./contexts/currentUser.tsx";

type PermissionCheck = (p: ReturnType<typeof usePermissions>) => boolean;

const tabs: Array<{
	name: string;
	path: string;
	noPad?: boolean;
	component: Component<any> | null;
	action?: "remove";
	style?: string;
	permissionCheck?: PermissionCheck;
	channelTypes?: string[];
}> = [
	{ name: "info", path: "", component: Info },
	{
		name: "invites",
		path: "invites",
		component: Invites,
		permissionCheck: (p) => p.has("InviteManage"),
	},
	{
		name: "permissions",
		path: "permissions",
		component: Permissions,
		noPad: true,
		permissionCheck: (p) => p.has("RoleManage"),
	},
	{
		name: "tags",
		path: "tags",
		component: Tags,
		permissionCheck: (p) => p.has("RoleManage"),
		channelTypes: ["Forum", "Forum2"],
		// permissionCheck: (p) => p.has("RoleManage") || p.has("ChannelManage"),
	},
	{
		name: "webhooks",
		path: "webhooks",
		component: Webhooks,
		permissionCheck: (p) => p.has("IntegrationsManage"),
	},
	{
		name: "remove channel",
		path: "remove",
		component: null as any,
		action: "remove",
		style: "danger",
		// TODO: check ThreadManage in threads, ChannelManage in channels
		permissionCheck: (p) => p.has("ThreadManage") || p.has("ChannelManage"),
	},
];

export const ChannelSettings = (props: { channel: Channel; page: string }) => {
	const ctx = useCtx();
	const api2 = useApi2();
	const [, modalCtl] = useModals();
	const currentUser = useCurrentUser();
	const user_id = () => currentUser()?.id;
	const perms = usePermissions(
		user_id,
		() => props.channel.room_id ?? undefined,
		() => props.channel.id,
	);

	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	const handleAction = (action: string) => {
		switch (action) {
			case "remove":
				modalCtl.confirm(
					`Are you sure you want to remove "${props.channel.name}"?`,
					(confirmed) => {
						if (confirmed) {
							api2.client.http.DELETE("/api/v1/channel/{channel_id}/remove", {
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
							<Show
								when={(!tab.channelTypes ||
									tab.channelTypes.includes(props.channel.type)) &&
									(!tab.permissionCheck || tab.permissionCheck(perms))}
							>
								<Switch>
									<Match when={tab.action}>
										<li>
											<button
												class="action"
												onClick={() => handleAction(tab.action!)}
												classList={{
													"danger": tab.style === "danger",
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
							</Show>
						)}
					</For>
				</ul>
			</nav>
			<main classList={{ padded: !currentTab()?.noPad }}>
				<Show when={currentTab()?.component} fallback="unknown page">
					<Dynamic
						component={currentTab()!.component!}
						channel={props.channel}
					/>
				</Show>
			</main>
		</div>
	);
};
