import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import {
	useApi2,
	useChannels2,
	useMessages2,
	useRoles2,
	useRoomMembers2,
	useThreadMembers2,
	useUsers2,
} from "@/api";
import type { RoomT, ThreadT } from "../../../types";
import type { ChannelSearch } from "../../../context";
import { User } from "sdk";
import { UUID } from "uuidv7";
import { EditorState, Plugin } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";
import { Node, Schema } from "prosemirror-model";
import { keymap } from "prosemirror-keymap";
import { history, redo, undo } from "prosemirror-history";
import { Portal } from "solid-js/web";
import { autoUpdate, flip, offset } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import icSearch from "../../../assets/search.png";
import { useChannel } from "../../../channelctx";
import { RoomSearch, useRoom } from "../../../contexts/room";
import { createEditor as createBaseEditor } from "../editor/mod.tsx";

const schema = new Schema({
	nodes: {
		doc: {
			content: "paragraph",
		},
		paragraph: {
			content: "inline*",
			group: "block",
			toDOM: () => ["p", 0],
		},
		text: {
			group: "inline",
		},
		author: {
			group: "inline",
			inline: true,
			atom: true,
			attrs: { id: { default: "" }, name: { default: "" } },
			toDOM: (node) => [
				"span",
				{ class: "filter-author", "data-id": node.attrs.id },
				["span", { class: "filter-prefix" }, "author:"],
				["span", { class: "filter-value" }, node.attrs.name],
			],
			parseDOM: [
				{
					tag: "span.filter-author",
					getAttrs: (dom: HTMLElement) => ({
						id: dom.dataset.id,
						name: dom.textContent?.replace(/^author:/, "") ?? "",
					}),
				},
			],
		},
		thread: {
			group: "inline",
			inline: true,
			atom: true,
			attrs: { id: { default: "" }, name: { default: "" } },
			toDOM: (node) => [
				"span",
				{ class: "filter-thread", "data-id": node.attrs.id },
				["span", { class: "filter-prefix" }, "thread:"],
				["span", { class: "filter-value" }, node.attrs.name],
			],
			parseDOM: [
				{
					tag: "span.filter-thread",
					getAttrs: (dom: HTMLElement) => ({
						id: dom.dataset.id,
						name: dom.textContent?.replace(/^thread:/, "") ?? "",
					}),
				},
			],
		},
		before: {
			group: "inline",
			inline: true,
			atom: true,
			attrs: { date: { default: "" } },
			toDOM: (node) => [
				"span",
				{ class: "filter-before" },
				["span", { class: "filter-prefix" }, "before:"],
				["span", { class: "filter-value" }, node.attrs.date],
			],
			parseDOM: [
				{
					tag: "span.filter-before",
					getAttrs: (dom: HTMLElement) => ({
						date: dom.textContent?.replace(/^before:/, "") ?? "",
					}),
				},
			],
		},
		after: {
			group: "inline",
			inline: true,
			atom: true,
			attrs: { date: { default: "" } },
			toDOM: (node) => [
				"span",
				{ class: "filter-after" },
				["span", { class: "filter-prefix" }, "after:"],
				["span", { class: "filter-value" }, node.attrs.date],
			],
			parseDOM: [
				{
					tag: "span.filter-after",
					getAttrs: (dom: HTMLElement) => ({
						date: dom.textContent?.replace(/^after:/, "") ?? "",
					}),
				},
			],
		},
		has: {
			group: "inline",
			inline: true,
			atom: true,
			attrs: { value: { default: "" } },
			toDOM: (node) => [
				"span",
				{ class: "filter-has" },
				["span", { class: "filter-prefix" }, "has:"],
				["span", { class: "filter-value" }, node.attrs.value],
			],
			parseDOM: [
				{
					tag: "span.filter-has",
					getAttrs: (dom: HTMLElement) => ({
						value: dom.textContent?.replace(/^has:/, "") ?? "",
					}),
				},
			],
		},
		pinned: {
			group: "inline",
			inline: true,
			atom: true,
			attrs: { value: { default: "" } },
			toDOM: (node) => [
				"span",
				{ class: "filter-pinned" },
				["span", { class: "filter-prefix" }, "pinned:"],
				["span", { class: "filter-value" }, node.attrs.value],
			],
			parseDOM: [
				{
					tag: "span.filter-pinned",
					getAttrs: (dom: HTMLElement) => ({
						value: dom.textContent?.replace(/^pinned:/, "") ?? "",
					}),
				},
			],
		},
		mentions: {
			group: "inline",
			inline: true,
			atom: true,
			attrs: { id: { default: "" }, name: { default: "" } },
			toDOM: (node) => [
				"span",
				{ class: "filter-mentions", "data-id": node.attrs.id },
				["span", { class: "filter-prefix" }, "mentions:"],
				["span", { class: "filter-value" }, node.attrs.name],
			],
			parseDOM: [
				{
					tag: "span.filter-mentions",
					getAttrs: (dom: HTMLElement) => ({
						id: dom.dataset.id,
						name: dom.textContent?.replace(/^mentions:/, "") ?? "",
					}),
				},
			],
		},
	},
});

const RECENT_SEARCHES_KEY = "recent_searches";

function getRecentSearches(): string[] {
	const stored = localStorage.getItem(RECENT_SEARCHES_KEY);
	if (!stored) return [];
	try {
		return JSON.parse(stored);
	} catch (e) {
		return [];
	}
}

function addRecentSearch(query: string) {
	if (!query.trim()) return;
	let searches = getRecentSearches();
	searches = [query, ...searches.filter((s) => s !== query)].slice(0, 10);
	localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(searches));
}

function serializeToQuery(state: EditorState): string {
	let query = "";
	state.doc.forEach((node) => {
		node.forEach((inlineNode) => {
			if (inlineNode.isText) {
				query += inlineNode.text;
			} else if (inlineNode.type.name === "author") {
				query += ` author:${inlineNode.attrs.id} `;
			} else if (inlineNode.type.name === "thread") {
				query += ` thread:${inlineNode.attrs.id} `;
			} else if (inlineNode.type.name === "before") {
				query += ` before:${inlineNode.attrs.date} `;
			} else if (inlineNode.type.name === "after") {
				query += ` after:${inlineNode.attrs.date} `;
			} else if (inlineNode.type.name === "has") {
				query += ` has:${inlineNode.attrs.value} `;
			} else if (inlineNode.type.name === "pinned") {
				query += ` pinned:${inlineNode.attrs.value} `;
			} else if (inlineNode.type.name === "mentions") {
				query += ` mentions:${inlineNode.attrs.id} `;
			}
		});
	});
	return query.trim().replace(/\s+/g, " ");
}

function parseQueryToNodes(
	query: string,
	users2: ReturnType<typeof useUsers2>,
	roomThreads: () => ThreadT[],
): { nodes: Node[]; text: string } {
	const nodes: Node[] = [];
	let textBuffer = "";

	// Regex to match filter patterns: filter:value or "quoted text"
	const tokenRegex =
		/(author|thread|before|after|has|pinned|mentions):(\S+)|"([^"]*)"/g;
	let lastIndex = 0;
	let match;

	while ((match = tokenRegex.exec(query)) !== null) {
		// Add text before this match
		const textBefore = query.slice(lastIndex, match.index);
		if (textBefore) {
			textBuffer += textBefore;
		}

		if (match[1]) {
			// Flush text buffer
			if (textBuffer) {
				nodes.push(schema.text(textBuffer));
				textBuffer = "";
			}

			const filterType = match[1];
			const value = match[2];

			if (filterType === "author") {
				const user = users2.cache.get(value);
				if (user) {
					nodes.push(
						schema.nodes.author.create({ id: user.id, name: user.name }),
					);
				} else {
					textBuffer += ` author:${value}`;
				}
			} else if (filterType === "thread") {
				const thread = roomThreads().find((t) => t.id === value);
				if (thread) {
					nodes.push(
						schema.nodes.thread.create({ id: thread.id, name: thread.name }),
					);
				} else {
					textBuffer += ` thread:${value}`;
				}
			} else if (filterType === "before") {
				nodes.push(schema.nodes.before.create({ date: value }));
			} else if (filterType === "after") {
				nodes.push(schema.nodes.after.create({ date: value }));
			} else if (filterType === "has") {
				nodes.push(schema.nodes.has.create({ value }));
			} else if (filterType === "pinned") {
				nodes.push(schema.nodes.pinned.create({ value }));
			} else if (filterType === "mentions") {
				nodes.push(schema.nodes.mentions.create({ id: value, name: value }));
			}
		} else if (match[3]) {
			// Quoted text - keep as text but include quotes
			textBuffer += match[0];
		}

		lastIndex = tokenRegex.lastIndex;
	}

	// Add remaining text
	if (lastIndex < query.length) {
		textBuffer += query.slice(lastIndex);
	}

	if (textBuffer) {
		nodes.push(schema.text(textBuffer));
	}

	return { nodes, text: textBuffer };
}

function dateToBoundaryUUID(
	dateString: string,
	boundary: "start" | "end",
): string | undefined {
	try {
		const date = new Date(dateString);
		if (isNaN(date.getTime())) return undefined;

		if (boundary === "start") {
			date.setUTCHours(0, 0, 0, 0);
			const unixTsMs = date.getTime();
			return UUID.fromFieldsV7(unixTsMs, 0, 0, 0).toString();
		} else {
			// end
			date.setUTCHours(23, 59, 59, 999);
			const unixTsMs = date.getTime();
			const randA = 0xfff;
			const randBHi = 0x3fffffff;
			const randBLo = 0xffffffff;
			return UUID.fromFieldsV7(unixTsMs, randA, randBHi, randBLo).toString();
		}
	} catch (e) {
		console.error("Invalid date for search filter:", e);
		return undefined;
	}
}

function syntaxHighlightingPlugin() {
	return new Plugin({
		props: {
			decorations(state) {
				const decorations: Decoration[] = [];
				state.doc.descendants((node, pos) => {
					if (node.type.name !== "text" && node.isAtom) {
						decorations.push(
							Decoration.inline(pos, pos + node.nodeSize, {
								class: `filter-atom filter-${node.type.name}`,
							}),
						);
						return false;
					}

					if (node.isText) {
						const text = node.text!;
						const phraseRegex = /"([^"]*)"/g;
						let match;
						while ((match = phraseRegex.exec(text))) {
							const from = pos + match.index;
							const to = from + match[0].length;
							decorations.push(
								Decoration.inline(from, to, {
									class: "filter-phrase",
								}),
							);
							decorations.push(
								Decoration.inline(from, from + 1, { class: "syn" }),
							);
							if (match[0].length > 1) {
								decorations.push(
									Decoration.inline(to - 1, to, { class: "syn" }),
								);
							}
						}

						const negationRegex = /(^|\s)-\S+/g;
						while ((match = negationRegex.exec(text))) {
							const from = pos + match.index + (match[1]?.length ?? 0);
							const to = from + match[0].length - (match[1]?.length ?? 0);
							decorations.push(
								Decoration.inline(from, to, {
									class: "filter-negation",
								}),
							);
							decorations.push(
								Decoration.inline(from, from + 1, { class: "syn" }),
							);
						}
					}
				});
				return DecorationSet.create(state.doc, decorations);
			},
		},
	});
}

const AutocompleteDropdown = (props: {
	filter: { type: string; query: string };
	channel?: ThreadT;
	room?: RoomT;
	onSelect: (node: Node) => void;
	onSelectFilter: (text: string) => void;
}) => {
	const channels2 = useChannels2();
	const threadMembers2 = useThreadMembers2();
	const roomMembers2 = useRoomMembers2();
	const users2 = useUsers2();
	const roomThreads = () =>
		[...channels2.cache.values()].filter(
			(c) =>
				c.room_id === ((props.channel?.room_id as any) ?? props.room?.id ?? ""),
		);
	const roles = useRoles2();
	const roomId = () => props.channel?.room_id ?? props.room?.id ?? null;
	const roomRoles = createMemo(() =>
		[...roles.cache.values()].filter((r) => r.room_id === roomId())
	);

	const threadMemberIds = () => {
		const threadId = props.channel?.id as any;
		if (!threadId) return [];
		return [...threadMembers2.cache.entries()]
			.filter(([key]) => key.startsWith(`${threadId}:`))
			.map(([, member]) => member.user_id);
	};

	const roomMemberIds = () => {
		const rid = (props.channel?.room_id as any) ?? props.room?.id ?? "";
		if (!rid) return [];
		return [...roomMembers2.cache.entries()]
			.filter(([key]) => key.startsWith(`${rid}:`))
			.map(([, member]) => member.user_id);
	};

	const authorSuggestions = createMemo(() => {
		const query = props.filter.query.toLowerCase();
		const tm = threadMemberIds();
		const rm = roomMemberIds();
		const all_user_ids = [...new Set([...tm, ...rm])];

		if (!query) return all_user_ids.slice(0, 10);

		const users = all_user_ids.map((id) => users2.cache.get(id)).filter(
			Boolean,
		) as User[];
		return users
			.filter(
				(u) =>
					u.name.toLowerCase().includes(query) ||
					u.id.toLowerCase().includes(query),
			)
			.map((u) => u.id)
			.slice(0, 10);
	});

	const threadSuggestions = createMemo(() => {
		const query = props.filter.query.toLowerCase();
		const threads = roomThreads() ?? [];
		if (!query) return threads.slice(0, 10);
		return threads
			.filter(
				(t) =>
					t.name.toLowerCase().includes(query) ||
					t.id.toLowerCase().includes(query),
			)
			.slice(0, 10);
	});

	const hasFilterSuggestions = createMemo(() => {
		const query = props.filter.query.toLowerCase();
		const options = [
			"attachment",
			"image",
			"audio",
			"video",
			"link",
			"embed",
		];
		if (!query) return options;
		return options.filter((o) => o.toLowerCase().includes(query));
	});

	const pinnedSuggestions = createMemo(() => {
		const query = props.filter.query.toLowerCase();
		const options = ["true", "false"];
		if (!query) return options;
		return options.filter((o) => o.toLowerCase().includes(query));
	});

	type Mentionable = {
		id: string;
		name: string;
		type: "user" | "role" | "special";
	};
	const mentionsSuggestions = createMemo(() => {
		const query = props.filter.query.toLowerCase();

		const tm = threadMemberIds();
		const rm = roomMemberIds();
		const all_user_ids = [...new Set([...tm, ...rm])];
		const users = (
			all_user_ids.map((id) => users2.cache.get(id)).filter(
				Boolean,
			) as User[]
		)
			.filter(
				(u) =>
					u.name.toLowerCase().includes(query) ||
					u.id.toLowerCase().includes(query),
			)
			.map(
				(u) =>
					({ id: `user-${u.id}`, name: u.name, type: "user" }) as Mentionable,
			);

		const roles = (roomRoles() ?? [])
			.filter((r) => r.name.toLowerCase().includes(query))
			.map(
				(r) =>
					({
						id: `role-${r.id}`,
						name: r.name,
						type: "role" as const,
					}) as Mentionable,
			);

		const special: Mentionable[] = [
			{ id: "everyone-room", name: "@room", type: "special" as const },
			{ id: "everyone-thread", name: "@thread", type: "special" as const },
		].filter((s) => (s.name as any).toLowerCase().includes(query));

		return [...users, ...roles, ...special].slice(0, 10);
	});

	const onAuthorSelect = (user_id: string) => {
		const user = users2.cache.get(user_id);
		if (!user) return;
		const node = schema.nodes.author.create({ id: user.id, name: user.name });
		props.onSelect(node);
	};

	const onThreadSelect = (thread: ThreadT) => {
		const node = schema.nodes.thread.create({
			id: thread.id,
			name: thread.name,
		});
		props.onSelect(node);
	};

	const onHasSelect = (value: string) => {
		const node = schema.nodes.has.create({ value });
		props.onSelect(node);
	};

	const onPinnedSelect = (value: string) => {
		const node = schema.nodes.pinned.create({ value });
		props.onSelect(node);
	};

	const onMentionsSelect = (mentionable: Mentionable) => {
		const node = schema.nodes.mentions.create({
			id: mentionable.id,
			name: mentionable.name,
		});
		props.onSelect(node);
	};

	const allFilterSuggestions = [
		"author:",
		"thread:",
		"before:",
		"after:",
		"has:",
		"pinned:",
		"mentions:",
	];
	const filterSuggestions = createMemo(() => {
		const query = props.filter.query.toLowerCase();
		if (!query) return allFilterSuggestions;
		return allFilterSuggestions.filter((f) => f.toLowerCase().includes(query));
	});

	const recentSearches = createMemo(() => {
		if (props.filter.type === "filter" && props.filter.query === "") {
			return getRecentSearches();
		}
		return [];
	});

	const hasSuggestions = createMemo(() => {
		if (props.filter.type === "author") {
			return authorSuggestions().length > 0;
		}
		if (props.filter.type === "thread") {
			return threadSuggestions().length > 0;
		}
		if (props.filter.type === "has") {
			return hasFilterSuggestions().length > 0;
		}
		if (props.filter.type === "pinned") {
			return pinnedSuggestions().length > 0;
		}
		if (props.filter.type === "mentions") {
			return mentionsSuggestions().length > 0;
		}
		if (props.filter.type === "filter") {
			return filterSuggestions().length > 0 || recentSearches().length > 0;
		}
		return false;
	});

	return (
		<Show when={hasSuggestions()}>
			<div class="search-autocomplete">
				<Show when={props.filter.type === "author"}>
					<ul>
						<For each={authorSuggestions()}>
							{(user_id) => {
								const user = users2.cache.get(user_id);
								return (
									<li
										class="autocomplete-item"
										onMouseDown={(e) => {
											e.preventDefault();
											onAuthorSelect(user_id);
										}}
									>
										<b>{user?.name}</b>
									</li>
								);
							}}
						</For>
					</ul>
				</Show>
				<Show when={props.filter.type === "thread"}>
					<ul>
						<For each={threadSuggestions()}>
							{(thread) => (
								<li
									class="autocomplete-item"
									onMouseDown={(e) => {
										e.preventDefault();
										onThreadSelect(thread);
									}}
								>
									<b>{thread.name}</b>
								</li>
							)}
						</For>
					</ul>
				</Show>
				<Show when={props.filter.type === "has"}>
					<ul>
						<For each={hasFilterSuggestions()}>
							{(value) => (
								<li
									class="autocomplete-item"
									onMouseDown={(e) => {
										e.preventDefault();
										onHasSelect(value);
									}}
								>
									<b>{value}</b>
								</li>
							)}
						</For>
					</ul>
				</Show>
				<Show when={props.filter.type === "pinned"}>
					<ul>
						<For each={pinnedSuggestions()}>
							{(value) => (
								<li
									class="autocomplete-item"
									onMouseDown={(e) => {
										e.preventDefault();
										onPinnedSelect(value);
									}}
								>
									<b>{value}</b>
								</li>
							)}
						</For>
					</ul>
				</Show>
				<Show when={props.filter.type === "mentions"}>
					<ul>
						<For each={mentionsSuggestions()}>
							{(mentionable) => (
								<li
									class="autocomplete-item"
									onMouseDown={(e) => {
										e.preventDefault();
										onMentionsSelect(mentionable);
									}}
								>
									<b>{mentionable.name}</b>
								</li>
							)}
						</For>
					</ul>
				</Show>
				<Show when={props.filter.type === "filter"}>
					<ul>
						<For each={filterSuggestions()}>
							{(filter) => (
								<li
									class="autocomplete-item"
									onMouseDown={(e) => {
										e.preventDefault();
										props.onSelectFilter(filter);
									}}
								>
									<b>{filter}</b>
								</li>
							)}
						</For>
						<Show when={recentSearches().length > 0}>
							<li
								class="dim"
								style="margin-top: 8px; font-size: 0.8em; text-transform: uppercase; font-weight: bold;"
							>
								Recent Searches
							</li>
							<For each={recentSearches()}>
								{(search) => (
									<li
										class="autocomplete-item"
										onMouseDown={(e) => {
											e.preventDefault();
											props.onSelectFilter(search);
										}}
									>
										{search}
									</li>
								)}
							</For>
						</Show>
					</ul>
				</Show>
			</div>
		</Show>
	);
};

function autocompletePlugin(
	setFilter: (filter: { type: string; query: string } | null) => void,
) {
	return new Plugin({
		state: {
			init: () => null,
			apply: (tr, value) => {
				if (tr.getMeta("skipAutocomplete")) {
					setFilter(null);
					return null;
				}

				const { selection } = tr;
				if (!selection.empty) {
					setFilter(null);
					return null;
				}

				const text = tr.doc.textContent;
				const cursorPos = selection.from;
				const textBeforeCursor = text.slice(0, cursorPos);

				const filterMatch = textBeforeCursor.match(
					/\b(author|thread|has|pinned|mentions):(\S*)$/,
				);
				if (filterMatch) {
					setFilter({ type: filterMatch[1], query: filterMatch[2] });
					return null;
				}

				// Only suggest at the end of a word/input
				if (text.slice(cursorPos).match(/^\S/)) {
					setFilter(null);
					return null;
				}

				const wordMatch = textBeforeCursor.match(/(\S+)$/);
				if (wordMatch) {
					const word = wordMatch[1];
					if (word.includes(":")) {
						setFilter(null);
						return null;
					}
					setFilter({ type: "filter", query: word });
				} else {
					// Empty or ends with space
					setFilter({ type: "filter", query: "" });
				}

				return null;
			},
		},
	});
}

export const SearchInput = (
	props: { channel?: ThreadT; room?: RoomT; autofocus?: boolean },
) => {
	const api2 = useApi2();
	const users2 = useUsers2();
	const messagesService = useMessages2();
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
	const [activeFilter, setActiveFilter] = createSignal<
		{
			type: string;
			query: string;
		} | null
	>(null);

	const [editorRef, setEditorRef] = createSignal<HTMLElement>();

	const position = useFloating(
		editorRef,
		dropdownRef,
		{
			whileElementsMounted: autoUpdate,
			middleware: [offset(4), flip()],
			placement: "bottom-start",
		},
	);

	const channelCtx = useChannel();
	const roomCtx = useRoom();

	const channels2 = useChannels2();
	const roomThreads = () =>
		[...channels2.cache.values()].filter(
			(c) => c.room_id === (props.channel?.room_id ?? props.room?.id ?? ""),
		);

	const currentSearch = () => {
		if (props.channel) return channelCtx?.[0].search;
		if (props.room) return roomCtx?.[0].search;
		return undefined;
	};

	const updateSearch = (val: ChannelSearch | RoomSearch | undefined) => {
		if (props.channel && channelCtx) {
			channelCtx[1]("search", val as any);
		} else if (props.room && roomCtx) {
			roomCtx[1]("search", val as any);
		}
	};

	// Clear editor when search is cleared
	createEffect(() => {
		const search = currentSearch();
		const view = editor.view;

		if (!view) return;

		// Clear editor when search is cleared or when there's no search
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

		// Extract filters directly from editor state using node IDs
		const filters: {
			author_ids?: string[];
			thread_ids?: string[];
			before?: string;
			after?: string;
			has?: string[];
			pinned?: string;
			mentions_ids?: string[];
			mentions_everyone?: boolean;
		} = {};

		const textQueryParts: string[] = [];

		editor.view.state.doc.forEach((node) => {
			node.forEach((inlineNode) => {
				if (inlineNode.isText) {
					textQueryParts.push(inlineNode.text!);
				} else if (inlineNode.type.name === "author") {
					if (!filters.author_ids) filters.author_ids = [];
					filters.author_ids.push(inlineNode.attrs.id);
				} else if (inlineNode.type.name === "thread") {
					if (!filters.thread_ids) filters.thread_ids = [];
					filters.thread_ids.push(inlineNode.attrs.id);
				} else if (inlineNode.type.name === "before") {
					filters.before = inlineNode.attrs.date;
				} else if (inlineNode.type.name === "after") {
					filters.after = inlineNode.attrs.date;
				} else if (inlineNode.type.name === "has") {
					if (!filters.has) filters.has = [];
					filters.has.push(inlineNode.attrs.value);
				} else if (inlineNode.type.name === "pinned") {
					filters.pinned = inlineNode.attrs.value;
				} else if (inlineNode.type.name === "mentions") {
					if (!filters.mentions_ids) filters.mentions_ids = [];
					filters.mentions_ids.push(inlineNode.attrs.id);
				}
			});
		});

		const textQuery = textQueryParts.join(" ");

		const existing = currentSearch();
		const searchState: any = {
			query: queryString,
			results: existing?.results ?? null,
			loading: true,
			author: filters.author_ids,
			before: filters.before,
			after: filters.after,
			channel: filters.thread_ids,
		};
		updateSearch(searchState);

		const queryParts: string[] = [];

		if (textQuery) {
			queryParts.push(`+(${textQuery})`);
		}

		if (filters.author_ids) {
			queryParts.push(`+author_id: IN [${filters.author_ids.join(" ")}]`);
		}

		if (props.channel) {
			if (props.channel.type === "Dm" || props.channel.type === "Gdm") {
				queryParts.push(`+channel_id:${props.channel.id}`);
			} else if (filters.thread_ids) {
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

		if (filters.before && filters.after) {
			const from_uuid = dateToBoundaryUUID(filters.after, "start");
			const to_uuid = dateToBoundaryUUID(filters.before, "end");
			if (from_uuid && to_uuid) {
				queryParts.push(`+created_at:[${from_uuid} TO ${to_uuid}]`);
			}
		} else if (filters.after) {
			const from_uuid = dateToBoundaryUUID(filters.after, "start");
			if (from_uuid) {
				queryParts.push(`+created_at:[${from_uuid} TO *]`);
			}
		} else if (filters.before) {
			const to_uuid = dateToBoundaryUUID(filters.before, "end");
			if (to_uuid) {
				queryParts.push(`+created_at:[* TO ${to_uuid}]`);
			}
		}

		if (filters.has) {
			const hasSubquery: string[] = [];

			if (filters.has.includes("attachment")) {
				hasSubquery.push(`metadata_fast.has_attachment:true`);
			}
			if (filters.has.includes("image")) {
				hasSubquery.push(`metadata_fast.has_image:true`);
			}
			if (filters.has.includes("audio")) {
				hasSubquery.push(`metadata_fast.has_audio:true`);
			}
			if (filters.has.includes("video")) {
				hasSubquery.push(`metadata_fast.has_video:true`);
			}
			if (filters.has.includes("link")) {
				hasSubquery.push(`metadata_fast.has_link:true`);
			}
			if (filters.has.includes("embed")) {
				hasSubquery.push(`metadata_fast.has_embed:true`);
			}

			if (hasSubquery.length === 1) {
				queryParts.push(`+${hasSubquery[0]}`);
			} else if (hasSubquery.length > 1) {
				queryParts.push(`+(${hasSubquery.join(" ")})`);
			}
		}

		if (filters.pinned) {
			queryParts.push(`+metadata_fast.pinned:${filters.pinned}`);
		}

		if (filters.mentions_ids) {
			const mentionSubquery: string[] = [];

			for (const mentionId of filters.mentions_ids) {
				if (mentionId.startsWith("user-")) {
					const user_id = mentionId.replace("user-", "");
					mentionSubquery.push(`metadata_fast.mentions_user:${user_id}`);
				} else if (mentionId.startsWith("role-")) {
					const role_id = mentionId.replace("role-", "");
					mentionSubquery.push(`metadata_fast.mentions_role:${role_id}`);
				} else if (
					mentionId === "everyone-room" || mentionId === "everyone-thread"
				) {
					mentionSubquery.push(`metadata_fast.mentions_everyone:true`);
				}
			}

			if (mentionSubquery.length === 1) {
				queryParts.push(`+${mentionSubquery[0]}`);
			} else if (mentionSubquery.length > 1) {
				queryParts.push(`+(${mentionSubquery.join(" ")})`);
			}
		}

		console.log("search input calculated query parts", queryParts);

		const body: {
			query?: string;
			sort_order?: "asc" | "desc";
			sort_field?: "Created" | "Relevancy";
			limit?: number;
			offset?: number;
			include_nsfw?: boolean;
		} = {
			query: queryParts.join(" ") || undefined,
			sort_order: "desc",
			sort_field: "Created",
			limit: 100,
		};

		const res = await messagesService.search(body);
		if (res) {
			updateSearch({
				...searchState,
				results: res,
				loading: false,
			});
		} else {
			updateSearch({
				...searchState,
				results: null,
				loading: false,
			});
		}
	};

	const insertNode = (node: Node) => {
		if (!editor.view) return;
		const { from } = editor.view.state.selection;
		const textBefore = editor.view.state.doc.textBetween(0, from, "\0");
		const filterMatch = textBefore.match(
			/\b(author|thread|has|pinned|mentions):(\S*)$/,
		);
		if (filterMatch) {
			const matchText = filterMatch[0];
			const start = from - matchText.length;
			const tr = editor.view.state.tr.replaceWith(start, from, node);
			editor.view.dispatch(tr);
			editor.view.focus();
		}
		setActiveFilter(null);
	};

	const insertFilter = (text: string) => {
		if (!editor.view) return;
		const { from } = editor.view.state.selection;
		const textBefore = editor.view.state.doc.textBetween(0, from, " ");
		const wordMatch = textBefore.match(/(\S+)$/);
		const start = wordMatch ? from - wordMatch[0].length : from;

		// Check if this is a filter with ID (from search history)
		const filterMatch = text.match(/^(author|thread|mentions):(\S+)$/);
		if (filterMatch) {
			const [, type, id] = filterMatch;
			let node: Node | null = null;
			if (type === "author") {
				const user = users2.cache.get(id);
				if (user) {
					node = schema.nodes.author.create({ id: user.id, name: user.name });
				}
			} else if (type === "thread") {
				const threads = roomThreads() ?? [];
				const thread = threads.find((t) => t.id === id);
				if (thread) {
					node = schema.nodes.thread.create({
						id: thread.id,
						name: thread.name,
					});
				}
			} else if (type === "mentions") {
				// mentions can be user-{id}, role-{id}, or special
				node = schema.nodes.mentions.create({ id, name: id });
			}
			if (node) {
				const tr = editor.view.state.tr.replaceWith(start, from, node);
				editor.view.dispatch(tr);
				editor.view.focus();
				setActiveFilter(null);
				return;
			}
		}

		// Check if this is a full query string from history (may contain multiple filters)
		if (text.match(/\b(author|thread|before|after|has|pinned|mentions):\S+/)) {
			const { nodes } = parseQueryToNodes(text, users2, roomThreads);
			if (nodes.length > 0) {
				const tr = editor.view.state.tr.replaceWith(start, from, ...nodes);
				editor.view.dispatch(tr);
				editor.view.focus();
				setActiveFilter(null);
				return;
			}
		}

		const tr = editor.view.state.tr.insertText(text, start, from);
		editor.view.dispatch(tr);
		editor.view.focus();
		setActiveFilter(null);
	};

	const editor = createBaseEditor({
		schema: (schema as any),
		createState: (schema) => {
			// Parse initial search query from history into nodes
			let docContent: any = null;
			const initialSearch = currentSearch();
			if (initialSearch?.query) {
				const { nodes } = parseQueryToNodes(
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
				schema: (schema as any),
				doc: docContent,
				plugins: [
					history(),
					keymap({
						"Ctrl-z": undo,
						"Ctrl-Shift-z": redo,
						"Escape": () => {
							if (!editor.view) return false;
							const { state } = editor.view;
							if (state.doc.textContent.length > 0) {
								const tr = state.tr.delete(0, state.doc.content.size);
								editor.view.dispatch(tr);
								handleSubmit();
								return true;
							}

							if (currentSearch()) {
								updateSearch(undefined);
							} else {
								const chatInput = document.querySelector(
									".chat .ProseMirror",
								) as HTMLInputElement | null;
								chatInput?.focus();
							}
							return true;
						},
					}),
					syntaxHighlightingPlugin(),
					autocompletePlugin((filter) => {
						if (filter && editor.view && !editor.view.hasFocus()) return;
						setActiveFilter(filter);
					}),
					new Plugin({
						props: {
							handleKeyDown(_view, event) {
								if (event.key === "Enter" && !event.shiftKey) {
									handleSubmit();
									setActiveFilter(null);
									return true;
								}
								return false;
							},
						},
					}),
				],
			});
		},
		handleDOMEvents: {
			focus: (view: any) => {
				const { state } = view;
				const { selection } = state;
				if (!selection.empty) {
					return false;
				}

				const text = state.doc.textContent;
				const cursorPos = selection.from;
				const textBeforeCursor = text.slice(0, cursorPos);

				const filterMatch = textBeforeCursor.match(
					/\b(author|thread|has|pinned|mentions):(\S*)$/,
				);
				if (filterMatch) {
					setActiveFilter({ type: filterMatch[1], query: filterMatch[2] });
					return false;
				}

				// Only suggest at the end of a word/input
				if (text.slice(cursorPos).match(/^\S/)) {
					return false;
				}

				const wordMatch = textBeforeCursor.match(/(\S+)$/);
				if (wordMatch) {
					const word = wordMatch[1];
					if (word.includes(":")) {
						return false;
					}
					setActiveFilter({ type: "filter", query: word });
				} else {
					// Empty or ends with space
					setActiveFilter({ type: "filter", query: "" });
				}
				return false;
			},
			blur: () => {
				// Use a small delay to allow click events on the dropdown to register
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
							width: `${(editorRef()?.offsetWidth || 0)}px`,
						}}
					>
						<AutocompleteDropdown
							filter={activeFilter()!}
							channel={props.channel}
							room={props.room}
							onSelect={insertNode}
							onSelectFilter={insertFilter}
						/>
					</div>
				</Show>
			</Portal>
			<img class="icon" src={icSearch} />
		</div>
	);
};
