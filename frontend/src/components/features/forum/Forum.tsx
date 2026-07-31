import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { debounce } from "@solid-primitives/scheduled";
import { A, useNavigate } from "@solidjs/router";
import type { EditorState } from "prosemirror-state";
import type { Channel } from "sdk";
import { useFloating } from "solid-floating-ui";
import {
	createMemo,
	createResource,
	createSignal,
	For,
	onCleanup,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import { uuidv7 } from "uuidv7";
import { useApi, useChannels, usePreferences, useThreads } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown";
import { Search } from "@/atoms/Search";
import { RenderUploadItem } from "@/components/features/chat/Input.tsx";
import { createEditor } from "@/components/features/editor/Editor.tsx";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useModals } from "@/contexts/modal";
import { useUploads } from "@/contexts/uploads.tsx";
import { useMessageSubmit } from "@/hooks/useMessageSubmit.ts";
import { usePermissions } from "@/hooks/usePermissions";
import { flags } from "@/lib/flags";
import { icChevron } from "@/utils/icons";
import { ThreadCard } from "./ThreadCard";
import {
	type Forum2Sort,
	type Forum2View,
	ThreadSorting,
} from "./ThreadSorting";

export const Forum = (props: { channel: Channel }) => {
	const api = useApi();
	const channels2 = useChannels();
	const threads2 = useThreads();
	const [, modalctl] = useModals();
	const room_id = () => props.channel.room_id ?? "";
	const forum_id = () => props.channel.id;
	const prefsService = usePreferences();
	const prefs = prefsService.useRead();

	// Call the appropriate hook based on filter at component level
	const activeThreads = threads2.useListForChannel(forum_id);
	const archivedThreads = threads2.useListArchivedForChannel(forum_id);
	const removedThreads = threads2.useListRemovedForChannel(forum_id);

	const [_bottom, setBottom] = createSignal<Element | undefined>();

	// TODO: Implement proper pagination for threads

	const getActiveThreads = () => {
		const list = activeThreads()?.state.ids || [];
		return list
			.map((id) => channels2.cache.get(id))
			.filter(
				(t): t is Channel =>
					t !== undefined && t.parent_id === props.channel.id,
			)
			.sort((a, b) => (a.id < b.id ? 1 : -1));
	};

	const getArchivedThreads = () => {
		const list = archivedThreads()?.state.ids || [];
		return list
			.map((id) => channels2.cache.get(id))
			.filter(
				(t): t is Channel =>
					t !== undefined && t.parent_id === props.channel.id,
			)
			.sort((a, b) => (a.id < b.id ? 1 : -1));
	};

	const getRemovedThreads = () => {
		const list = removedThreads()?.state.ids || [];
		return list
			.map((id) => channels2.cache.get(id))
			.filter(
				(t): t is Channel =>
					t !== undefined && t.parent_id === props.channel.id,
			)
			.sort((a, b) => (a.id < b.id ? 1 : -1));
	};

	function createThread(room_id: string) {
		modalctl.prompt("name?", (name) => {
			if (!name) return;
			channels2.create(room_id, {
				name,
				parent_id: props.channel.id,
				type:
					props.channel.type === "Ticket" ? "ThreadPrivate" : "ThreadPublic",
			});
		});
	}

	const user = useCurrentUser();
	const user_id = () => user()?.id;
	const perms = usePermissions(user_id, room_id, () => undefined);
	const openInSidebar = () => prefs.frontend.threads_sidebar_forum === "yes";

	// TODO: deduplicate Forum and Forum2 code

	const [menuOpen, setMenuOpen] = createSignal(false);
	const [referenceEl, setReferenceEl] = createSignal<HTMLElement>();
	const [floatingEl, setFloatingEl] = createSignal<HTMLElement>();
	const position = useFloating(referenceEl, floatingEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset(5), flip(), shift()],
		placement: "bottom-end",
	});

	const [sortBy, setSortBy] = createSignal<Forum2Sort>("new");
	const [viewAs, setViewAs] = createSignal<Forum2View>("compact");
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
			const tantivyQuery = `+(${query}) +channel_id:${forum_id()} +subtype: IN [ThreadPublic ThreadPrivate ThreadForum2]`;
			const res = await api.client.http.POST("/api/v1/search/channels", {
				body: {
					query: tantivyQuery,
					limit: 50,
					offset: 0,
				},
			});
			return res.data?.channels ?? [];
		} catch (e) {
			console.error("Search failed", e);
			return [];
		}
	});

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
			return sortThreads(results.filter((t) => t.parent_id === forum_id()));
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

	const [columns, setColumns] = createSignal(1);
	const obs = new ResizeObserver((entries) => {
		for (const e of entries) {
			const width = e.contentBoxSize[0].inlineSize;
			// TODO: don't hardcode this?
			const THREAD_CARD_WIDTH = 240;
			setColumns(Math.floor(width / THREAD_CARD_WIDTH));
		}
	});

	onCleanup(() => obs.disconnect());

	return (
		<div
			class="forum room-home"
			data-forum-view={viewAs()}
			style={{ "--column-count": columns() }}
			ref={(el) => obs.observe(el)}
		>
			<div style="display:flex">
				<div style="flex:1">
					<h2>{props.channel.name}</h2>
					<Show when={props.channel.description}>
						{(desc) => <Markdown content={desc()} />}
					</Show>
				</div>
				<div style="display:flex;flex-direction:column;gap:4px">
					<A
						style="padding: 0 4px"
						href={`/channel/${props.channel.id}/settings`}
					>
						settings
					</A>
				</div>
			</div>
			<Show when={flags.has("thread_quick_create")}>
				<br />
				<QuickCreate channel={props.channel} />
			</Show>
			<br />
			<div class="forum2-header">
				<Search
					placeholder="search threads..."
					value={searchQuery}
					onInput={(s) => {
						setSearchQuery(s);
						debouncedSetSearch(s);
					}}
				/>

				<button
					type="button"
					class="button primary"
					style="margin-left: 8px;border-radius:4px"
					onClick={() => {
						const rid = room_id();
						if (rid) createThread(rid);
					}}
				>
					create thread
				</button>
			</div>
			<div style="display:flex; align-items:center">
				<h3 style="font-size:1rem; margin-top:8px;flex:1">
					{activeThreads()?.state.ids.length ?? "loading"} threads
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
								<ThreadSorting
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

			<ul class="thread-list">
				<For each={threads().active}>
					{(thread) => (
						<li>
							<ThreadCard thread={thread} openInSidebar={openInSidebar()} />
						</li>
					)}
				</For>
			</ul>

			<Show when={threads().archived.length}>
				<h3 class="dim" style="margin-top:16px;">
					older threads
				</h3>
				<ul class="thread-list">
					<For each={threads().archived}>
						{(thread) => (
							<li>
								<ThreadCard thread={thread} openInSidebar={openInSidebar()} />
							</li>
						)}
					</For>
				</ul>
			</Show>

			<div ref={setBottom}></div>
		</div>
	);
};

function hasLastVersionId(
	ch: Channel,
): ch is Channel & { last_version_id: string } {
	return "last_version_id" in ch;
}

// NOTE the room id is reused as the channel id for draft messages and attachments
const QuickCreate = (props: { channel: Channel }) => {
	const channels2 = useChannels();
	const n = useNavigate();
	const channelCtx = useChannel();
	const uploads = useUploads();
	const submit = useMessageSubmit(() => props.channel.id);
	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	return (
		<Show when={channelCtx} fallback={<div>Loading editor...</div>}>
			{(ctx) => {
				const [ch, chUpdate] = ctx();
				const editor = createEditor({
					channelId: () => props.channel.id,
					roomId: () => props.channel.room_id ?? "",
					toolbar,
					autocomplete,
				});

				function uploadFile(e: InputEvent) {
					const target = e.target as HTMLInputElement;
					if (!target.files) return;
					const files = Array.from(target.files);
					for (const file of files) {
						handleUpload(file);
					}
				}

				function handleUpload(file: File) {
					console.log(file);
					const local_id = uuidv7();
					uploads.init(local_id, props.channel.id, file);
				}

				const onSubmit = (text: string) => {
					if (!text) return false;
					const rid = props.channel.room_id;
					if (!rid) return false;
					channels2
						.create(rid, {
							name: "thread",
							parent_id: props.channel.id,
						})
						.then((t) => {
							if (!t) return;
							submit(text, false, t.id);
							n(`/channel/${t.id}`);
						});
					return true;
				};

				const onChange = (state: EditorState) => {
					chUpdate("editor_state", state);
				};

				const atts = () => ch.attachments;
				return (
					<div class="message-input quick-create">
						<div style="margin-bottom: 2px">quick create thread</div>
						<Show when={atts()?.length}>
							<div class="attachments">
								<header>
									{atts()?.length}{" "}
									{atts()?.length === 1 ? "attachment" : "attachments"}
								</header>
								<ul>
									<For each={atts()}>
										{(att) => (
											<RenderUploadItem
												thread_id={props.channel.id}
												att={att}
											/>
										)}
									</For>
								</ul>
							</div>
						</Show>
						<div class="text">
							<label class="upload">
								+
								<input
									multiple
									type="file"
									onInput={uploadFile}
									value="upload file"
								/>
							</label>
							<editor.View
								onSubmit={onSubmit}
								onChange={onChange}
								onUpload={handleUpload}
								placeholder={"send a message..."}
							/>
						</div>
					</div>
				);
			}}
		</Show>
	);
};
