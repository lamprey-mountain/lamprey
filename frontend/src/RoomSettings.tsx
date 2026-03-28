import { useCurrentUser } from "./contexts/currentUser.tsx";
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
} from "./components/features/room_settings/mod.tsx";
import * as Admin from "./components/features/admin_settings/mod.tsx";
import { Permission, SERVER_ROOM_ID } from "sdk";
import { A, useNavigate } from "@solidjs/router";
import { useCtx } from "./context.ts";
import { useApi2 } from "@/api";
import { useModals } from "./contexts/modal.tsx";
import { usePermissions } from "./hooks/usePermissions.ts";
import { flags } from "./flags.ts";

// Tab type definitions with proper discriminated unions
type CategoryTab = { category: string };

type ActionTab = {
	name: string;
	action: "delete";
	style?: "danger";
	permissionCheck?: (p: ReturnType<typeof usePermissions>) => boolean;
	ownerOnly?: boolean;
};

type PageTab = {
	name: string;
	path: string;
	noPad?: boolean;
	component: Component<any>;
	permissionCheck?: (p: ReturnType<typeof usePermissions>) => boolean;
	ownerOnly?: boolean;
};

type TabItem = CategoryTab | ActionTab | PageTab;

// Helper for type-safe matching with SolidJS Switch/Match
function matches<S extends TabItem>(
	e: TabItem,
	predicate: (e: TabItem) => e is S,
): S | false {
	return predicate(e) ? e : false;
}

// Type guard functions
function isCategoryTab(tab: TabItem): tab is CategoryTab {
	return "category" in tab;
}

function isActionTab(tab: TabItem): tab is ActionTab {
	return "action" in tab;
}

function isPageTab(tab: TabItem): tab is PageTab {
	return "path" in tab;
}

// TODO: more permission checks
const tabs: TabItem[] = [
	{ category: "overview" },
	{ name: "info", path: "", component: Info },
	{
		name: "analytics",
		path: "analytics",
		component: Analytics,
		permissionCheck: (p) => p.has("AnalyticsView"),
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
		permissionCheck: (p) => p.has("AuditLogView"),
	},
	{
		name: "delete room",
		action: "delete",
		style: "danger",
		ownerOnly: true,
	},
];

const todo = (_what: string) => null as unknown as Component<any>;

const adminTabs: TabItem[] = [
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
		permissionCheck: (p) => p.has("AuditLogView"),
	},
];

type GroupedTab = {
	category: string;
	items: (ActionTab | PageTab)[];
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
		if (isCategoryTab(tab)) {
			currentGroup = { category: tab.category, items: [] };
			groups.push(currentGroup);
		} else if (currentGroup) {
			const isVisible =
				(!tab.permissionCheck || tab.permissionCheck(perms)) &&
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
	const api2 = useApi2();
	const [, modalCtl] = useModals();
	const currentUser = useCurrentUser();
	const user_id = () => currentUser()?.id;
	const perms = usePermissions(
		user_id,
		() => props.room.id,
		() => undefined,
	);
	const currentTabs = () => props.room.id === SERVER_ROOM_ID ? adminTabs : tabs;
	const currentTab = () =>
		currentTabs().find((i): i is PageTab => isPageTab(i) && i.path === (props.page ?? ""));

	const groupedTabs = createMemo(() =>
		groupTabsByCategory(currentTabs(), perms, user_id, props.room)
	);

	const nav = useNavigate();

	const handleAction = (action: string) => {
		switch (action) {
			case "delete":
				modalCtl.confirm(
					`Are you sure you want to delete "${props.room.name}"?`,
					(confirmed) => {
						if (confirmed) {
							api2.client.http.DELETE("/api/v1/room/{room_id}", {
								params: { path: { room_id: props.room.id } },
							}).then(() => {
								nav("/");
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
				: {currentTab()?.name}{" "}
				<A href={`/room/${props.room.id}`}>back</A>
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
											<Match when={matches(tab, isActionTab)}>
												{(item) => (
													<li>
														<button
															class="action"
															onClick={() => handleAction(item().action)}
															classList={{
																"danger": item().style === "danger",
															}}
														>
															{item().name}
														</button>
													</li>
												)}
											</Match>
											<Match when={matches(tab, isPageTab)}>
												{(item) => (
													<li>
														<A
															href={`/room/${props.room.id}/settings/${item().path}`}
														>
															{item().name}
														</A>
													</li>
												)}
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
