import {
	createEffect,
	createMemo,
	createSignal,
	For,
	onCleanup,
	Show,
} from "solid-js";
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
import { Decoration, DecorationSet, EditorView } from "prosemirror-view";
import { Node, Schema } from "prosemirror-model";
import { keymap } from "prosemirror-keymap";
import { history, redo, undo } from "prosemirror-history";
import { Portal } from "solid-js/web";
import { autoUpdate, flip, offset, size } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import icSearch from "../../../assets/search.png";
import { useChannel } from "../../../channelctx";
import { RoomSearch, useRoom } from "../../../contexts/room";
import {
	createEditor as createBaseEditor,
	createPlaceholderPlugin,
} from "../editor/mod.tsx";

type FilterNodeSpec = {
	group: string;
	inline: boolean;
	atom: boolean;
	attrs: Record<string, { default: string | boolean }>;
	toDOM: (
		node: import("prosemirror-model").Node,
	) => import("prosemirror-model").DOMOutputSpec;
	parseDOM: Array<{
		tag: string;
		getAttrs: (dom: HTMLElement) => Record<string, unknown>;
	}>;
};

const createFilterNode = (
	name: string,
	valueKey: "id" | "value" | "date" = "value",
	hasNameAttr: boolean = false,
): FilterNodeSpec => ({
	group: "inline",
	inline: true,
	atom: true,
	attrs: {
		[valueKey]: { default: "" },
		...(hasNameAttr ? { name: { default: "" } } : {}),
		negated: { default: false },
	},
	toDOM: (node: import("prosemirror-model").Node) => {
		const displayValue = hasNameAttr ? node.attrs.name : node.attrs[valueKey];
		return [
			"span",
			{
				class: `filter-${name}${node.attrs.negated ? " filter-negated" : ""}`,
				...(valueKey === "id" ? { "data-id": node.attrs.id } : {}),
			},
			[
				"span",
				{ class: "filter-prefix" },
				node.attrs.negated ? `-${name}:` : `${name}:`,
			],
			["span", { class: "filter-value" }, displayValue],
		];
	},
	parseDOM: [
		{
			tag: `span.filter-${name}`,
			getAttrs: (dom: HTMLElement) => ({
				[valueKey]: valueKey === "id"
					? dom.dataset.id
					: dom.querySelector(".filter-value")?.textContent ?? "",
				...(hasNameAttr
					? { name: dom.querySelector(".filter-value")?.textContent ?? "" }
					: {}),
				negated: dom.classList.contains("filter-negated"),
			}),
		},
	],
});

const schema = new Schema({
	nodes: {
		doc: { content: "paragraph" },
		paragraph: {
			content: "inline*",
			group: "block",
			toDOM: () => ["p", 0],
		},
		text: { group: "inline" },
		author: createFilterNode("author", "id", true),
		channel: createFilterNode("channel", "id", true),
		before: createFilterNode("before", "date"),
		after: createFilterNode("after", "date"),
		has: createFilterNode("has", "value"),
		pinned: createFilterNode("pinned", "value"),
		mentions: createFilterNode("mentions", "id", true),
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
	const normalizedQuery = query.trim().replace(/\s+/g, " ");
	let searches = getRecentSearches();
	searches = [normalizedQuery, ...searches.filter((s) => s !== normalizedQuery)]
		.slice(0, 10);
	localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(searches));
}

function serializeToQuery(state: EditorState): string {
	let query = "";
	state.doc.forEach((node) => {
		node.forEach((inlineNode) => {
			if (inlineNode.isText) {
				query += inlineNode.text;
			} else {
				const type = inlineNode.type.name;
				const negated = inlineNode.attrs.negated ? "-" : "";
				if (type === "author" || type === "channel" || type === "mentions") {
					query += ` ${negated}${type}:${inlineNode.attrs.id} `;
				} else if (type === "before" || type === "after") {
					query += ` ${negated}${type}:${inlineNode.attrs.date} `;
				} else if (type === "has" || type === "pinned") {
					query += ` ${negated}${type}:${inlineNode.attrs.value} `;
				}
			}
		});
	});
	return query.trim().replace(/\s+/g, " ");
}

function parseSearchQuery(query: string) {
	const tokens: {
		type: "filter" | "negated-filter";
		filterType: string;
		value: string;
		from: number;
		to: number;
		negated: boolean;
	}[] = [];

	const filterRegex =
		/(-?)(author|channel|before|after|has|pinned|mentions):(\S*)/g;
	let match;

	while ((match = filterRegex.exec(query)) !== null) {
		const isNegated = !!match[1];
		tokens.push({
			type: isNegated ? "negated-filter" : "filter",
			filterType: match[2],
			value: match[3],
			from: match.index,
			to: match.index + match[0].length,
			negated: isNegated,
		});
	}

	return tokens;
}

function parseQueryToNodes(
	query: string,
	users2: ReturnType<typeof useUsers2>,
	roomThreads: () => ThreadT[],
): Node[] {
	const nodes: Node[] = [];
	let textBuffer = "";

	const tokenRegex =
		/(-?)(author|channel|before|after|has|pinned|mentions):(\S*)|"([^"]*)"/g;
	let lastIndex = 0;
	let match;

	while ((match = tokenRegex.exec(query)) !== null) {
		const textBefore = query.slice(lastIndex, match.index);
		if (textBefore) textBuffer += textBefore;

		if (match[2]) {
			if (textBuffer) {
				nodes.push(schema.text(textBuffer));
				textBuffer = "";
			}

			const isNegated = !!match[1];
			const filterType = match[2];
			const value = match[3];

			if (filterType === "author") {
				const user = users2.cache.get(value);
				if (user) {
					nodes.push(
						schema.nodes.author.create({
							id: user.id,
							name: user.name,
							negated: isNegated,
						}),
					);
				} else {
					textBuffer += `${isNegated ? "-" : ""}author:${value}`;
				}
			} else if (filterType === "channel") {
				const thread = roomThreads().find((t) => t.id === value);
				if (thread) {
					nodes.push(
						schema.nodes.channel.create({
							id: thread.id,
							name: thread.name,
							negated: isNegated,
						}),
					);
				} else {
					textBuffer += `${isNegated ? "-" : ""}channel:${value}`;
				}
			} else if (filterType === "before") {
				nodes.push(
					schema.nodes.before.create({ date: value, negated: isNegated }),
				);
			} else if (filterType === "after") {
				nodes.push(
					schema.nodes.after.create({ date: value, negated: isNegated }),
				);
			} else if (filterType === "has") {
				nodes.push(schema.nodes.has.create({ value, negated: isNegated }));
			} else if (filterType === "pinned") {
				nodes.push(schema.nodes.pinned.create({ value, negated: isNegated }));
			} else if (filterType === "mentions") {
				nodes.push(
					schema.nodes.mentions.create({
						id: value,
						name: value,
						negated: isNegated,
					}),
				);
			}
		} else if (match[4] !== undefined) {
			textBuffer += match[0];
		}

		lastIndex = tokenRegex.lastIndex;
	}

	if (lastIndex < query.length) textBuffer += query.slice(lastIndex);
	if (textBuffer) nodes.push(schema.text(textBuffer));

	return nodes;
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

						const tokens = parseSearchQuery(text);
						for (const token of tokens) {
							const from = pos + token.from;
							const to = pos + token.to;
							const negatedClass = token.negated ? " filter-negated" : "";
							decorations.push(
								Decoration.inline(from, to, {
									class:
										`filter-token filter-${token.filterType}${negatedClass}`,
								}),
							);
						}

						const phraseRegex = /"([^"]*)"/g;
						let match;
						while ((match = phraseRegex.exec(text))) {
							const from = pos + match.index;
							const to = from + match[0].length;
							decorations.push(
								Decoration.inline(from, to, { class: "filter-phrase" }),
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
							const negatedText = match[0].trimStart();

							if (
								!negatedText.match(
									/^-(author|channel|before|after|has|pinned|mentions):/,
								)
							) {
								decorations.push(
									Decoration.inline(from, to, { class: "filter-negation" }),
								);
								decorations.push(
									Decoration.inline(from, from + 1, { class: "syn" }),
								);
							}
						}
					}
				});
				return DecorationSet.create(state.doc, decorations);
			},
		},
	});
}

const AutocompleteDropdown = (props: {
	filter: { type: string; query: string; negated?: boolean };
	channel?: ThreadT;
	room?: RoomT;
	onSelect: (node: Node) => void;
	onSelectFilter: (text: string) => void;
	hoveredIndex?: number;
	setHoveredIndex?: (index: number) => void;
	onItemsChange?: (
		items: { rawValue?: string; onSelect: () => void; isSeparator?: boolean }[],
		selectItem: (idx: number) => void,
	) => void;
}) => {
	const channels2 = useChannels2();
	const threadMembers2 = useThreadMembers2();
	const roomMembers2 = useRoomMembers2();
	const users2 = useUsers2();
	const roles = useRoles2();

	const roomThreads = () =>
		[...channels2.cache.values()].filter((c) =>
			c.room_id === ((props.channel?.room_id as any) ?? props.room?.id ?? "")
		);

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
		if (props.filter.type !== "author") return [];

		const query = props.filter.query.toLowerCase();
		const all_user_ids = [
			...new Set([...threadMemberIds(), ...roomMemberIds()]),
		];
		if (!query) return all_user_ids.slice(0, 10);

		const users = all_user_ids.map((id) => users2.cache.get(id)).filter(
			Boolean,
		) as User[];
		return users.filter((u) =>
			u.name.toLowerCase().includes(query) || u.id.toLowerCase().includes(query)
		).map((u) => u.id).slice(0, 10);
	});

	const channelSuggestions = createMemo(() => {
		const query = props.filter.query.toLowerCase();
		const threads = roomThreads() ?? [];
		if (!query) return threads.slice(0, 10);
		return threads.filter((t) =>
			t.name.toLowerCase().includes(query) || t.id.toLowerCase().includes(query)
		).slice(0, 10);
	});

	const hasFilterSuggestions = createMemo(() => {
		const query = props.filter.query.toLowerCase();
		const options = ["attachment", "image", "audio", "video", "link", "embed"];
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
		const all_user_ids = [
			...new Set([...threadMemberIds(), ...roomMemberIds()]),
		];

		const users = (all_user_ids.map((id) =>
			users2.cache.get(id)
		).filter(Boolean) as User[])
			.filter((u) =>
				u.name.toLowerCase().includes(query) ||
				u.id.toLowerCase().includes(query)
			)
			.map((u) =>
				({ id: `user-${u.id}`, name: u.name, type: "user" }) as Mentionable
			);

		const _roles = (roomRoles() ?? [])
			.filter((r) => r.name.toLowerCase().includes(query))
			.map((r) =>
				({
					id: `role-${r.id}`,
					name: r.name,
					type: "role" as const,
				}) as Mentionable
			);

		const special: Mentionable[] = [
			{ id: "everyone-room", name: "@room", type: "special" as const },
			{ id: "everyone-thread", name: "@thread", type: "special" as const },
		].filter((s) => (s.name as any).toLowerCase().includes(query));

		return [...users, ..._roles, ...special].slice(0, 10);
	});

	const onAuthorSelect = (user_id: string) => {
		const user = users2.cache.get(user_id);
		if (!user) return;
		props.onSelect(
			schema.nodes.author.create({
				id: user.id,
				name: user.name,
				negated: props.filter.negated,
			}),
		);
	};

	const onChannelSelect = (thread: ThreadT) => {
		props.onSelect(
			schema.nodes.channel.create({
				id: thread.id,
				name: thread.name,
				negated: props.filter.negated,
			}),
		);
	};

	const onHasSelect = (value: string) => {
		props.onSelect(
			schema.nodes.has.create({ value, negated: props.filter.negated }),
		);
	};

	const onPinnedSelect = (value: string) => {
		props.onSelect(
			schema.nodes.pinned.create({ value, negated: props.filter.negated }),
		);
	};

	const onMentionsSelect = (mentionable: Mentionable) => {
		props.onSelect(
			schema.nodes.mentions.create({
				id: mentionable.id,
				name: mentionable.name,
				negated: props.filter.negated,
			}),
		);
	};

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
		const query = props.filter.query.toLowerCase();
		const negated = props.filter.negated;
		if (!query) {
			return negated
				? allFilterSuggestions.map((f) => "-" + f)
				: allFilterSuggestions;
		}

		return allFilterSuggestions
			.filter((f) => f.toLowerCase().includes(query))
			.map((f) => negated ? "-" + f : f);
	});

	const recentSearches = createMemo(() => {
		if (props.filter.type === "filter" && props.filter.query === "") {
			return getRecentSearches();
		}
		return [];
	});

	const formatRecentSearch = (query: string) => {
		const parts: (string | { type: string; value: string })[] = [];
		let lastIndex = 0;
		const tokens = parseSearchQuery(query);

		const phraseRegex = /"([^"]*)"/g;
		const phrases: { from: number; to: number; text: string }[] = [];
		let match;
		while ((match = phraseRegex.exec(query)) !== null) {
			phrases.push({
				from: match.index,
				to: match.index + match[0].length,
				text: match[0],
			});
		}

		const negationRegex = /(^|\s)-\S+/g;
		const negations: { from: number; to: number; text: string }[] = [];
		while ((match = negationRegex.exec(query)) !== null) {
			const from = match.index + (match[1]?.length ?? 0);
			const to = from + match[0].length - (match[1]?.length ?? 0);
			const text = match[0].trimStart();

			if (!text.match(/^-(author|channel|before|after|has|pinned|mentions):/)) {
				negations.push({ from, to, text });
			}
		}

		type Segment = {
			from: number;
			to: number;
			type: string;
			value: string;
			filterType?: string;
			negated?: boolean;
		};
		const segments: Segment[] = [];

		for (const token of tokens) {
			const negationPrefixLen = token.negated ? 1 : 0;
			const filterNameStart = token.from + negationPrefixLen;
			const filterNameEnd = filterNameStart + token.filterType.length;
			const colonEnd = filterNameEnd + 1;

			if (token.negated) {
				segments.push({
					from: token.from,
					to: filterNameStart,
					type: "filter-syntax",
					value: "-",
				});
			}

			segments.push({
				from: filterNameStart,
				to: filterNameEnd,
				type: "filter-name",
				value: token.filterType,
				filterType: token.filterType,
				negated: token.negated,
			});

			segments.push({
				from: filterNameEnd,
				to: colonEnd,
				type: "filter-syntax",
				value: ":",
				filterType: token.filterType,
				negated: token.negated,
			});

			if (token.value) {
				segments.push({
					from: colonEnd,
					to: token.to,
					type: "filter-value",
					value: token.value,
					filterType: token.filterType,
					negated: token.negated,
				});
			}
		}

		for (const phrase of phrases) {
			segments.push({
				from: phrase.from,
				to: phrase.from + 1,
				type: "filter-syntax",
				value: '"',
			});
			if (phrase.text.length > 2) {
				segments.push({
					from: phrase.from + 1,
					to: phrase.to - 1,
					type: "filter-phrase-content",
					value: phrase.text.slice(1, -1),
				});
			}
			if (phrase.text.length > 1) {
				segments.push({
					from: phrase.to - 1,
					to: phrase.to,
					type: "filter-syntax",
					value: '"',
				});
			}
		}

		for (const negation of negations) {
			segments.push({
				from: negation.from,
				to: negation.from + 1,
				type: "filter-syntax",
				value: "-",
			});
			segments.push({
				from: negation.from + 1,
				to: negation.to,
				type: "filter-negation-content",
				value: negation.text.slice(1),
			});
		}

		segments.sort((a, b) => a.from - b.from);

		const merged: Segment[] = [];
		for (const seg of segments) {
			const overlaps = merged.find((m) =>
				!(seg.to <= m.from || seg.from >= m.to)
			);
			if (!overlaps) {
				merged.push(seg);
			} else if (
				seg.type.startsWith("filter-") && !overlaps.type.startsWith("filter-")
			) {
				const idx = merged.indexOf(overlaps);
				merged.splice(idx, 1, seg);
			}
		}
		merged.sort((a, b) => a.from - b.from);

		let pos = 0;
		for (const seg of merged) {
			if (seg.from > pos) parts.push(query.slice(pos, seg.from));

			if (seg.type === "filter-name") {
				parts.push({ type: "filter-name", value: seg.value });
			} else if (seg.type === "filter-syntax") {
				parts.push({ type: "filter-syntax", value: seg.value });
			} else if (seg.type === "filter-value") {
				const { filterType, value, negated } = seg;
				if (filterType === "author") {
					const user = users2.cache.get(value);
					parts.push({
						type: negated ? "filter-negated-value" : "filter-value",
						value: user?.name ?? value,
					});
				} else if (filterType === "channel") {
					const thread = roomThreads().find((t) => t.id === value);
					parts.push({
						type: negated ? "filter-negated-value" : "filter-value",
						value: thread?.name ?? value,
					});
				} else if (filterType === "mentions") {
					let displayName = value;
					if (value.startsWith("user-")) {
						const user = users2.cache.get(value.replace("user-", ""));
						displayName = user?.name ?? value.replace("user-", "");
					} else if (value.startsWith("role-")) {
						const role = roomRoles().find((r) =>
							r.id === value.replace("role-", "")
						);
						displayName = role?.name ?? value.replace("role-", "");
					} else if (value === "everyone-room") displayName = "@room";
					else if (value === "everyone-thread") displayName = "@thread";

					parts.push({
						type: negated ? "filter-negated-value" : "filter-value",
						value: displayName,
					});
				} else {
					parts.push({
						type: negated ? "filter-negated-value" : "filter-value",
						value,
					});
				}
			} else if (seg.type === "filter-phrase-content") {
				parts.push({ type: "filter-phrase-content", value: seg.value });
			} else if (seg.type === "filter-negation-content") {
				parts.push({ type: "filter-negation-content", value: seg.value });
			}

			pos = seg.to;
		}

		if (pos < query.length) parts.push(query.slice(pos));
		return parts;
	};

	const hasSuggestions = createMemo(() => {
		if (props.filter.type === "author") return authorSuggestions().length > 0;
		if (props.filter.type === "channel") return channelSuggestions().length > 0;
		if (props.filter.type === "has") return hasFilterSuggestions().length > 0;
		if (props.filter.type === "pinned") return pinnedSuggestions().length > 0;
		if (props.filter.type === "mentions") {
			return mentionsSuggestions().length > 0;
		}
		if (props.filter.type === "filter") {
			return filterSuggestions().length > 0 || recentSearches().length > 0;
		}
		return false;
	});

	const items = createMemo(() => {
		const type = props.filter.type;
		type LabelPart = string | { type: string; value: string };
		const result: {
			id: string;
			label: string | LabelPart[];
			rawValue?: string;
			onSelect: () => void;
			isSeparator?: boolean;
		}[] = [];

		if (type === "author") {
			authorSuggestions().forEach((user_id) => {
				const user = users2.cache.get(user_id);
				result.push({
					id: `author-${user_id}`,
					label: user?.name ?? user_id,
					onSelect: () => onAuthorSelect(user_id),
				});
			});
		} else if (type === "channel") {
			channelSuggestions().forEach((thread) => {
				result.push({
					id: `channel-${thread.id}`,
					label: thread.name,
					onSelect: () => onChannelSelect(thread),
				});
			});
		} else if (type === "has") {
			hasFilterSuggestions().forEach((value) => {
				result.push({
					id: `has-${value}`,
					label: value,
					onSelect: () => onHasSelect(value),
				});
			});
		} else if (type === "pinned") {
			pinnedSuggestions().forEach((value) => {
				result.push({
					id: `pinned-${value}`,
					label: value,
					onSelect: () => onPinnedSelect(value),
				});
			});
		} else if (type === "mentions") {
			mentionsSuggestions().forEach((mentionable) => {
				result.push({
					id: `mentions-${mentionable.id}`,
					label: mentionable.name,
					onSelect: () => onMentionsSelect(mentionable),
				});
			});
		} else if (type === "filter") {
			filterSuggestions().forEach((filter) => {
				result.push({
					id: `filter-${filter}`,
					label: filter,
					onSelect: () => props.onSelectFilter(filter),
				});
			});
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
						label: formatRecentSearch(search),
						rawValue: search,
						onSelect: () => props.onSelectFilter(search),
					});
				});
			}
		}

		return result;
	});

	const handleSelect = (idx: number) => {
		const its = items();
		const item = its[idx];
		if (item && !item.isSeparator) item.onSelect();
	};

	createEffect(() => props.onItemsChange?.(items(), handleSelect));

	return (
		<Show when={hasSuggestions()}>
			<div class="search-autocomplete">
				<ul>
					<For each={items()}>
						{(item, idx) => {
							const isHovered = () => idx() === (props.hoveredIndex ?? 0);
							const isSeparator = () => item.isSeparator;
							const renderLabel = () => {
								const label = item.label;
								if (Array.isArray(label)) {
									return label.map((part) => (typeof part === "string"
										? <span>{part}</span>
										: <span class={part.type}>{part.value}</span>)
									);
								}
								return label;
							};

							return (
								<Show
									when={!isSeparator()}
									fallback={
										<li class="autocomplete-separator">Recent Searches</li>
									}
								>
									<li
										id={item.id}
										class="autocomplete-item"
										classList={{ hovered: isHovered() }}
										onMouseDown={(e) => {
											e.preventDefault();
											item.onSelect();
										}}
										onMouseEnter={() => props.setHoveredIndex?.(idx())}
									>
										{renderLabel()}
									</li>
								</Show>
							);
						}}
					</For>
				</ul>
			</div>
		</Show>
	);
};

function getFilterFromSelection(
	state: EditorState,
): { type: string; query: string; negated?: boolean } | null {
	const { selection } = state;
	if (!selection.empty) return null;

	const $pos = state.doc.resolve(selection.from);
	const nodeBefore = $pos.nodeBefore;

	if (!nodeBefore) return { type: "filter", query: "" };
	if (!nodeBefore.isText) return null;

	const textBeforeCursor = nodeBefore.text!;

	const tokens = parseSearchQuery(textBeforeCursor);
	const lastToken = tokens[tokens.length - 1];
	if (lastToken && lastToken.to === textBeforeCursor.length) {
		return {
			type: lastToken.filterType,
			query: lastToken.value,
			negated: lastToken.negated,
		};
	}

	const partialFilterMatch = textBeforeCursor.match(
		/(-?)(author|channel|before|after|has|pinned|mentions):$/,
	);
	if (partialFilterMatch) {
		return {
			type: partialFilterMatch[2],
			query: "",
			negated: !!partialFilterMatch[1],
		};
	}

	if (textBeforeCursor.match(/\s$/)) return { type: "filter", query: "" };

	const wordMatch = textBeforeCursor.match(/(\S+)$/);
	if (wordMatch) {
		const word = wordMatch[1];
		const cleanWord = word.startsWith("-") ? word.slice(1) : word;
		if (cleanWord.includes(":")) return null;

		return { type: "filter", query: cleanWord, negated: word.startsWith("-") };
	}

	return { type: "filter", query: "" };
}

function autocompletePlugin(
	setFilter: (
		filter: { type: string; query: string; negated?: boolean } | null,
	) => void,
) {
	return new Plugin({
		state: {
			init: () => null,
			apply: (tr, value, _oldState, newState) => {
				if (tr.getMeta("skipAutocomplete")) {
					setFilter(null);
					return null;
				}
				if (!tr.docChanged && !tr.selectionSet) return value;

				setFilter(getFilterFromSelection(newState));
				return null;
			},
		},
	});
}

export const SearchInput = (
	props: { channel?: ThreadT; room?: RoomT; autofocus?: boolean },
) => {
	const users2 = useUsers2();
	const messagesService = useMessages2();
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
	const [activeFilter, setActiveFilter] = createSignal<
		{ type: string; query: string; negated?: boolean } | null
	>(null);
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

	const channelCtx = useChannel();
	const roomCtx = useRoom();
	const channels2 = useChannels2();

	const roomThreads = () =>
		[...channels2.cache.values()].filter((c) =>
			c.room_id === (props.channel?.room_id ?? props.room?.id ?? "")
		);

	const currentSearch = () => {
		if (props.channel) return channelCtx?.[0].search;
		if (props.room) return roomCtx?.[0].search;
		return undefined;
	};

	const updateSearch = (val: ChannelSearch | RoomSearch | undefined) => {
		if (props.channel && channelCtx) channelCtx[1]("search", val as any);
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
					mentionId === "everyone-room" || mentionId === "everyone-thread"
				) mentionSubquery.push(`metadata_fast.mentions_everyone:true`);
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
					recent.includes(text) && cleanText.length > 0 &&
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
						const user = users2.cache.get(id);
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
		if (items && items[hoveredIndex()]) {
			items[hoveredIndex()].scrollIntoView({ block: "nearest" });
		}
	});

	const editor = createBaseEditor({
		schema: schema as any,
		createState: (schema) => {
			let docContent: Node | undefined = undefined;
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
								const hasSelectableItems = items.length > 0 &&
									items.some((i) => !i.isSeparator);

								if (filterActive && hasSelectableItems) {
									if (event.key === "ArrowDown" || event.key === "ArrowUp") {
										setUserNavigated(true);
										event.preventDefault();
										setHoveredIndex((prev) => {
											const max = items.length - 1;
											let next = event.key === "ArrowDown"
												? (prev >= max ? 0 : prev + 1)
												: (prev <= 0 ? max : prev - 1);

											if (items[next]?.isSeparator) {
												next = event.key === "ArrowDown"
													? (next >= max ? 0 : next + 1)
													: (next <= 0 ? max : next - 1);
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
							width: `${(editorRef()?.offsetWidth || 0)}px`,
						}}
					>
						<AutocompleteDropdown
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
