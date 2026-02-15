import { Channel, getTimestampFromUUID } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	onCleanup,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import { useCtx } from "./context";
import { useApi } from "./api";
import { Time } from "./Time";
import { useModals } from "./contexts/modal";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { usePermissions } from "./hooks/usePermissions";
import { md } from "./markdown";
import { flags } from "./flags";
import { ChannelContext, createInitialChannelState } from "./channelctx";
import { createStore } from "solid-js/store";
import { Document } from "./Document";

export const Wiki = (props: { channel: Channel }) => {
	const ctx = useCtx();
	const api = useApi();
	const [, modalctl] = useModals();
	const room_id = () => props.channel.room_id!;
	const wiki_id = () => props.channel.id;

	const [documentFilter, setDocumentFilter] = createSignal("active");
	const [sortBy, setSortBy] = createSignal<
		"new" | "activity" | "reactions:+1" | "random" | "hot" | "hot2"
	>("new");
	const [viewAs, setViewAs] = createSignal<"list" | "gallery">("list");
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
			!referenceEl()!.contains(e.target as Node) &&
			floatingEl() &&
			!floatingEl()!.contains(e.target as Node)
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

	const fetchMore = () => {
		const filter = documentFilter();
		// Assuming threads API handles generic child channels or we use it for documents too
		if (filter === "active") {
			return api.threads.listForChannel(wiki_id);
		} else if (filter === "archived") {
			return api.threads.listArchivedForChannel(wiki_id);
		} else if (filter === "removed") {
			return api.threads.listRemovedForChannel(wiki_id);
		}
	};

	const documentsResource = createMemo(fetchMore);

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(
		() => (bottom() ? [bottom()!] : []),
		(entries) => {
			for (const entry of entries) {
				if (entry.isIntersecting) fetchMore();
			}
		},
	);

	const getDocuments = () => {
		const items = documentsResource()?.()?.items;
		if (!items) return [];
		// sort descending by id
		return [...items].filter((t) => t.parent_id === props.channel.id).sort((
			a,
			b,
		) => {
			if (sortBy() === "new") {
				return a.id < b.id ? 1 : -1;
			} else if (sortBy() === "activity") {
				// activity
				const tA = a.last_version_id ?? a.id;
				const tB = b.last_version_id ?? b.id;
				return tA < tB ? 1 : -1;
			}
			return 0;
		});
	};

	function createDocument(room_id: string) {
		modalctl.prompt("name?", (name) => {
			if (!name) return;
			api.channels.create(room_id, {
				name,
				parent_id: props.channel.id,
				type: "Document",
			});
		});
	}

	const user_id = () => api.users.cache.get("@self")?.id;
	const perms = usePermissions(user_id, room_id, () => undefined);

	const [documentId, setDocumentId] = createSignal<null | string>(null);

	const getOrCreateChannelContext = (channelId: string) => {
		if (!ctx.channel_contexts.has(channelId)) {
			const store = createStore(createInitialChannelState());
			ctx.channel_contexts.set(channelId, store);
		}
		return ctx.channel_contexts.get(channelId)!;
	};

	return (
		<div class="forum2">
			<div class="list">
				<Show when={flags.has("thread_quick_create")}>
					<br />
					{/* <QuickCreate channel={props.channel} /> */}
					<br />
				</Show>
				<div style="display:flex; align-items:center">
					<input
						placeholder="search documents"
						type="search"
						class="search-pad"
					/>
					<button
						class="primary"
						style="margin-left: 8px;border-radius:4px"
						onClick={() => createDocument(room_id())}
					>
						create document
					</button>
				</div>
				<div style="display:flex; align-items:center">
					<h3 style="font-size:1rem; margin-top:8px;flex:1">
						{getDocuments().length} {documentFilter()} documents
					</h3>
					<div class="sort-view-container">
						<button
							ref={setReferenceEl}
							onClick={() => setMenuOpen(!menuOpen())}
							class="secondary sort-view-button"
							classList={{ selected: menuOpen() }}
						>
							<span>sort and view</span>
							<svg
								width="10"
								height="6"
								viewBox="0 0 10 6"
								fill="none"
								xmlns="http://www.w3.org/2000/svg"
							>
								<path
									d="M1 1L5 5L9 1"
									stroke="currentColor"
									stroke-width="1.5"
									stroke-linecap="round"
									stroke-linejoin="round"
								/>
							</svg>
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
									<menu>
										<div class="subtext header">
											sort by
										</div>
										<button
											onClick={() => {
												setSortBy("new");
												setMenuOpen(false);
											}}
											class="menu-item"
										>
											Newest documents first
											<Show when={sortBy() === "new"}>
												<span>✓</span>
											</Show>
										</button>
										<button
											onClick={() => {
												setSortBy("activity");
												setMenuOpen(false);
											}}
											class="menu-item"
										>
											Recently active documents
											<Show when={sortBy() === "activity"}>
												<span>✓</span>
											</Show>
										</button>
										<button
											onClick={() => {
												setSortBy("reactions:+1");
												setMenuOpen(false);
											}}
											class="menu-item"
										>
											Expected to be helpful
											<Show when={sortBy() === "reactions:+1"}>
												<span>✓</span>
											</Show>
										</button>
										<button
											onClick={() => {
												setSortBy("random");
												setMenuOpen(false);
											}}
											class="menu-item"
										>
											Random ordering
											<Show when={sortBy() === "random"}>
												<span>✓</span>
											</Show>
										</button>
										<button
											onClick={() => {
												setSortBy("hot");
												setMenuOpen(false);
											}}
											class="menu-item"
										>
											Hot
											<Show when={sortBy() === "hot"}>
												<span>✓</span>
											</Show>
										</button>
										<button
											onClick={() => {
												setSortBy("hot2");
												setMenuOpen(false);
											}}
											class="menu-item"
										>
											Hot 2
											<Show when={sortBy() === "hot2"}>
												<span>✓</span>
											</Show>
										</button>
										<hr />
										<div class="subtext header">
											view as
										</div>
										<button
											onClick={() => {
												setViewAs("list");
												setMenuOpen(false);
											}}
											class="menu-item"
										>
											List
											<Show when={viewAs() === "list"}>
												<span>✓</span>
											</Show>
										</button>
										<button
											onClick={() => {
												setViewAs("gallery");
												setMenuOpen(false);
											}}
											class="menu-item"
										>
											Gallery
											<Show when={viewAs() === "gallery"}>
												<span>✓</span>
											</Show>
										</button>
									</menu>
								</div>
							</Show>
						</Portal>
					</div>
					<div class="filters">
						<button
							classList={{ selected: documentFilter() === "active" }}
							onClick={[setDocumentFilter, "active"]}
						>
							active
						</button>
						<button
							classList={{ selected: documentFilter() === "archived" }}
							onClick={[setDocumentFilter, "archived"]}
						>
							archived
						</button>
						<Show when={perms.has("ThreadManage")}>
							<button
								classList={{ selected: documentFilter() === "removed" }}
								onClick={[setDocumentFilter, "removed"]}
							>
								removed
							</button>
						</Show>
					</div>
				</div>
				<ul>
					<For each={getDocuments()}>
						{(doc) => (
							<li>
								<article
									class="thread menu-thread thread-card"
									data-thread-id={doc.id}
								>
									<header onClick={() => setDocumentId(doc.id)}>
										<div class="top">
											<div class="icon"></div>
											<div class="spacer">{doc.name}</div>
											<div class="time">
												Created <Time date={getTimestampFromUUID(doc.id)} />
											</div>
										</div>
										<div
											class="bottom"
											onClick={() => setDocumentId(doc.id)}
										>
											<div class="dim">
												{doc.message_count} message(s) &bull; last update{" "}
												<Time
													date={getTimestampFromUUID(
														doc.last_version_id ?? doc.id,
													)}
												/>
											</div>
											<Show when={doc.description}>
												<div
													class="description markdown"
													innerHTML={md(doc.description ?? "") as string}
												>
												</div>
											</Show>
										</div>
									</header>
								</article>
							</li>
						)}
					</For>
				</ul>
				<div ref={setBottom}></div>
			</div>
			<Show when={documentId()}>
				{(did) => {
					const documentChannel = api.channels.cache.get(did());
					if (!documentChannel) return;
					const docCtx = getOrCreateChannelContext(did());
					return (
						<ChannelContext.Provider value={docCtx}>
							<Document channel={documentChannel} />
						</ChannelContext.Provider>
					);
				}}
			</Show>
		</div>
	);
};
