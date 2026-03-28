import { createMemo, For, Show, Switch } from "solid-js";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import {
	Appearance,
	Applications,
	AuditLog,
	Authentication,
	Blocked,
	Chat,
	Connections,
	Data,
	Keybinds,
	Language,
	Notifications,
	Profile,
	Sessions,
	Voice,
} from "./components/features/user_settings/mod.tsx";
import type { User } from "sdk";

// Tab type definitions with proper discriminated unions
type CategoryTab = { category: string };

type PageTab = {
	name: string;
	path: string;
	component: any;
	noPad?: boolean;
};

type TabItem = CategoryTab | PageTab;

// Helper for type-safe matching with SolidJS Switch/Match
function matches<S extends TabItem>(
	e: TabItem,
	predicate: (e: TabItem) => e is S,
): S | false {
	return predicate(e) ? e : false;
}

// Type guard function
function isCategoryTab(tab: TabItem): tab is CategoryTab {
	return "category" in tab;
}

function isPageTab(tab: TabItem): tab is PageTab {
	return "path" in tab;
}

const tabs: TabItem[] = [
	{ category: "account" },
	{ name: "profile", path: "", component: Profile },
	{ name: "authentication", path: "authentication", component: Authentication },
	{ name: "sessions", path: "sessions", component: Sessions },
	{ name: "audit log", path: "audit-log", component: AuditLog },
	{ name: "blocked users", path: "blocks", component: Blocked },
	{ name: "connections", path: "connections", component: Connections },
	{ name: "data", path: "data", component: Data },
	{ category: "application" },
	{ name: "appearance", path: "appearance", component: Appearance },
	{ name: "notifications", path: "notifications", component: Notifications },
	{ name: "voice", path: "voice", component: Voice },
	{ name: "chat", path: "chat", component: Chat },
	{ name: "language", path: "language", component: Language },
	{ name: "keybinds", path: "keybinds", component: Keybinds },
	{ category: "developer" },
	{
		name: "applications",
		path: "applications",
		component: Applications,
		noPad: true,
	},
];

type GroupedTab = {
	category: string;
	items: PageTab[];
};

function groupTabsByCategory(tabs: TabItem[]): GroupedTab[] {
	const groups: GroupedTab[] = [];
	let currentGroup: GroupedTab | null = null;

	for (const tab of tabs) {
		if (isCategoryTab(tab)) {
			currentGroup = { category: tab.category, items: [] };
			groups.push(currentGroup);
		} else if (currentGroup) {
			currentGroup.items.push(tab);
		}
	}

	return groups.filter((g) => g.items.length > 0);
}

export const UserSettings = (props: { user: User; page: string }) => {
	const currentTab = (): PageTab | undefined => {
		const tab = tabs.find((i) => isPageTab(i) && i.path === (props.page ?? ""));
		return tab && isPageTab(tab) ? tab : undefined;
	};
	const groupedTabs = createMemo(() => groupTabsByCategory(tabs));

	return (
		<div class="settings">
			<header>
				user settings <A href="/">home</A>
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
										<li>
											<A href={`/settings/${tab.path}`}>
												{tab.name}
											</A>
										</li>
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
						user={props.user}
					/>
				</Show>
			</main>
		</div>
	);
};
