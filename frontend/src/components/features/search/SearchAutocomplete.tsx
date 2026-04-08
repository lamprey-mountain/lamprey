import type { Node } from "prosemirror-model";
import type { User } from "sdk";
import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import {
	useChannels2,
	useRoles2,
	useRoomMembers2,
	useThreadMembers2,
	useUsers2,
} from "@/api";
import type { RoomT, ThreadT } from "../../../types";
import { schema } from "./schema";
import { getRecentSearches, parseSearchQuery } from "./utils";

export const SearchAutocomplete = (props: {
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
		[...channels2.cache.values()].filter(
			(c) =>
				c.room_id === ((props.channel?.room_id as any) ?? props.room?.id ?? ""),
		);

	const roomId = () => props.channel?.room_id ?? props.room?.id ?? null;
	const roomRoles = createMemo(() =>
		[...roles.cache.values()].filter((r) => r.room_id === roomId()),
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

		const users = all_user_ids
			.map((id) => users2.cache.get(id))
			.filter(Boolean) as User[];
		return users
			.filter(
				(u) =>
					u.name.toLowerCase().includes(query) ||
					u.id.toLowerCase().includes(query),
			)
			.map((u) => u.id)
			.slice(0, 10);
	});

	const channelSuggestions = createMemo(() => {
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

		const users = (
			all_user_ids.map((id) => users2.cache.get(id)).filter(Boolean) as User[]
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

		const _roles = (roomRoles() ?? [])
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
				? allFilterSuggestions.map((f) => `-${f}`)
				: allFilterSuggestions;
		}

		return allFilterSuggestions
			.filter((f) => f.toLowerCase().includes(query))
			.map((f) => (negated ? `-${f}` : f));
	});

	const recentSearches = createMemo(() => {
		if (props.filter.type === "filter" && props.filter.query === "") {
			return getRecentSearches();
		}
		return [];
	});

	const formatRecentSearch = (query: string) => {
		const parts: (string | { type: string; value: string })[] = [];
		const _lastIndex = 0;
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
			const overlaps = merged.find(
				(m) => !(seg.to <= m.from || seg.from >= m.to),
			);
			if (!overlaps) {
				merged.push(seg);
			} else if (
				seg.type.startsWith("filter-") &&
				!overlaps.type.startsWith("filter-")
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
						const role = roomRoles().find(
							(r) => r.id === value.replace("role-", ""),
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
									return label.map((part) =>
										typeof part === "string" ? (
											<span>{part}</span>
										) : (
											<span class={part.type}>{part.value}</span>
										),
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
