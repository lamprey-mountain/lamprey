import { autoUpdate, flip, offset, size } from "@floating-ui/dom";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import type { Node } from "prosemirror-model";
import { EditorState, Plugin } from "prosemirror-state";
import type { EditorView } from "prosemirror-view";
import type { User } from "sdk";
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
import type { ChannelSearch } from "@/app/context";
import icSearch from "@/assets/search.png";
import {
	createEditor as createBaseEditor,
	createPlaceholderPlugin,
} from "@/components/features/editor/mod.tsx";
import { useOptionalChannel } from "@/contexts/channel";
import { type RoomSearch, useRoom } from "@/contexts/room";
import type { RoomT, ThreadT } from "@/types";
import {
	autocompletePlugin,
	getFilterFromSelection,
	syntaxHighlightingPlugin,
} from "./plugins";
import { SearchAutocomplete } from "./SearchAutocomplete";
import { schema } from "./schema";
import {
	addRecentSearch,
	dateToBoundaryUUID,
	getRecentSearches,
	parseQueryToNodes,
	serializeToQuery,
} from "./utils";

export const SearchInput = (props: {
	channel?: ThreadT;
	room?: RoomT;
	autofocus?: boolean;
}) => {
	const users2 = useUsers();
	const messagesService = useMessages();
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
	const [activeFilter, setActiveFilter] = createSignal<{
		type: string;
		query: string;
		negated?: boolean;
	} | null>(null);
	const [hoveredIndex, setHoveredIndex] = createSignal<number>(0);
	const [editorRef, setEditorRef] = createSignal<HTMLElement>();
	const [userNavigated, setUserNavigated] = createSignal(false);

	let currentItemsRef: {
		items: { rawValue?: string; onSelect: () => void; isSeparator?: boolean }[];
		selectItem: (idx: number) => void;
	} | null = null;

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
	const channels2 = useChannels();

	const roomThreads = () =>
		[...channels2.cache.values()].filter(
			(c) => c.room_id === (props.channel?.room_id ?? props.room?.id ?? ""),
		);

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

	const handleSubmit = async () => {
		if (!editor.view) return;
		const queryString = serializeToQuery(editor.view.state);
		if (!queryString) {
			updateSearch(undefined);
			return;
		}

		addRecentSearch(queryString);

		const filters: {
			author_ids?: string[];
			not_author_ids?: string[];
			thread_ids?: string[];
			not_thread_ids?: string[];
			before?: string;
			after?: string;
			has?: string[];
			not_has?: string[];
			pinned?: string;
			mentions_ids?: string[];
			not_mentions_ids?: string[];
		} = {};

		const textQueryParts: string[] = [];
		const negatedTextQueryParts: string[] = [];

		editor.view.state.doc.forEach((node) => {
			node.forEach((inlineNode) => {
				if (inlineNode.isText) {
					const text = inlineNode.text?.trim();
					if (text) {
						const words = text.split(/\s+/);
						for (const word of words) {
							if (word.startsWith("-") && word.length > 1) {
								negatedTextQueryParts.push(word.slice(1));
							} else if (word) textQueryParts.push(word);
						}
					}
				} else {
					const type = inlineNode.type.name;
					const negated = inlineNode.attrs.negated;

					if (type === "author") {
						if (negated) {
							if (!filters.not_author_ids) filters.not_author_ids = [];
							filters.not_author_ids.push(inlineNode.attrs.id);
						} else {
							if (!filters.author_ids) filters.author_ids = [];
							filters.author_ids.push(inlineNode.attrs.id);
						}
					} else if (type === "channel") {
						if (negated) {
							if (!filters.not_thread_ids) filters.not_thread_ids = [];
							filters.not_thread_ids.push(inlineNode.attrs.id);
						} else {
							if (!filters.thread_ids) filters.thread_ids = [];
							filters.thread_ids.push(inlineNode.attrs.id);
						}
					} else if (type === "before") {
						filters.before = inlineNode.attrs.date;
					} else if (type === "after") {
						filters.after = inlineNode.attrs.date;
					} else if (type === "has") {
						if (negated) {
							if (!filters.not_has) filters.not_has = [];
							filters.not_has.push(inlineNode.attrs.value);
						} else {
							if (!filters.has) filters.has = [];
							filters.has.push(inlineNode.attrs.value);
						}
					} else if (type === "pinned") {
						filters.pinned = inlineNode.attrs.value;
					} else if (type === "mentions") {
						if (negated) {
							if (!filters.not_mentions_ids) filters.not_mentions_ids = [];
							filters.not_mentions_ids.push(inlineNode.attrs.id);
						} else {
							if (!filters.mentions_ids) filters.mentions_ids = [];
							filters.mentions_ids.push(inlineNode.attrs.id);
						}
					}
				}
			});
		});

		const textQuery = textQueryParts.join(" ");
		const searchState: ChannelSearch = {
			query: queryString,
			results: (currentSearch()?.results as any) ?? null,
			loading: true,
			author: filters.author_ids,
			before: filters.before,
			after: filters.after,
			channel: filters.thread_ids,
		};
		updateSearch(searchState);

		const queryParts: string[] = [];

		if (textQuery.trim()) queryParts.push(`+(${textQuery.trim()})`);
		if (negatedTextQueryParts.length) {
			queryParts.push(`-(${negatedTextQueryParts.join(" ")})`);
		}

		if (filters.author_ids?.length) {
			queryParts.push(`+author_id: IN [${filters.author_ids.join(" ")}]`);
		}
		if (filters.not_author_ids?.length) {
			queryParts.push(`-author_id: IN [${filters.not_author_ids.join(" ")}]`);
		}

		if (props.channel) {
			if (props.channel.type === "Dm" || props.channel.type === "Gdm") {
				queryParts.push(`+channel_id:${props.channel.id}`);
			} else if (filters.thread_ids?.length) {
				queryParts.push(`+channel_id: IN [${filters.thread_ids.join(" ")}]`);
				if (props.channel.room_id) {
					queryParts.push(`+room_id:${props.channel.room_id}`);
				}
			} else if (props.channel.room_id) {
				queryParts.push(`+room_id:${props.channel.room_id}`);
			} else {
				queryParts.push(`+channel_id:${props.channel.id}`);
			}
		} else if (props.room) {
			queryParts.push(`+room_id:${props.room.id}`);
		}

		if (filters.not_thread_ids?.length) {
			queryParts.push(`-channel_id: IN [${filters.not_thread_ids.join(" ")}]`);
		}

		if (filters.before && filters.after) {
			const from_uuid = dateToBoundaryUUID(filters.after, "start");
			const to_uuid = dateToBoundaryUUID(filters.before, "end");
			if (from_uuid && to_uuid) {
				queryParts.push(`+created_at:[${from_uuid} TO ${to_uuid}]`);
			}
		} else if (filters.after) {
			const from_uuid = dateToBoundaryUUID(filters.after, "start");
			if (from_uuid) queryParts.push(`+created_at:[${from_uuid} TO *]`);
		} else if (filters.before) {
			const to_uuid = dateToBoundaryUUID(filters.before, "end");
			if (to_uuid) queryParts.push(`+created_at:[* TO ${to_uuid}]`);
		}

		const mapHas = (hasVals: string[]) => {
			const hasSubquery: string[] = [];
			if (hasVals.includes("attachment")) {
				hasSubquery.push(`metadata_fast.has_attachment:true`);
			}
			if (hasVals.includes("image")) {
				hasSubquery.push(`metadata_fast.has_image:true`);
			}
			if (hasVals.includes("audio")) {
				hasSubquery.push(`metadata_fast.has_audio:true`);
			}
			if (hasVals.includes("video")) {
				hasSubquery.push(`metadata_fast.has_video:true`);
			}
			if (hasVals.includes("link")) {
				hasSubquery.push(`metadata_fast.has_link:true`);
			}
			if (hasVals.includes("embed")) {
				hasSubquery.push(`metadata_fast.has_embed:true`);
			}
			return hasSubquery;
		};

		if (filters.has?.length) {
			const hasSubquery = mapHas(filters.has);
			if (hasSubquery.length === 1) queryParts.push(`+${hasSubquery[0]}`);
			else if (hasSubquery.length > 1) {
				queryParts.push(`+(${hasSubquery.join(" ")})`);
			}
		}

		if (filters.not_has?.length) {
			const notHasSubquery = mapHas(filters.not_has);
			if (notHasSubquery.length === 1) queryParts.push(`-${notHasSubquery[0]}`);
			else if (notHasSubquery.length > 1) {
				queryParts.push(`-(${notHasSubquery.join(" ")})`);
			}
		}

		if (filters.pinned) {
			queryParts.push(`+metadata_fast.pinned:${filters.pinned}`);
		}

		const mapMentions = (mentions: string[]) => {
			const mentionSubquery: string[] = [];
			for (const mentionId of mentions) {
				if (mentionId.startsWith("user-")) {
					mentionSubquery.push(
						`metadata_fast.mentions_user:${mentionId.replace("user-", "")}`,
					);
				} else if (mentionId.startsWith("role-")) {
					mentionSubquery.push(
						`metadata_fast.mentions_role:${mentionId.replace("role-", "")}`,
					);
				} else if (
					mentionId === "everyone-room" ||
					mentionId === "everyone-thread"
				)
					mentionSubquery.push(`metadata_fast.mentions_everyone:true`);
			}
			return mentionSubquery;
		};

		if (filters.mentions_ids?.length) {
			const mentionSubquery = mapMentions(filters.mentions_ids);
			if (mentionSubquery.length === 1) {
				queryParts.push(`+${mentionSubquery[0]}`);
			} else if (mentionSubquery.length > 1) {
				queryParts.push(`+(${mentionSubquery.join(" ")})`);
			}
		}

		if (filters.not_mentions_ids?.length) {
			const notMentionSubquery = mapMentions(filters.not_mentions_ids);
			if (notMentionSubquery.length === 1) {
				queryParts.push(`-${notMentionSubquery[0]}`);
			} else if (notMentionSubquery.length > 1) {
				queryParts.push(`-(${notMentionSubquery.join(" ")})`);
			}
		}

		const body = {
			query: queryParts.join(" ") || undefined,
			sort_order: "desc" as const,
			sort_field: "Created" as const,
			limit: 100,
		};

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

		const match = textBefore.match(
			/-?(author|channel|before|after|has|pinned|mentions):(\S*)$/,
		);
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

	const insertFilter = (text: string) => {
		requestAnimationFrame(() => {
			const view = editor.view;
			if (!view || !view.state || !view.dom?.isConnected) return;

			try {
				const { from } = view.state.selection;
				const $pos = view.state.doc.resolve(from);
				const nodeBefore = $pos.nodeBefore;
				const textBefore = nodeBefore?.isText ? nodeBefore.text! : "";

				const wordMatch = textBefore.match(/(\S+)$/);
				const start = wordMatch ? from - wordMatch[0].length : from;

				const isNegatedSuggestion = text.startsWith("-");
				const cleanText = isNegatedSuggestion ? text.slice(1) : text;

				const recent = getRecentSearches();
				if (
					recent.includes(text) &&
					cleanText.length > 0 &&
					!isNegatedSuggestion &&
					!cleanText.match(/^(author|channel|mentions):/)
				) {
					const nodes = parseQueryToNodes(text, users2, roomThreads);
					const tr = view.state.tr.delete(0, view.state.doc.content.size);
					if (nodes.length > 0) tr.insert(0, nodes);
					view.dispatch(tr);

					setActiveFilter(null);
					setHoveredIndex(0);
					setTimeout(() => {
						if (editor.view?.dom?.isConnected) {
							handleSubmit();
							editor.view.focus();
						}
					}, 50);
					return;
				}

				const filterMatch = cleanText.match(
					/^(author|channel|mentions):(\S*)$/,
				);
				if (filterMatch) {
					const [, type, id] = filterMatch;
					let node: Node | null = null;

					if (type === "author") {
						const user = users2.cache.get(id) as User | undefined;
						if (user) {
							node = schema.nodes.author.create({
								id: user.id,
								name: user.name,
								negated: isNegatedSuggestion,
							});
						}
					} else if (type === "channel") {
						const thread = roomThreads()?.find((t) => t.id === id);
						if (thread) {
							node = schema.nodes.channel.create({
								id: thread.id,
								name: thread.name,
								negated: isNegatedSuggestion,
							});
						}
					} else if (type === "mentions") {
						node = schema.nodes.mentions.create({
							id,
							name: id,
							negated: isNegatedSuggestion,
						});
					}

					if (node) {
						const tr = view.state.tr.replaceWith(start, from, node);
						tr.insertText(" ", tr.mapping.map(from));
						view.dispatch(tr);

						setActiveFilter(null);
						setHoveredIndex(0);
						setTimeout(() => {
							if (editor.view?.dom?.isConnected) editor.view.focus();
						}, 10);
						return;
					}
				}

				// Check if this is a full query string from history (may contain multiple filters)
				if (
					text.match(/\b(author|thread|before|after|has|pinned|mentions):\S+/)
				) {
					const nodes = parseQueryToNodes(text, users2, roomThreads);
					if (nodes.length > 0) {
						const tr = editor.view.state.tr.replaceWith(
							start,
							from,
							nodes as any,
						);
						editor.view.dispatch(tr);
						editor.view.focus();
						setActiveFilter(null);
						return;
					}
				}
			} catch (e) {
				console.warn("insertFilter error:", e);
			}
		});
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

			if (initialSearch?.query) {
				const nodes = parseQueryToNodes(
					initialSearch.query,
					users2,
					roomThreads,
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

								const items = currentItemsRef?.items || [];
								const hasSelectableItems =
									items.length > 0 && items.some((i) => !i.isSeparator);

								if (filterActive && hasSelectableItems) {
									if (event.key === "ArrowDown" || event.key === "ArrowUp") {
										setUserNavigated(true);
										event.preventDefault();
										setHoveredIndex((prev) => {
											const max = items.length - 1;
											let next =
												event.key === "ArrowDown"
													? prev >= max
														? 0
														: prev + 1
													: prev <= 0
														? max
														: prev - 1;

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
										if (
											event.key === "Enter" &&
											filterActive.type === "filter" &&
											!userNavigated()
										) {
											return false;
										}

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
				],
			});
		},
		handleDOMEvents: {
			focus: (view: EditorView) => {
				setActiveFilter(getFilterFromSelection(view.state));
				return false;
			},
			blur: () => {
				setTimeout(() => setActiveFilter(null), 150);
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
							onItemsChange={(its, selectItem) => {
								currentItemsRef = { items: its, selectItem };
							}}
						/>
					</div>
				</Show>
			</Portal>
			<img class="icon" src={icSearch} />
		</div>
	);
};
