import type { Node } from "prosemirror-model";
import type { User } from "sdk";
import { createEffect, createMemo, For, Show } from "solid-js";
import { ChannelIcon } from "@/avatar/ChannelIcon";
import { Avatar } from "@/avatar/UserAvatar";
import type { RoomT, ThreadT } from "@/types";
import { SEARCH_FILTERS, type SearchContext } from "./filters.config";
import { schema } from "./schema";
import { getRecentSearches, parseSearchQuery } from "./utils";

type LabelPart = string | { type: string; value: string; parts?: LabelPart[] };

export type AutocompleteItem = {
	id: string;
	label: string | LabelPart[];
	rawValue?: string;
	user?: User;
	channel?: ThreadT;
	onSelect: () => void;
	isSeparator?: boolean;
};

export const SearchAutocomplete = (props: {
	filter: { type: string; query: string; negated?: boolean };
	channel?: ThreadT;
	room?: RoomT;
	onSelect: (node: Node) => void;
	onSelectFilter: (text: string, isRecent?: boolean) => void;
	hoveredIndex?: number;
	setHoveredIndex?: (index: number) => void;
	searchContext: SearchContext;
	onItemsChange?: (
		items: AutocompleteItem[],
		selectItem: (idx: number, shouldSubmit: boolean) => void,
	) => void;
}) => {
	const _roomId = () => props.channel?.room_id ?? props.room?.id ?? null;

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

	const hasSuggestions = createMemo(() => {
		const type = props.filter.type;
		if (type === "filter") {
			return filterSuggestions().length > 0 || recentSearches().length > 0;
		}
		const def = SEARCH_FILTERS[type];
		if (!def) return false;
		return (
			def.getSuggestions(props.filter.query, props.searchContext).length > 0
		);
	});

	const items = createMemo(() => {
		const type = props.filter.type;
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
					onSelect: () => props.onSelectFilter(filter),
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
						label: formatRecentSearch(search, props.searchContext),
						rawValue: search,
						onSelect: () => props.onSelectFilter(search, true),
					});
				});
			}
		} else {
			// Delegate to the filter registry for suggestions
			const def = SEARCH_FILTERS[type];
			if (!def) return result;

			const suggestions = def.getSuggestions(
				props.filter.query,
				props.searchContext,
			);

			suggestions.forEach((item) => {
				result.push({
					id: item.id,
					label: item.label,
					user: item.user,
					channel: item.channel,
					onSelect: () => {
						// Create the PM node via the registry
						const astNode = {
							type,
							value: item.id.replace(`${type}-`, ""),
							name: item.label,
							negated: props.filter.negated ?? false,
						};
						const pmNode = def.toPMNode(astNode, schema as any);
						props.onSelect(pmNode);
					},
				});
			});
		}

		return result;
	});

	const handleSelect = (idx: number, _shouldSubmit: boolean) => {
		const its = items();
		const item = its[idx];
		if (item && !item.isSeparator) item.onSelect();
	};

	createEffect(() => {
		const its = items();
		props.onItemsChange?.(its, handleSelect);
	});

	return (
		<Show when={hasSuggestions()}>
			<div class="search-autocomplete">
				<ul>
					<For each={items()}>
						{(item, idx) => {
							const isHovered = () => idx() === (props.hoveredIndex ?? 0);
							const isSeparator = () => item.isSeparator;

							const renderLabel = (label: string | LabelPart[]): any => {
								if (typeof label === "string") return label;
								return label.map((part) => {
									if (typeof part === "string") return <span>{part}</span>;

									if (part.parts) {
										return (
											<span class={part.type}>{renderLabel(part.parts)}</span>
										);
									}

									return <span class={part.type}>{part.value}</span>;
								});
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
										classList={{
											hovered: isHovered(),
											"not-recent": !item.id.startsWith("recent"),
										}}
										onMouseDown={(e) => {
											e.preventDefault();
											item.onSelect();
										}}
										onMouseEnter={() => props.setHoveredIndex?.(idx())}
									>
										<Show
											when={item.user}
											fallback={
												<Show when={item.channel}>
													{(ch) => (
														<ChannelIcon
															channel={ch()}
															style="width: 16px; height: 16px; flex: none;"
														/>
													)}
												</Show>
											}
										>
											{(user) => <Avatar user={user()} />}
										</Show>
										{renderLabel(item.label)}
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

// ---------------------------------------------------------------------------
// Format a recent search string with syntax highlighting for display
// Uses the tokenizer for consistent parsing
// ---------------------------------------------------------------------------

function formatRecentSearch(
	query: string,
	ctx: SearchContext,
): (string | { type: string; value: string })[] {
	const tokens = parseSearchQuery(query);

	// Collect phrase spans
	const phraseRegex = /"([^"]*)"/g;
	const phrases: { from: number; to: number; text: string }[] = [];
	for (
		let m = phraseRegex.exec(query);
		m !== null;
		m = phraseRegex.exec(query)
	) {
		phrases.push({
			from: m.index,
			to: m.index + m[0].length,
			text: m[0],
		});
	}

	// Collect negated text spans (non-filter)
	const negationRegex = /(^|\s)-\S+/g;
	const negations: { from: number; to: number; text: string }[] = [];
	for (
		let m2 = negationRegex.exec(query);
		m2 !== null;
		m2 = negationRegex.exec(query)
	) {
		const from = m2.index + (m2[1]?.length ?? 0);
		const to = from + m2[0].length - (m2[1]?.length ?? 0);
		const text = m2[0].trimStart();

		if (!text.match(/^-(author|channel|before|after|has|pinned|mentions):/)) {
			negations.push({ from, to, text });
		}
	}

	// Build segments from filter tokens
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

	// Add phrase segments
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

	// Add negation segments
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

	// Sort and merge overlapping segments
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

	const parts: LabelPart[] = [];
	let pos = 0;

	// Use a while loop or reduce to handle grouping segments by filter
	for (let i = 0; i < merged.length; i++) {
		const seg = merged[i];
		if (seg.from > pos) parts.push(query.slice(pos, seg.from));

		if (seg.filterType) {
			// 3. Start building the "Atom" structure
			const groupType = seg.filterType;
			const isNegated = seg.negated;

			// Collect all segments belonging to this specific filter
			const filterSegments: Segment[] = [];
			while (
				i < merged.length &&
				merged[i].filterType === groupType &&
				merged[i].negated === isNegated
			) {
				filterSegments.push(merged[i]);
				i++;
			}
			i--; // Step back because the for-loop increments i

			const valueSeg = filterSegments.find((s) => s.type === "filter-value");
			let displayValue = valueSeg?.value ?? "";

			// 4. Resolve the display name (Usernames, Channel names, etc.)
			if (valueSeg) {
				if (groupType === "author") {
					displayValue =
						ctx.users.cache.get(valueSeg.value)?.name ?? valueSeg.value;
				} else if (groupType === "channel") {
					displayValue =
						ctx.roomThreads().find((t) => t.id === valueSeg.value)?.name ??
						valueSeg.value;
				} else if (groupType === "mentions") {
					const val = valueSeg.value;
					if (val.startsWith("user-")) {
						displayValue =
							ctx.users.cache.get(val.replace("user-", ""))?.name ?? val;
					} else if (val.startsWith("role-")) {
						const role = [...ctx.roles.cache.values()].find(
							(r) => r.id === val.replace("role-", ""),
						);
						displayValue = role?.name ?? val;
					} else if (val === "everyone-room") displayValue = "@room";
					else if (val === "everyone-thread") displayValue = "@thread";
				}
			}

			// 5. Create the nested "Atom" object
			parts.push({
				type: `filter-${groupType} filter-atom`, // Matches PM class
				value: "",
				parts: [
					{
						type: "filter-prefix",
						value: `${isNegated ? "-" : ""}${groupType}:`,
					},
					{
						type: isNegated ? "filter-negated-value" : "filter-value",
						value: displayValue,
					},
				],
			});

			pos = filterSegments[filterSegments.length - 1].to;
		} else {
			// Handle plain text/phrases/negations as before
			if (seg.type === "filter-phrase-content") {
				parts.push({ type: "filter-phrase-content", value: seg.value });
			} else if (seg.type === "filter-syntax") {
				parts.push({ type: "filter-syntax", value: seg.value });
			} else if (seg.type === "filter-negation-content") {
				parts.push({ type: "filter-negation-content", value: seg.value });
			}
			pos = seg.to;
		}
	}

	if (pos < query.length) parts.push(query.slice(pos));
	return parts;
}
