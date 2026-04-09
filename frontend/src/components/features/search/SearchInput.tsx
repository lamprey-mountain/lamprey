import { autoUpdate, flip, offset, size } from "@floating-ui/dom";
import { gapCursor } from "prosemirror-gapcursor";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import type { Node } from "prosemirror-model";
import { EditorState, Plugin } from "prosemirror-state";
import type { EditorView } from "prosemirror-view";
import { useFloating } from "solid-floating-ui";
import { createEffect, createMemo, createSignal, Show } from "solid-js";
import { Portal } from "solid-js/web";
import {
	useChannels,
	useMessages,
	useRoles,
	useRoomMembers,
	useThreadMembers,
	useUsers,
} from "@/api";
import icSearch from "@/assets/search.png";
import {
	createEditor as createBaseEditor,
	createPlaceholderPlugin,
} from "@/components/features/editor/mod.tsx";
import { useOptionalChannel } from "@/contexts/channel";
import { type RoomSearch, useRoom } from "@/contexts/room";
import type { RoomT, ThreadT } from "@/types";
import type { ChannelSearch } from "@/types/chat";
import { FILTER_NAMES } from "./filters.config";
import {
	autocompletePlugin,
	getFilterFromSelection,
	syntaxHighlightingPlugin,
} from "./plugins";
import { SearchAutocomplete } from "./SearchAutocomplete";
import { schema } from "./schema";
import { buildBackendSearchBody } from "./searchCompiler";
import { addRecentSearch, parseQueryToNodes, serializeToQuery } from "./utils";

export const SearchInput = (props: {
	channel?: ThreadT;
	room?: RoomT;
	autofocus?: boolean;
}) => {
	const usersStore = useUsers();
	const messagesService = useMessages();
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
	const [activeFilter, setActiveFilter] = createSignal<{
		type: string;
		query: string;
		negated?: boolean;
	} | null>(null);
	const [hoveredIndex, setHoveredIndex] = createSignal<number>(0);
	const [editorRef, setEditorRef] = createSignal<HTMLElement>();

	let currentItemsRef: {
		items: { onSelect: () => void; isSeparator?: boolean }[];
		selectItem: (idx: number) => void;
	} | null = null;

	// Shared context object passed to autocomplete suggestions
	const searchContext = createMemo(() => {
		const channelsStore = useChannels();
		const roomThreads = () =>
			[...channelsStore.cache.values()].filter(
				(c) => c.room_id === (props.channel?.room_id ?? props.room?.id ?? ""),
			);
		return {
			users: usersStore,
			channels: channelsStore,
			roomMembers: useRoomMembers(),
			threadMembers: useThreadMembers(),
			roles: useRoles(),
			roomThreads,
			roomId: props.channel?.room_id ?? props.room?.id ?? null,
			channel: props.channel,
			room: props.room,
		};
	});

	const position = useFloating(editorRef, dropdownRef, {
		whileElementsMounted: autoUpdate,
		middleware: [
			offset(4),
			flip(),
			size({
				padding: 16,
				apply({ availableHeight, elements }) {
					Object.assign(elements.floating.style, {
						maxHeight: `${Math.max(0, availableHeight)}px`,
					});
				},
			}),
		],
		placement: "bottom-start",
	});

	const channelCtx = useOptionalChannel();
	const roomCtx = useRoom();

	const currentSearch = () => {
		if (props.channel) return channelCtx[0]?.search;
		if (props.room) return roomCtx?.[0].search;
		return undefined;
	};

	const updateSearch = (val: ChannelSearch | RoomSearch | undefined) => {
		if (props.channel && channelCtx[1]) channelCtx[1]("search", val as any);
		else if (props.room && roomCtx) roomCtx[1]("search", val as any);
	};

	createEffect(() => {
		const search = currentSearch();
		const view = editor.view;
		if (!view) return;

		if (!search && view.state.doc.content.size > 0) {
			const tr = view.state.tr.delete(0, view.state.doc.content.size);
			tr.setMeta("skipAutocomplete", true);
			view.dispatch(tr);
		}
	});

	/**
	 * Build the backend query using the compiler and execute the search.
	 * All backend-specific logic lives in `buildBackendSearchBody`.
	 */
	const handleSubmit = async () => {
		if (!editor.view) return;
		const queryString = serializeToQuery(editor.view.state);
		if (!queryString) {
			updateSearch(undefined);
			return;
		}

		addRecentSearch(queryString);

		const searchState: ChannelSearch = {
			query: queryString,
			results: (currentSearch()?.results as any) ?? null,
			loading: true,
			// Keep legacy fields for backward compat
			author: undefined,
			before: undefined,
			after: undefined,
			channel: undefined,
		};
		updateSearch(searchState);

		const body = buildBackendSearchBody(editor.view.state, {
			channel: props.channel,
			room: props.room,
		}) as unknown as Record<string, unknown>;

		const res = await messagesService.search(body);
		updateSearch({ ...searchState, results: res || null, loading: false });
	};

	const insertNode = (node: Node) => {
		const view = editor.view;
		if (!view) return;
		const { from } = view.state.selection;
		const textBefore = view.state.doc.textBetween(
			Math.max(0, from - 100),
			from,
			" ",
		);

		const filterRegex = new RegExp(`-?(${FILTER_NAMES.join("|")}):(\\S*)$`);
		const match = textBefore.match(filterRegex);
		if (match) {
			const start = from - match[0].length;
			const tr = view.state.tr.replaceWith(start, from, node);
			tr.insertText(" ", tr.mapping.map(from));
			view.dispatch(tr);
		}

		setActiveFilter(null);
		setHoveredIndex(0);
		view.focus();
	};

	const insertFilter = (text: string, isRecent?: boolean) => {
		const view = editor.view;
		if (!view) return;

		try {
			const { from } = view.state.selection;
			const $pos = view.state.doc.resolve(from);
			const nodeBefore = $pos.nodeBefore;
			const textBefore = nodeBefore?.isText ? nodeBefore.text! : "";

			const wordMatch = textBefore.match(/(\S+)$/);
			const start = wordMatch ? from - wordMatch[0].length : from;

			if (isRecent) {
				const ctx = searchContext();
				const nodes = parseQueryToNodes(text, ctx.users, ctx.roomThreads);
				const tr = view.state.tr.delete(0, view.state.doc.content.size);
				if (nodes.length > 0) tr.insert(0, nodes);
				view.dispatch(tr);

				setActiveFilter(null);
				setHoveredIndex(0);
				view.focus();
				handleSubmit();
				return;
			}

			// insert the text (e.g., "has:")
			const tr = view.state.tr.replaceWith(
				start,
				from,
				view.state.schema.text(text),
			);
			view.dispatch(tr);

			// keep the menu open we inserted a filter trigger
			if (text.endsWith(":")) {
				setHoveredIndex(0);
			} else {
				setActiveFilter(null);
				setHoveredIndex(0);
			}

			view.focus();
		} catch (e) {
			console.warn("insertFilter error:", e);
		}
	};

	createEffect(() => {
		activeFilter();
		setHoveredIndex(0);
	});

	createEffect(() => {
		if (!activeFilter()) return;
		const items = dropdownRef()?.querySelectorAll(".autocomplete-item");
		if (items?.[hoveredIndex()]) {
			items[hoveredIndex()].scrollIntoView({ block: "nearest" });
		}
	});

	const editor = createBaseEditor({
		schema: schema as any,
		createState: (schema) => {
			let docContent: Node | undefined;
			const initialSearch = currentSearch();
			const ctx = searchContext();

			if (initialSearch?.query) {
				const nodes = parseQueryToNodes(
					initialSearch.query,
					ctx.users,
					ctx.roomThreads,
				);
				if (nodes.length > 0) {
					docContent = schema.nodes.doc.create(undefined, [
						schema.nodes.paragraph.create(undefined, nodes),
					]);
				}
			}

			return EditorState.create({
				schema: schema as any,
				doc: docContent,
				plugins: [
					createPlaceholderPlugin(),
					history(),
					keymap({
						"Ctrl-z": undo,
						"Ctrl-Shift-z": redo,
					}),
					syntaxHighlightingPlugin(),
					autocompletePlugin((filter) => {
						if (filter && editor.view && !editor.view.hasFocus()) return;
						setActiveFilter(filter);
					}),
					new Plugin({
						props: {
							handleKeyDown(_view, event) {
								const filterActive = activeFilter();

								if (filterActive) {
									const items = currentItemsRef?.items || [];

									if (event.key === "ArrowDown" || event.key === "ArrowUp") {
										event.preventDefault();
										setHoveredIndex((prev) => {
											const max = items.length - 1;
											if (max < 0) return prev;
											let next =
												event.key === "ArrowDown"
													? prev >= max
														? 0
														: prev + 1
													: prev <= 0
														? max
														: prev - 1;

											// Skip separators
											if (items[next]?.isSeparator) {
												next =
													event.key === "ArrowDown"
														? next >= max
															? 0
															: next + 1
														: next <= 0
															? max
															: next - 1;
											}
											return next;
										});
										return true;
									}

									if (event.key === "Enter" || event.key === "Tab") {
										event.preventDefault();
										if (currentItemsRef) {
											currentItemsRef.selectItem(hoveredIndex());
										}
										return true;
									}

									if (event.key === "Escape") {
										event.preventDefault();
										setActiveFilter(null);
										return true;
									}
								} else {
									if (event.key === "Enter" && !event.shiftKey) {
										event.preventDefault();
										handleSubmit();
										return true;
									}
									if (event.key === "Escape") {
										event.preventDefault();
										const { state } = _view;
										if (
											state.doc.textContent.length > 0 ||
											state.doc.childCount > 1 ||
											(state.doc.firstChild &&
												state.doc.firstChild.childCount > 0)
										) {
											const tr = state.tr.delete(0, state.doc.content.size);
											_view.dispatch(tr);
											handleSubmit();
										} else {
											if (currentSearch()) {
												updateSearch(undefined);
											} else {
												const chatInput = document.querySelector(
													".chat .ProseMirror",
												) as HTMLInputElement | null;
												chatInput?.focus();
											}
										}
										return true;
									}
								}
								return false;
							},
						},
					}),
					// doesnt seem to do anything?
					gapCursor(),
				],
			});
		},
		handleDOMEvents: {
			focus: (view: EditorView) => {
				setActiveFilter(getFilterFromSelection(view.state));
				return false;
			},
			blur: () => {
				setActiveFilter(null);
				return false;
			},
		},
		autofocus: props.autofocus ?? !!currentSearch(),
	});

	return (
		<div class="search-container">
			<div class="search-input" ref={setEditorRef}>
				<editor.View placeholder="search" />
			</div>
			<Portal>
				<Show when={activeFilter()}>
					<div
						ref={setDropdownRef}
						class="floating"
						style={{
							position: position.strategy,
							top: `${position.y ?? 0}px`,
							left: `${position.x ?? 0}px`,
							width: `${editorRef()?.offsetWidth || 0}px`,
						}}
					>
						<SearchAutocomplete
							filter={activeFilter()!}
							channel={props.channel}
							room={props.room}
							onSelect={insertNode}
							onSelectFilter={insertFilter}
							hoveredIndex={hoveredIndex()}
							setHoveredIndex={setHoveredIndex}
							searchContext={searchContext()}
							onItemsChange={(its, selectItem) => {
								currentItemsRef = { items: its, selectItem };
							}}
						/>
					</div>
				</Show>
			</Portal>
			<img class="icon" src={icSearch} alt="" aria-hidden="true" />
		</div>
	);
};
