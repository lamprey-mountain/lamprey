import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { debounce } from "@solid-primitives/scheduled";
import { useNavigate } from "@solidjs/router";
import { type Channel, getTimestampFromUUID } from "sdk";
import { useFloating } from "solid-floating-ui";
import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	For,
	onCleanup,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import { useApi, useChannels, usePreferences, useThreads } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown";
import { Search } from "@/atoms/Search";
import { Time } from "@/atoms/Time";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser";
import { useModals } from "@/contexts/modal";
import { usePermissions } from "@/hooks/usePermissions";
import { icChevron } from "@/utils/icons";
import { ChannelIcon } from "../shared/User";
import { DocumentCard } from "./DocumentCard";
import {
	type DocumentSort,
	DocumentSorting,
	type DocumentView,
} from "./DocumentSorting";

// Type guard for Channel with last_version_id
function hasLastVersionId(
	ch: Channel,
): ch is Channel & { last_version_id: string } {
	return "last_version_id" in ch;
}

export const Wiki = (props: { channel: Channel }) => {
	const api2 = useApi();
	const channels2 = useChannels();
	const [, modalctl] = useModals();
	const room_id = () => props.channel.room_id!;
	const wiki_id = () => props.channel.id;

	const [sortBy, setSortBy] = createSignal<DocumentSort>("new");
	const [viewAs, setViewAs] = createSignal<DocumentView>("list");
	const [showRemoved, setShowRemoved] = createSignal(false);
	const [searchQuery, setSearchQuery] = createSignal("");
	const [debouncedSearch, setDebouncedSearch] = createSignal("");

	const debouncedSetSearch = debounce(
		(value: string) => setDebouncedSearch(value),
		300,
	);

	const [searchResults] = createResource(debouncedSearch, async (query) => {
		if (!query.trim()) return [];
		try {
			const tantivyQuery = `+(${query}) +channel_id:${wiki_id()} +subtype: IN [Document]`;
			const res = await api2.client.http.POST("/api/v1/search/channels", {
				body: {
					query: tantivyQuery,
					limit: 50,
					offset: 0,
					// TODO: use selected sorting option
					field: "Id",
				},
			});
			return res.data?.channels ?? [];
		} catch (e) {
			console.error("Search failed", e);
			return [];
		}
	});

	const [menuOpen, setMenuOpen] = createSignal(false);
	const [referenceEl, setReferenceEl] = createSignal<HTMLElement>();
	const [floatingEl, setFloatingEl] = createSignal<HTMLElement>();
	const position = useFloating(referenceEl, floatingEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset(5), flip(), shift()],
		placement: "bottom-end",
	});

	const clickOutside = (e: MouseEvent) => {
		if (
			menuOpen() &&
			referenceEl() &&
			!referenceEl()?.contains(e.target as Node) &&
			floatingEl() &&
			!floatingEl()?.contains(e.target as Node)
		) {
			setMenuOpen(false);
		}
	};

	createEffect(() => {
		if (menuOpen()) {
			document.addEventListener("mousedown", clickOutside);
			onCleanup(() => document.removeEventListener("mousedown", clickOutside));
		}
	});

	const threads2 = useThreads();
	const activeThreads = threads2.useListForChannel(wiki_id);
	const archivedThreads = threads2.useListArchivedForChannel(wiki_id);
	const removedThreads = threads2.useListRemovedForChannel(wiki_id);

	const sortThreads = (items: Channel[]) => {
		return [...items].sort((a, b) => {
			if (sortBy() === "new") {
				return a.id < b.id ? 1 : -1;
			} else if (sortBy() === "activity") {
				const tA = hasLastVersionId(a) ? a.last_version_id : a.id;
				const tB = hasLastVersionId(b) ? b.last_version_id : b.id;
				return tA < tB ? 1 : -1;
			}
			return 0;
		});
	};

	const unorderedThreads = createMemo(() => {
		const query = searchQuery().toLowerCase();
		if (query.length > 0) {
			const results = searchResults() ?? [];
			return sortThreads(results.filter((t) => t.parent_id === wiki_id()));
		}

		const allIds = new Set([
			...(activeThreads()?.state.ids ?? []),
			...(archivedThreads()?.state.ids ?? []),
			...(showRemoved() ? (removedThreads()?.state.ids ?? []) : []),
		]);
		const threads = [...allIds]
			.map((id) => channels2.cache.get(id))
			.filter(
				(t): t is Channel =>
					t !== undefined && t.parent_id === props.channel.id,
			);
		return sortThreads(threads);
	});

	const threads = createMemo(() => {
		const all = unorderedThreads();
		return all.reduce(
			(acc, t) => {
				if (t.archived_at) {
					acc.archived.push(t);
				} else {
					acc.active.push(t);
				}
				return acc;
			},
			{ active: [] as Channel[], archived: [] as Channel[] },
		);
	});

	const [_bottom, setBottom] = createSignal<Element | undefined>();

	// TODO: fetch more when scrolling to bottom
	// createIntersectionObserver(
	// 	() => (bottom() ? [bottom()!] : []),
	// 	(entries) => {
	// 		for (const entry of entries) {
	// 			if (entry.isIntersecting) fetchMore();
	// 		}
	// 	},
	// );

	function createDocument(room_id: string) {
		modalctl.prompt("name?", (name) => {
			if (!name) return;
			channels2.create(room_id, {
				name,
				parent_id: props.channel.id,
				type: "Document",
			});
		});
	}

	const currentUser = useCurrentUser();
	const user_id = () => currentUser()?.id;
	const perms = usePermissions(user_id, room_id, () => undefined);

	const prefsService = usePreferences();
	const prefs = prefsService.useRead();
	const openInSidebar = () => prefs.frontend.threads_sidebar_document === "yes";

	// TODO: make "n documents" text show how many search results there are
	// TODO: reactively update document list

	return (
		<div class="wiki-channel">
			<div class="list">
				<div style="display:flex; align-items:center">
					<Search
						placeholder="search documents..."
						value={searchQuery}
						onInput={(s) => {
							setSearchQuery(s);
							debouncedSetSearch(s);
						}}
					/>
					<button
						type="button"
						class="button primary"
						style="margin-left: 8px;border-radius:4px;white-space:nowrap"
						onClick={() => createDocument(room_id())}
					>
						{/* TODO: add icon */}
						create document
					</button>
				</div>
				<div style="display:flex; align-items:center">
					<h3 style="font-size:1rem; margin-top:8px;flex:1">
						{activeThreads()?.state.ids.length ?? "loading"} documents
					</h3>
					<div class="sort-view-container">
						<button
							type="button"
							class="button sort-view-button"
							ref={setReferenceEl}
							onClick={() => setMenuOpen(!menuOpen())}
							classList={{ selected: menuOpen() }}
						>
							<span>sort and view</span>
							<Icon src={icChevron} />
						</button>
						<Portal>
							<Show when={menuOpen()}>
								<div
									ref={setFloatingEl}
									class="sort-view-menu"
									style={{
										position: position.strategy,
										top: `${position.y ?? 0}px`,
										left: `${position.x ?? 0}px`,
										"z-index": 1000,
									}}
								>
									<DocumentSorting
										sorting={sortBy()}
										view={viewAs()}
										onSort={(s) => {
											setSortBy(s);
											setMenuOpen(false);
										}}
										onView={(v) => {
											setViewAs(v);
											setMenuOpen(false);
										}}
										showRemoved={showRemoved()}
										onToggleRemoved={(s) => {
											setShowRemoved(s);
											setMenuOpen(false);
										}}
										canManage={perms.has("ThreadManage")}
									/>
								</div>
							</Show>
						</Portal>
					</div>
				</div>

				<ul>
					<For each={threads().active}>
						{(thread) => (
							<li>
								<DocumentCard thread={thread} openInSidebar={openInSidebar()} />
							</li>
						)}
					</For>
				</ul>

				<Show when={threads().archived.length}>
					<h3 class="dim" style="margin-top:16px;">
						older documents
					</h3>
					<ul>
						<For each={threads().archived}>
							{(thread) => (
								<li>
									<DocumentCard
										thread={thread}
										openInSidebar={openInSidebar()}
									/>
								</li>
							)}
						</For>
					</ul>
				</Show>

				<div ref={setBottom}></div>
			</div>
		</div>
	);
};
