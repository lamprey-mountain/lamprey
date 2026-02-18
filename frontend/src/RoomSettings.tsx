import { Component, createMemo, For, Match, Show, Switch } from "solid-js";
import type { RoomT } from "./types.ts";
import { Dynamic } from "solid-js/web";
import {
	AuditLog,
	Automod,
	Bans,
	Bots,
	Emoji,
	Info,
	Invites,
	Members,
	Metrics as Analytics,
	Roles,
	Webhooks,
} from "./room_settings/mod.tsx";
import * as Admin from "./admin_settings/mod.tsx";
import { Permission, SERVER_ROOM_ID } from "sdk";
import { A } from "@solidjs/router";
import { useCtx } from "./context.ts";
import { useApi } from "./api.tsx";
import { useModals } from "./contexts/modal.tsx";
import { usePermissions } from "./hooks/usePermissions.ts";
import { flags } from "./flags.ts";

// TODO: more permission checks
const tabs: Array<
	{ category: string } | {
		name: string;
		path: string;
		noPad?: boolean;
		// TODO: fix type errors
		// component: Component,
		component: any;
		permissionCheck?: (p: Set<Permission>) => boolean;
		ownerOnly?: boolean;
	} | {
		name: string;
		action: "delete";
		style?: "danger";
		permissionCheck?: (p: Set<Permission>) => boolean;
		ownerOnly?: boolean;
	}
> = [
	{ category: "overview" },
	{ name: "info", path: "", component: Info },
	{
		name: "analytics",
		path: "analytics",
		component: Analytics,
		permissionCheck: (p) => p.has("ViewAnalytics"),
	},
	{ name: "emoji", path: "emoji", component: Emoji },
	{ category: "integrations" },
	{ name: "bots", path: "bots", component: Bots },
	{
		name: "webhooks",
		path: "webhooks",
		component: Webhooks,
		permissionCheck: (p) => p.has("IntegrationsManage"),
	},
	{ category: "access" },
	{
		name: "invites",
		path: "invites",
		component: Invites,
		permissionCheck: (p) => p.has("InviteManage"),
	},
	{
		name: "roles",
		path: "roles",
		component: Roles,
		noPad: true,
		permissionCheck: (p) => p.has("RoleManage"),
	},
	{ name: "members", path: "members", component: Members },
	{ category: "moderation" },
	{
		name: "automod",
		path: "automod",
		component: Automod,
		permissionCheck: (p) => p.has("RoomManage") && flags.has("automod"),
	},
	{
		name: "bans",
		path: "bans",
		component: Bans,
		permissionCheck: (p) => p.has("MemberBan"),
	},
	{
		name: "audit log",
		path: "logs",
		component: AuditLog,
		permissionCheck: (p) => p.has("ViewAuditLog"),
	},
	{
		name: "delete room",
		action: "delete",
		style: "danger",
		ownerOnly: true,
	},
];

const todo = (what: string) => () => `todo: ${what}` as Component;

const adminTabs: Array<
	{ category: string } | {
		name: string;
		path: string;
		noPad?: boolean;
		// TODO: fix type errors
		// component: Component,
		component: any;
		permissionCheck?: (p: Set<Permission>) => boolean;
		ownerOnly?: boolean;
	}
> = [
	{ category: "overview" },
	{ name: "info", path: "", component: Admin.ServerInfo },

	// control access to this server
	{ category: "access" },
	{
		name: "invites",
		path: "invites",
		component: Admin.Invites,
		permissionCheck: (p) => p.has("InviteManage"),
	},
	{
		name: "roles",
		path: "roles",
		component: Roles,
		noPad: true,
		permissionCheck: (p) => p.has("RoleManage"),
	},

	// manage data/content/resources on this server
	{ category: "resources" },
	{ name: "users", path: "users", component: Admin.Users },
	{ name: "rooms", path: "rooms", component: Admin.Rooms },
	{ name: "media", path: "media", component: todo("query and manage media") },
	{
		name: "applications",
		path: "applications",
		component: todo("query and manage applications"),
	},
	{
		name: "channels",
		path: "channels",
		component: todo("query and manage channels"),
	},
	{
		name: "servers",
		path: "servers",
		component: todo("query and manage servers"),
	},

	// manage services
	{ category: "service" },
	{
		name: "voice",
		path: "voice",
		component: todo("list and manage voice sfus/servers"),
	},
	// { name: "media", path: "media", component: todo("view stats about cdn/media?") },

	// server moderation
	{ category: "moderation" },
	{
		name: "audit log",
		path: "logs",
		component: Admin.AuditLog,
		permissionCheck: (p) => p.has("ViewAuditLog"),
	},
];

type TabItem = typeof tabs[number];
type GroupedTab = {
	category: string;
	items: Exclude<TabItem, { category: string }>[];
};

function groupTabsByCategory(
	tabs: TabItem[],
	perms: ReturnType<typeof usePermissions>,
	user_id: () => string | undefined,
	room: RoomT,
): GroupedTab[] {
	const groups: GroupedTab[] = [];
	let currentGroup: GroupedTab | null = null;

	for (const tab of tabs) {
		if ("category" in tab) {
			currentGroup = { category: tab.category, items: [] };
			groups.push(currentGroup);
		} else if (currentGroup) {
			const isVisible = (!tab.permissionCheck || tab.permissionCheck(perms)) &&
				(!tab.ownerOnly || room.owner_id === user_id());
			if (isVisible) {
				currentGroup.items.push(tab);
			}
		}
	}

	return groups.filter((g) => g.items.length > 0);
}

export const RoomSettings = (props: { room: RoomT; page: string }) => {
	const ctx = useCtx();
	const api = useApi();
	const [, modalCtl] = useModals();
	const user_id = () => api.users.cache.get("@self")?.id;
	const perms = usePermissions(
		user_id,
		() => props.room.id,
		() => undefined,
	);
	const currentTabs = () => props.room.id === SERVER_ROOM_ID ? adminTabs : tabs;
	const currentTab = () =>
		currentTabs().find((i) => i.path === (props.page ?? ""))!;

	const groupedTabs = createMemo(() =>
		groupTabsByCategory(currentTabs(), perms, user_id, props.room)
	);

	const handleAction = (action: string) => {
		switch (action) {
			case "delete":
				modalCtl.confirm(
					`Are you sure you want to delete "${props.room.name}"?`,
					(confirmed) => {
						if (confirmed) {
							ctx.client.http.DELETE("/api/v1/room/{room_id}", {
								params: { path: { room_id: props.room.id } },
							}).then(() => {
								window.location.href = "/";
							}).catch((error) => {
								console.error("Failed to delete room:", error);
								modalCtl.alert("Failed to delete room: " + error.message);
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
				{props.room.id === SERVER_ROOM_ID ? "admin settings" : "room settings"}
				: {currentTab()?.name} <A href={`/room/${props.room.id}`}>back</A>
			</header>
			<nav>
				<ul>
					<For each={groupedTabs()}>
						{(group, groupIdx) => (
							<>
								<div
									class="dim"
									style={{
										"margin-top": groupIdx() === 0 ? "" : "12px",
										"margin": "2px 8px",
									}}
								>
									{group.category}
								</div>
								<For each={group.items}>
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
														href={`/room/${props.room.id}/settings/${tab.path}`}
													>
														{tab.name}
													</A>
												</li>
											</Match>
										</Switch>
									)}
								</For>
							</>
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
