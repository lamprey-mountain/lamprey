import type { Node } from "prosemirror-model";
import type { User } from "sdk";
import {
	createMemo,
	createSignal,
	For,
	type JSX,
	Match,
	Show,
	Switch,
} from "solid-js";
import icSearch from "@/assets/search.png";
import { ChannelIcon } from "@/avatar/ChannelIcon";
import { Avatar } from "@/avatar/UserAvatar";
import type { RoomT, ThreadT } from "@/types";
import { SEARCH_FILTERS, type SearchContext } from "./filters.config";
import { FilterChipUI } from "./SearchFilterChip";
import type { LabelPart } from "./types";
import { formatRecentSearch } from "./utils";

const PRESET_SEARCHES = [
	// TODO
];

// filter metadata for the left column display
const FILTER_METADATA = [
	{
		key: "author",
		label: "From a specific user",
		desc: "author:user",
	},
	{
		key: "channel",
		label: "In a specific channel",
		desc: "in:channel",
	},
	{
		key: "has",
		label: "Has specific data",
		desc: "has:thing",
	},
	{
		key: "before",
		label: "Before a date",
		desc: "before:date",
	},
	{
		key: "after",
		label: "After a date",
		desc: "after:date",
	},
	{
		key: "pinned",
		label: "Pinned messages",
		desc: "pinned:true",
	},
	{
		key: "mentions",
		label: "Mentions someone",
		desc: "mentions:user",
	},
];

// "has" filter value descriptions for the right column values panel
const HAS_VALUE_DESCRIPTIONS = [
	{
		key: "image",
		label: "Images",
		desc: "Has attached image (png, jpeg, gif)",
	},
	{
		key: "video",
		label: "Videos",
		desc: "Has attached video (mp4, webm, mov)",
	},
	{
		key: "audio",
		label: "Audio",
		desc: "Has attached audio (mp3, ogg)",
	},
	{
		key: "attachment",
		label: "Attachments",
		desc: "Has any file attachment",
	},
	{
		key: "link",
		label: "Links",
		desc: "Content contains a link",
	},
	{
		key: "embed",
		label: "Embeds",
		desc: "Message has an embed preview",
	},
];

export type LabelType =
	| "author"
	| "channel"
	| "before"
	| "after"
	| "has"
	| "pinned"
	| "mentions"
	| "filter-phrase-content"
	| "filter-syntax"
	| "filter-negation-content";

export type AutocompleteItem = {
	id: string;
	label: string | LabelPart[];
	rawValue?: string;
	user?: User;
	channel?: ThreadT;
	onSelect: () => void;
	isSeparator?: boolean;
};

export type Completion =
	| { type: "recent_search"; query: string }
	| { type: "text"; text: string }
	| { type: "node"; node: Node };

export const SearchAutocomplete = (props: {
	filter: { type: string; query: string; negated?: boolean } | null;
	channel?: ThreadT;
	room?: RoomT;
	onCompletion: (c: Completion) => void;
	hoveredIndex?: number;
	setHoveredIndex?: (index: number) => void;
	searchContext: SearchContext;
	onPointerDown?: (e: PointerEvent) => void;
	onBlur?: (e: FocusEvent) => void;

	autocompleteItems: AutocompleteItem[];
	filterSuggestions: string[];
	recentSearches: string[];
	hasSuggestions: boolean;
}) => {
	const [hoveredFilter, setHoveredFilter] = createSignal<string | null>(null);

	const isShowingTwoColumn = () =>
		props.filter?.type === "filter" && props.filter.query === "";

	const handleFilterHover = (key: string | null) => {
		setHoveredFilter(key);
	};

	const handleFilterSelect = (key: string) => {
		props.onCompletion({ type: "text", text: `${key}:` });
	};

	const renderLabel = (label: LabelPart, isHovered: boolean): JSX.Element => {
		if (typeof label === "string") return label;
		if (Array.isArray(label))
			return label.map((part) => renderLabel(part, isHovered));

		return (
			<FilterChipUI
				type={label.type}
				label={label.value}
				user={label.user}
				channel={label.channel}
				negated={label.negated}
				animate={isHovered}
			/>
		);
	};

	return (
		<Show when={props.hasSuggestions}>
			<div
				class="search-autocomplete"
				onClick={(e) => {
					e.stopPropagation(), e.stopImmediatePropagation();
				}}
				onPointerDown={props.onPointerDown}
				onBlur={props.onBlur}
			>
				<Show
					when={isShowingTwoColumn()}
					fallback={
						<div class="side right">
							<ul>
								<For each={props.autocompleteItems}>
									{(item, idx) => {
										const isHovered = () => idx() === (props.hoveredIndex ?? 0);
										const isSeparator = () => item.isSeparator;

										return (
											<Show
												when={!isSeparator()}
												fallback={
													<li class="autocomplete-separator">
														Recent Searches
													</li>
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
														{(user) => (
															<Avatar user={user()} animate={isHovered()} />
														)}
													</Show>
													{renderLabel(item.label, isHovered())}
												</li>
											</Show>
										);
									}}
								</For>
							</ul>
						</div>
					}
				>
					<div class="side left">
						<h3 class="dim">filters</h3>
						<ul class="filters-list">
							<For each={FILTER_METADATA}>
								{(meta) => {
									const isHovered = () => hoveredFilter() === meta.key;
									const handleSelect = () => handleFilterSelect(meta.key);
									return (
										<li
											class="filter-item"
											classList={{ hovered: isHovered() }}
											onMouseEnter={() => handleFilterHover(meta.key)}
											onMouseDown={() => {
												handleSelect();
											}}
											onKeyDown={(e) => {
												if (e.key === "Enter" || e.key === " ") {
													e.preventDefault();
													handleSelect();
												}
											}}
										>
											<img
												class="icon"
												src={icSearch}
												alt=""
												aria-hidden="true"
											/>
											<div class="filter-label">{meta.label}</div>
											<div class="filter-desc dim">{meta.desc}</div>
										</li>
									);
								}}
							</For>
						</ul>
					</div>
					<Show when={hoveredFilter() || !props.filter?.query}>
						<div class="side right">
							<Show
								when={hoveredFilter()}
								fallback={
									<>
										<header class="presets-header">
											<h3 class="dim">preset searches</h3>
											<button
												class="dim link"
												type="button"
												onClick={() => console.log("todo")}
											>
												edit presets
											</button>
										</header>
										<ul class="presets-list">
											<li
												class="preset-item"
												onMouseDown={() =>
													props.onCompletion({
														type: "recent_search",
														query: `has:image channel:${props.channel?.name ?? "channel"}`,
													})
												}
											>
												<div class="preset-label">
													All images in this channel
												</div>
												<div class="preset-desc dim">
													{renderLabel(
														formatRecentSearch(
															`has:image channel:${props.channel?.name ?? "channel"}`,
															props.searchContext,
														),
														false,
													)}
												</div>
											</li>
											<li
												class="preset-item"
												onMouseDown={() =>
													props.onCompletion({
														type: "recent_search",
														query: `has:link channel:${props.channel?.name ?? "channel"}`,
													})
												}
											>
												<div class="preset-label">
													All links in this channel
												</div>
												<div class="preset-desc dim">
													{renderLabel(
														formatRecentSearch(
															`has:link channel:${props.channel?.name ?? "channel"}`,
															props.searchContext,
														),
														false,
													)}
												</div>
											</li>
										</ul>
										<Show when={props.recentSearches.length > 0}>
											<h3 class="dim recent-searches">recent searches</h3>
											<ul class="presets-list">
												<For each={props.recentSearches}>
													{(search, idx) => (
														<li
															class="preset-item"
															onMouseDown={() =>
																props.onCompletion({
																	type: "recent_search",
																	query: search,
																})
															}
															onMouseEnter={() =>
																props.setHoveredIndex?.(idx())
															}
														>
															<div class="preset-label">
																{renderLabel(
																	formatRecentSearch(
																		search,
																		props.searchContext,
																	),
																	idx() === props.hoveredIndex,
																)}
															</div>
														</li>
													)}
												</For>
											</ul>
										</Show>
										<Show when={props.recentSearches.length === 0}>
											<div
												class="dim"
												style="text-align: center; padding: 16px;"
											>
												no recent searches
											</div>
										</Show>
									</>
								}
							>
								{(filterKey) => (
									<FilterValuesPanel
										filterKey={filterKey()}
										searchContext={props.searchContext}
										onSelect={(value) => {
											props.onCompletion({
												type: "text",
												text: `${filterKey()}:${value}`,
											});
										}}
									/>
								)}
							</Show>
						</div>
					</Show>
				</Show>
			</div>
		</Show>
	);
};

// ---------------------------------------------------------------------------
// Filter Values Panel - shows when a filter is hovered in two-column mode
// ---------------------------------------------------------------------------

const FilterValuesPanel = (props: {
	filterKey: string;
	searchContext: SearchContext;
	onSelect: (value: string) => void;
}) => {
	const renderHasValues = () => {
		return (
			<ul class="filters-list">
				<For each={HAS_VALUE_DESCRIPTIONS}>
					{(item) => {
						const handleSelect = () => props.onSelect(item.key);
						return (
							<li
								class="filter-item"
								onMouseDown={() => handleSelect()}
								onKeyDown={(e) => {
									if (e.key === "Enter" || e.key === " ") {
										e.preventDefault();
										handleSelect();
									}
								}}
							>
								<img class="icon" src={icSearch} alt="" aria-hidden="true" />
								<div class="filter-label">{item.label}</div>
								<div class="filter-desc dim">{item.desc}</div>
							</li>
						);
					}}
				</For>
			</ul>
		);
	};

	const renderPinnedValues = () => {
		const options = ["true", "false"];
		return (
			<ul class="filters-list">
				<For each={options}>
					{(value) => (
						<li
							class="filter-item"
							onMouseDown={() => props.onSelect(value)}
							onKeyDown={(e) => {
								if (e.key === "Enter" || e.key === " ") {
									e.preventDefault();
									props.onSelect(value);
								}
							}}
						>
							<div class="filter-label">{value}</div>
						</li>
					)}
				</For>
			</ul>
		);
	};

	const renderAuthorValues = () => {
		const ctx = props.searchContext;
		const allIds = [
			...new Set([
				...(ctx.channel
					? [...ctx.threadMembers.cache.entries()]
							.filter(([key]) => key.startsWith(`${ctx.channel?.id}:`))
							.map(([, member]) => member.user_id)
					: []),
				...(ctx.roomId
					? [...ctx.roomMembers.cache.entries()]
							.filter(([key]) => key.startsWith(`${ctx.roomId}:`))
							.map(([, member]) => member.user_id)
					: []),
			]),
		];
		const users = allIds
			.map((id) => ctx.users.cache.get(id))
			.filter((u): u is NonNullable<typeof u> => Boolean(u))
			.slice(0, 10);

		return (
			<ul class="filters-list">
				<For each={users}>
					{(user) => (
						<li
							class="filter-item"
							onMouseDown={() => props.onSelect(user.id)}
							onKeyDown={(e) => {
								if (e.key === "Enter" || e.key === " ") {
									e.preventDefault();
									props.onSelect(user.id);
								}
							}}
						>
							<Avatar user={user} />
							<div class="filter-label">{user.name}</div>
						</li>
					)}
				</For>
			</ul>
		);
	};

	const renderChannelValues = () => {
		const threads = props.searchContext.roomThreads();
		return (
			<ul class="filters-list">
				<For each={threads}>
					{(thread) => (
						<li
							class="filter-item"
							onClick={() => props.onSelect(thread.id)}
							onKeyDown={(e) => {
								if (e.key === "Enter" || e.key === " ") {
									e.preventDefault();
									props.onSelect(thread.id);
								}
							}}
						>
							<ChannelIcon
								channel={thread}
								style="width: 16px; height: 16px; flex: none;"
							/>
							<div class="filter-label">{thread.name}</div>
						</li>
					)}
				</For>
			</ul>
		);
	};

	return (
		<>
			<h3 class="dim">values</h3>
			<Switch>
				<Match when={props.filterKey === "has"}>{renderHasValues()}</Match>
				<Match when={props.filterKey === "pinned"}>
					{renderPinnedValues()}
				</Match>
				<Match when={props.filterKey === "author"}>
					{renderAuthorValues()}
				</Match>
				<Match when={props.filterKey === "channel"}>
					{renderChannelValues()}
				</Match>
				<Match when={true}>
					<div class="dim" style="text-align: center; padding: 16px;">
						type to search...
					</div>
				</Match>
			</Switch>
		</>
	);
};
