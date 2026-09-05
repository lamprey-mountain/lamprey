import { useNavigate } from "@solidjs/router";
import type { Channel, Message, Room } from "sdk";
import { createMemo, For, Show } from "solid-js";
import { useChannels } from "@/api";
import { Dropdown } from "@/atoms/Dropdown";
import {
	type SearchSort,
	type SearchState,
	useSearch,
} from "@/contexts/search";
import { MessageView } from "./Message";
import { MessageToolbarMount } from "./MessageToolbar";
import { MessageToolbarProvider } from "./message-toolbar-context";

export const SearchResultsHeader = (props: {
	search: SearchState;
	searchId: string;
	clearSearch: () => void;
}) => {
	const searchCtx = useSearch();

	return (
		<header class="search-results-header">
			<Show when={!props.search.loading} fallback={<>Searching...</>}>
				{props.search.results?.approximate_total ?? 0} results
			</Show>
			<div style="flex:1"></div>
			<Dropdown
				required
				selected={props.search.sort ?? "newest"}
				options={[
					{ item: "newest", label: "Newest" },
					{ item: "oldest", label: "Oldest" },
					{ item: "relevancy", label: "Relevancy" },
				]}
				enableWheel={false}
				onSelect={(sort: SearchSort | null) => {
					if (!sort) return;
					searchCtx.setStates(props.searchId, "sort", sort);
				}}
			/>
			<button
				type="button"
				class="button"
				onClick={() => {
					if (props.searchId) {
						props.clearSearch();
					}
				}}
			>
				Clear
			</button>
		</header>
	);
};

export const SearchResults = (props: {
	channel?: Channel;
	room?: Room;
	search: SearchState;
}) => {
	const searchCtx = useSearch();
	const navigate = useNavigate();

	const searchId = createMemo(
		() => props.channel?.id ?? props.room?.id ?? "global",
	);

	const clearSearch = () => {
		// HACK: close search results
		// ideally, i'd delete the property, but solidjs doesnt seem to work with that
		searchCtx.setStates(searchId(), undefined as any);
	};

	const onResultClick = (message: Message) => {
		navigate(`/channel/${message.channel_id}/message/${message.id}`);
		const id = searchId();
		if (id) {
			clearSearch();
		}
	};

	return (
		<aside class="search-results search-results-sidebar">
			<MessageToolbarProvider>
				<SearchResultsHeader
					search={props.search}
					searchId={searchId()}
					clearSearch={clearSearch}
				/>
				<Show when={!props.search.loading}>
					<ul>
						<For each={props.search.results?.messages}>
							{(message, index) => {
								const prev = () => {
									const i = index();
									if (i > 0) return props.search.results?.messages[i - 1];
									return undefined;
								};
								return (
									<SearchResultItem
										message={message}
										prevMessage={prev()}
										onResultClick={onResultClick}
									/>
								);
							}}
						</For>
					</ul>
				</Show>
				<MessageToolbarMount />
			</MessageToolbarProvider>
		</aside>
	);
};

export const SearchResultItem = (props: {
	message: Message;
	prevMessage?: Message;
	onResultClick: (message: Message) => void;
}) => {
	const channels2 = useChannels();
	const channel = channels2.use(() => props.message.channel_id);
	const showHeader = () =>
		!props.prevMessage ||
		props.prevMessage.channel_id !== props.message.channel_id;

	return (
		<>
			<Show when={showHeader() && channel()}>
				<div style="padding: 4px 12px 0; font-weight: bold; color: var(--text-dim);">
					#{channel()?.name}
				</div>
			</Show>
			<li onClick={() => props.onResultClick(props.message)}>
				<MessageView message={props.message} separate={true} />
			</li>
		</>
	);
};
