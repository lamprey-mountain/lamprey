import {
	createMemo,
	createSignal,
	For,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import { useApi } from "./api";
import { useCtx } from "./context";
import type { ThreadT } from "./types";
import type { ChannelSearch } from "./context";
import { User } from "sdk";
import { UUID } from "uuidv7";
import { EditorState, Plugin } from "prosemirror-state";
import { Decoration, DecorationSet, EditorView } from "prosemirror-view";
import { Node, Schema } from "prosemirror-model";
import { keymap } from "prosemirror-keymap";
import { history, redo, undo } from "prosemirror-history";
import { Portal } from "solid-js/web";
import { autoUpdate, flip, offset } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";

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
				`author:${node.attrs.name}`,
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
				`thread:${node.attrs.name}`,
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
				`before:${node.attrs.date}`,
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
				`after:${node.attrs.date}`,
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
				`has:${node.attrs.value}`,
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
				`pinned:${node.attrs.value}`,
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
				`mentions:${node.attrs.name}`,
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
	channel: ThreadT;
	onSelect: (node: Node) => void;
	onSelectFilter: (text: string) => void;
}) => {
	const api = useApi();
	const threadMembers = api.thread_members.list(() => props.channel.id);
	const roomMembers = api.room_members.list(() => props.channel.room_id ?? "");
	const roomThreads = api.channels.list(() => props.channel.room_id ?? "");
	const roomRoles = api.roles.list(() => props.channel.room_id ?? "");

	const authorSuggestions = createMemo(() => {
		const query = props.filter.query.toLowerCase();
		const tm = threadMembers()?.items.map((m) => m.user_id) ?? [];
		const rm = roomMembers()?.items.map((m) => m.user_id) ?? [];
		const all_user_ids = [...new Set([...tm, ...rm])];

		if (!query) return all_user_ids.slice(0, 10);

		const users = all_user_ids.map((id) => api.users.cache.get(id)).filter(
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
		const threads = roomThreads()?.items ?? [];
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

		const tm = threadMembers()?.items.map((m) => m.user_id) ?? [];
		const rm = roomMembers()?.items.map((m) => m.user_id) ?? [];
		const all_user_ids = [...new Set([...tm, ...rm])];
		const users = (
			all_user_ids.map((id) => api.users.cache.get(id)).filter(
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

		const roles = (roomRoles()?.items ?? [])
			.filter((r) => r.name.toLowerCase().includes(query))
			.map(
				(r) =>
					({ id: `role-${r.id}`, name: r.name, type: "role" }) as Mentionable,
			);

		const special: Mentionable[] = [
			{ id: "everyone-room", name: "@room", type: "special" },
			{ id: "everyone-thread", name: "@thread", type: "special" },
		].filter((s) => s.name.toLowerCase().includes(query));

		return [...users, ...roles, ...special].slice(0, 10);
	});

	const onAuthorSelect = (user_id: string) => {
		const user = api.users.cache.get(user_id);
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
			return filterSuggestions().length > 0;
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
								const user = api.users.cache.get(user_id);
								return (
									<li
										onMouseDown={(e) => {
											e.preventDefault();
											onAuthorSelect(user_id);
										}}
									>
										<b>{user?.name}</b>
										<span class="dim">({user_id})</span>
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
									onMouseDown={(e) => {
										e.preventDefault();
										onThreadSelect(thread);
									}}
								>
									<b>{thread.name}</b>
									<span class="dim">({thread.id})</span>
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
									onMouseDown={(e) => {
										e.preventDefault();
										onMentionsSelect(mentionable);
									}}
								>
									<b>{mentionable.name}</b>
									<span class="dim">({mentionable.type})</span>
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
									onMouseDown={(e) => {
										e.preventDefault();
										props.onSelectFilter(filter);
									}}
								>
									<b>{filter}</b>
								</li>
							)}
						</For>
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

export const SearchInput = (props: { channel: ThreadT }) => {
	const api = useApi();
	const ctx = useCtx();
	let editorRef: HTMLDivElement | undefined;
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
	let view: EditorView;
	const [activeFilter, setActiveFilter] = createSignal<
		{
			type: string;
			query: string;
		} | null
	>(null);

	const position = useFloating(
		() => editorRef,
		dropdownRef,
		{
			whileElementsMounted: autoUpdate,
			middleware: [offset(4), flip()],
			placement: "bottom-start",
		},
	);

	const handleSubmit = async () => {
		const queryString = serializeToQuery(view.state);
		if (!queryString) {
			ctx.channel_search.delete(props.channel.id);
			return;
		}

		const parts = queryString.split(/\s+/);
		const textQueryParts: string[] = [];
		const filters: Record<string, string[]> = {};
		const filterRegex =
			/^(author|thread|before|after|has|pinned|mentions):(.+)$/;

		for (const part of parts) {
			const match = part.match(filterRegex);
			if (match && match[2]) {
				const key = match[1];
				const value = match[2];
				if (!filters[key]) {
					filters[key] = [];
				}
				filters[key].push(value);
			} else {
				textQueryParts.push(part);
			}
		}
		const textQuery = textQueryParts.join(" ");

		const existing = ctx.channel_search.get(props.channel.id);
		const searchState: ChannelSearch = {
			query: queryString,
			results: existing?.results ?? null,
			loading: true,
			author: filters.author,
			before: filters.before?.[0],
			after: filters.after?.[0],
			channel: filters.channel,
		};
		ctx.channel_search.set(props.channel.id, searchState);

		const body: {
			query?: string;
			user_id?: string[];
			thread_id?: string[];
			room_id?: string[];
			has_attachment?: boolean;
			has_image?: boolean;
			has_audio?: boolean;
			has_video?: boolean;
			has_link?: boolean;
			has_embed?: boolean;
			pinned?: boolean;
			mentions_users?: string[];
			mentions_roles?: string[];
			mentions_everyone_room?: boolean;
			mentions_everyone_thread?: boolean;
		} = { query: textQuery || undefined };
		const params: { query: { limit: number; from?: string; to?: string } } = {
			query: { limit: 100 },
		};

		if (filters.author) body.user_id = filters.author;

		if (props.channel.type === "Dm" || props.channel.type === "Gdm") {
			body.thread_id = [props.channel.id];
		} else if (filters.thread) {
			body.thread_id = filters.thread;
			if (props.channel.room_id) body.room_id = [props.channel.room_id];
		} else if (props.channel.room_id) {
			body.room_id = [props.channel.room_id];
		} else {
			body.thread_id = [props.channel.id];
		}

		if (filters.before?.[0]) {
			const to_uuid = dateToBoundaryUUID(filters.before[0], "end");
			if (to_uuid) params.query.to = to_uuid;
		}
		if (filters.after?.[0]) {
			const from_uuid = dateToBoundaryUUID(filters.after[0], "start");
			if (from_uuid) params.query.from = from_uuid;
		}

		if (filters.has) {
			if (filters.has.includes("attachment")) body.has_attachment = true;
			if (filters.has.includes("image")) body.has_image = true;
			if (filters.has.includes("audio")) body.has_audio = true;
			if (filters.has.includes("video")) body.has_video = true;
			if (filters.has.includes("link")) body.has_link = true;
			if (filters.has.includes("embed")) body.has_embed = true;
		}

		if (filters.pinned?.[0]) {
			body.pinned = filters.pinned[0] === "true";
		}

		if (filters.mentions) {
			const mentions_users: string[] = [];
			const mentions_roles: string[] = [];
			for (const mention of filters.mentions) {
				if (mention.startsWith("user-")) {
					mentions_users.push(mention.replace("user-", ""));
				} else if (mention.startsWith("role-")) {
					mentions_roles.push(mention.replace("role-", ""));
				} else if (mention === "everyone-room") {
					body.mentions_everyone_room = true;
				} else if (mention === "everyone-thread") {
					body.mentions_everyone_thread = true;
				}
			}
			if (mentions_users.length > 0) body.mentions_users = mentions_users;
			if (mentions_roles.length > 0) body.mentions_roles = mentions_roles;
		}

		const res = await api.client.http.POST("/api/v1/search/message", {
			body,
			params,
		});
		if (res.data) {
			ctx.channel_search.set(props.channel.id, {
				...searchState,
				results: res.data,
				loading: false,
			});
		} else {
			ctx.channel_search.set(props.channel.id, {
				...searchState,
				results: null,
				loading: false,
			});
		}
	};

	const insertNode = (node: Node) => {
		const { from } = view.state.selection;
		const textBefore = view.state.doc.textBetween(0, from, "\0");
		const filterMatch = textBefore.match(
			/\b(author|thread|has|pinned|mentions):(\S*)$/,
		);
		if (filterMatch) {
			const matchText = filterMatch[0];
			const start = from - matchText.length;
			const tr = view.state.tr.replaceWith(start, from, node);
			view.dispatch(tr);
			view.focus();
		}
	};

	const insertFilter = (text: string) => {
		const { from } = view.state.selection;
		const textBefore = view.state.doc.textBetween(0, from, " ");
		const wordMatch = textBefore.match(/(\S+)$/);
		const start = wordMatch ? from - wordMatch[0].length : from;
		const tr = view.state.tr.insertText(text, start, from);
		view.dispatch(tr);
		view.focus();
	};

	onMount(() => {
		const state = EditorState.create({
			schema,
			plugins: [
				history(),
				keymap({
					"Ctrl-z": undo,
					"Ctrl-Shift-z": redo,
					Enter: () => {
						handleSubmit();
						return true;
					},
					"Escape": () => {
						const { state } = view;
						if (state.doc.textContent.length > 0) {
							const tr = state.tr.delete(0, state.doc.content.size);
							view.dispatch(tr);
							handleSubmit();
							return true;
						}

						if (ctx.channel_search.has(props.channel.id)) {
							ctx.channel_search.delete(props.channel.id);
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
				autocompletePlugin(setActiveFilter),
			],
		});

		view = new EditorView(editorRef!, {
			state,
			decorations(state) {
				if (state.doc.firstChild!.firstChild === null) {
					const placeholder = (
						<div class="placeholder" role="presentation">
							search
						</div>
					) as HTMLDivElement;
					return DecorationSet.create(state.doc, [
						Decoration.widget(0, placeholder),
					]);
				}
				return DecorationSet.empty;
			},
			handleDOMEvents: {
				focus: (view) => {
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
		});

		onCleanup(() => {
			view.destroy();
		});
	});

	return (
		<div class="search-container">
			<div class="search-input" ref={editorRef!}></div>
			<Portal>
				<Show when={activeFilter()}>
					<div
						ref={setDropdownRef}
						class="floating"
						style={{
							position: position.strategy,
							top: `${position.y ?? 0}px`,
							left: `${position.x ?? 0}px`,
							width: `${editorRef?.offsetWidth || 0}px`,
						}}
					>
						<AutocompleteDropdown
							filter={activeFilter()!}
							thread={props.channel}
							onSelect={insertNode}
							onSelectFilter={insertFilter}
						/>
					</div>
				</Show>
			</Portal>
		</div>
	);
};
