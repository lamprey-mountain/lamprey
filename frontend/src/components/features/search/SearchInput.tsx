import { autoUpdate, flip, offset, size } from "@floating-ui/dom";
import { gapCursor } from "prosemirror-gapcursor";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import type { Node } from "prosemirror-model";
import { EditorState, Plugin } from "prosemirror-state";
import type { EditorView, NodeView } from "prosemirror-view";
import type { User } from "sdk";
import { useFloating } from "solid-floating-ui";
import {
	type Accessor,
	createEffect,
	createMemo,
	createSignal,
	getOwner,
	runWithOwner,
	Show,
} from "solid-js";
import { Portal, render } from "solid-js/web";
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
import {
	FILTER_NAMES,
	SEARCH_FILTERS,
	type SearchContext,
} from "./filters.config";
import {
	type ActiveFilter,
	autocompletePlugin,
	getFilterFromSelection,
	syntaxHighlightingPlugin,
} from "./plugins";
import {
	type AutocompleteItem,
	type Completion,
	SearchAutocomplete,
} from "./SearchAutocomplete";
import { FilterChipUI } from "./SearchFilterChip";
import { schema } from "./schema";
import { buildBackendSearchBody } from "./searchCompiler";
import { tokenizeSearch } from "./tokenizer";
import type { LabelPart } from "./types";
import {
	addRecentSearch,
	getRecentSearches,
	parseQueryToNodes,
	serializeToQuery,
} from "./utils";

// ---------------------------------------------------------------------------
// NodeView factory for rendering filter chips inside ProseMirror
// ---------------------------------------------------------------------------

const createFilterNodeView = (
	type: string,
	searchContext: () => SearchContext,
	owner: ReturnType<typeof getOwner>,
	animate: Accessor<boolean>,
) => {
	return (node: Node): NodeView => {
		const dom = document.createElement("span");

		const getProps = () => {
			const id = node.attrs.id || node.attrs.value || node.attrs.date;
			const ctx = searchContext();
			let user: User | undefined;
			let channel: ThreadT | undefined;

			if (type === "author") {
				user = ctx.users.cache.get(id);
			} else if (type === "channel") {
				channel = ctx.roomThreads().find((t) => t.id === id);
			} else if (type === "mentions" && id.startsWith("user-")) {
				user = ctx.users.cache.get(id.replace("user-", ""));
			}

			return {
				type,
				label: node.attrs.name || id,
				user,
				channel,
				negated: node.attrs.negated,
			};
		};

		// Capture props synchronously before rendering
		const props = getProps();
		let currentProps = props;

		const dispose = render(
			() =>
				runWithOwner(owner, () => (
					<FilterChipUI {...currentProps} animate={animate()} />
				)),
			dom,
		);

		return {
			dom,
			update: (newNode: Node) => {
				if (newNode.type !== node.type) return false;
				node = newNode;
				currentProps = getProps();
				return true;
			},
			destroy: () => dispose(),
		};
	};
};

export const SearchInput = (props: {
	channel?: ThreadT;
	room?: RoomT;
	autofocus?: boolean;
}) => {
	const usersStore = useUsers();
	const messagesService = useMessages();
	const owner = getOwner();
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
	const [activeFilter, setActiveFilter] = createSignal<ActiveFilter | null>(
		null,
	);

	const [hoveredIndex, setHoveredIndex] = createSignal<number>(0);
	const [editorRef, setEditorRef] = createSignal<HTMLElement>();
	const [editorFocused, setEditorFocused] = createSignal(false);

	const allFilterSuggestions = [
		"author:",
		"channel:",
		"before:",
		"after:",
		"has:",
		"pinned:",
		"mentions:",
	];

	const filterSuggestions = createMemo(() => {
		const f = activeFilter();
		if (!f) return allFilterSuggestions;
		const query = f.query.toLowerCase();
		const negated = f.negated;
		if (!query) {
			return negated
				? allFilterSuggestions.map((f) => `-${f}`)
				: allFilterSuggestions;
		}

		return allFilterSuggestions
			.filter((f) => f.toLowerCase().includes(query))
			.map((f) => (negated ? `-${f}` : f));
	});

	const recentSearches = createMemo(() => {
		const f = activeFilter();
		if (f?.type === "filter" && f.query === "") {
			return getRecentSearches();
		}
		return [];
	});

	const autocompleteItems = createMemo(() => {
		const f = activeFilter();
		if (!f) return [];
		const type = f.type;
		const result: {
			id: string;
			label: string | LabelPart[];
			rawValue?: string;
			user?: User;
			channel?: ThreadT;
			onSelect: () => void;
			isSeparator?: boolean;
		}[] = [];

		if (type === "filter") {
			// Filter keyword suggestions
			filterSuggestions().forEach((filter) => {
				result.push({
					id: `filter-${filter}`,
					label: filter,
					onSelect: () => handleCompletion({ type: "text", text: filter }),
				});
			});

			// Recent searches
			const searches = recentSearches();
			if (searches.length > 0) {
				result.push({
					id: "recent-separator",
					label: "",
					onSelect: () => {},
					isSeparator: true,
				});
				searches.forEach((search, idx) => {
					result.push({
						id: `recent-${idx}`,
						label: formatRecentSearch(search, searchContext()),
						rawValue: search,
						onSelect: () =>
							handleCompletion({ type: "recent_search", query: search }),
					});
				});
			}
		} else {
			const def = SEARCH_FILTERS[type];
			if (!def) return result;

			const suggestions = def.getSuggestions(f.query, searchContext());

			suggestions.forEach((item) => {
				result.push({
					id: item.id,
					label: item.label,
					user: item.user,
					channel: item.channel,
					onSelect: () => {
						const astNode = {
							type,
							value: item.id.replace(`${type}-`, ""),
							name: item.label,
							negated: f.negated ?? false,
						};
						const pmNode = def.toPMNode(astNode);
						handleCompletion({ type: "node", node: pmNode });
					},
				});
			});
		}

		return result;
	});

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

	const handleCompletion = (c: Completion) => {
		if (c.type === "recent_search") {
			insertFilter(c.query, true, true);
		} else if (c.type === "text") {
			insertFilter(c.text, false, true);
		} else if (c.type === "node") {
			insertNode(c.node, true);
		}
	};

	const insertNode = (node: Node, shouldSubmit: boolean) => {
		const view = editor.view;
		const filter = activeFilter();
		if (!view || !filter) return;

		const tr = view.state.tr.replaceWith(filter.from, filter.to, node);
		tr.insertText(" ", tr.mapping.map(filter.from));
		tr.setMeta("skipAutocomplete", true);
		view.dispatch(tr);

		setActiveFilter(null);
		setHoveredIndex(0);
		view.focus();

		if (shouldSubmit) handleSubmit();
	};

	const insertFilter = (
		text: string,
		isRecent?: boolean,
		shouldSubmit?: boolean,
	) => {
		const view = editor.view;
		const filter = activeFilter();
		if (!view || !filter) return;

		if (isRecent) {
			const ctx = searchContext();
			const nodes = parseQueryToNodes(text, ctx);
			const tr = view.state.tr.delete(0, view.state.doc.content.size);
			if (nodes.length > 0) tr.insert(0, nodes);
			tr.setMeta("skipAutocomplete", true);
			view.dispatch(tr);
			setActiveFilter(null);
			setHoveredIndex(0);
			view.focus();
			handleSubmit();
			return;
		}

		const tr = view.state.tr.insertText(text, filter.from, filter.to);

		if (!text.endsWith(":")) {
			tr.setMeta("skipAutocomplete", true);
			setActiveFilter(null);
		}

		view.dispatch(tr);
		view.focus();
		if (shouldSubmit && !text.endsWith(":")) handleSubmit();
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

	const [autocompleteFocused, setAutocompleteFocused] = createSignal(false);

	const editor = createBaseEditor({
		schema: schema as any,
		nodeViews: () => ({
			author: createFilterNodeView(
				"author",
				searchContext,
				owner,
				editorFocused,
			),
			channel: createFilterNodeView(
				"channel",
				searchContext,
				owner,
				editorFocused,
			),
			mentions: createFilterNodeView(
				"mentions",
				searchContext,
				owner,
				editorFocused,
			),
		}),
		createState: (schema) => {
			let docContent: Node | undefined;
			const initialSearch = currentSearch();
			const ctx = searchContext();

			if (initialSearch?.query) {
				const nodes = parseQueryToNodes(initialSearch.query, ctx);
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
							handleKeyDown(view, event) {
								const items = autocompleteItems();
								const f = activeFilter();

								if (f) {
									if (items.length > 0) {
										if (event.key === "ArrowDown") {
											setHoveredIndex((prev) => (prev + 1) % items.length);
											return true;
										} else if (event.key === "ArrowDown") {
											setHoveredIndex((prev) => (prev * 2 - 1) % items.length);
											return true;
										} else if (event.key === "Tab") {
											const item = items[hoveredIndex()];
											item.onSelect();
											return true;
										} else if (event.key === "Enter") {
											const item = items[hoveredIndex()];
											item.onSelect();
											handleSubmit();
											return true;
										}
									}

									if (event.key === "Escape") {
										event.preventDefault();
										setActiveFilter(null);
										return true;
									}
								}

								if (event.key === "Enter" && !event.shiftKey) {
									handleSubmit();
									return true;
								} else if (event.key === "Escape") {
									event.preventDefault();
									const { state } = view;
									if (
										state.doc.textContent.length > 0 ||
										state.doc.childCount > 1 ||
										(state.doc.firstChild &&
											state.doc.firstChild.childCount > 0)
									) {
										const tr = state.tr.delete(0, state.doc.content.size);
										view.dispatch(tr);
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
				setEditorFocused(true);
				return false;
			},
			blur: () => {
				if (autocompleteFocused()) {
					setActiveFilter({ type: "filter", query: "", from: 1, to: 1 });
				} else {
					setActiveFilter(null);
				}
				setEditorFocused(false);
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
			<img class="icon" src={icSearch} alt="" aria-hidden="true" />
			<Portal mount={document.getElementById("overlay")!}>
				<Show when={true}>
					<div
						ref={setDropdownRef}
						class="floating"
						style={{
							position: position.strategy,
							top: `${position.y ?? 0}px`,
							left: `${position.x ?? 0}px`,
							// width: `${(editorRef()?.offsetWidth || 300) * 2}px`,
							// TODO: handle responsive ui
							width: "600px",
						}}
					>
						<SearchAutocomplete
							filter={activeFilter()!}
							channel={props.channel}
							room={props.room}
							onCompletion={handleCompletion}
							hoveredIndex={hoveredIndex()}
							setHoveredIndex={setHoveredIndex}
							searchContext={searchContext()}
							onPointerDown={() => setAutocompleteFocused(true)}
							onBlur={() => setAutocompleteFocused(false)}
							autocompleteItems={autocompleteItems()}
							filterSuggestions={filterSuggestions()}
							recentSearches={recentSearches()}
						/>
					</div>
				</Show>
			</Portal>
		</div>
	);
};

function formatRecentSearch(query: string, ctx: SearchContext): LabelPart[] {
	const tokens = tokenizeSearch(query);
	const parts: LabelPart[] = [];
	let lastTo = 0;

	for (const token of tokens) {
		// 1. Push plain text between tokens
		if (token.from > lastTo) {
			parts.push(query.slice(lastTo, token.from));
		}
		lastTo = token.to;

		// 2. Handle Text/Phrases
		if (token.type !== "filter") {
			parts.push(token.value);
			continue;
		}

		// 3. Handle Filters cleanly using the registry
		const def = SEARCH_FILTERS[token.filterType];
		if (def && def.resolveDisplayData) {
			const resolved = def.resolveDisplayData(token.value, ctx);
			parts.push({
				type: token.filterType,
				value: resolved.name ?? token.value,
				user: resolved.user,
				channel: resolved.channel,
				negated: token.negated,
				parts: [], // Triggers FilterChipUI
			});
		} else {
			// Fallback for simple filters like has:image
			parts.push({
				type: token.filterType,
				value: token.value,
				negated: token.negated,
				parts: [],
			});
		}
	}

	if (lastTo < query.length) parts.push(query.slice(lastTo));
	return parts;
}
