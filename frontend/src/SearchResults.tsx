import { useNavigate } from "@solidjs/router";
import type { Channel, Message, Room } from "sdk";
import { For, Show } from "solid-js";
import { useChannel } from "./channelctx";
import type { ChannelSearch } from "./context";
import { useRoom } from "./contexts/room";
import { MessageView } from "./Message";
import { useChannels2 } from "./api";

export const SearchResults = (props: {
	channel?: Channel;
	room?: Room;
	search: ChannelSearch;
}) => {
	const channelCtx = useChannel();
	const roomCtx = useRoom();
	const navigate = useNavigate();

	const searchId = () => props.channel?.id ?? props.room?.id;

	const clearSearch = () => {
		if (props.channel && channelCtx) {
			channelCtx[1]("search", undefined);
		} else if (props.room && roomCtx) {
			roomCtx[1]("search", undefined);
		}
	};

	const onResultClick = (message: Message) => {
		navigate(`/channel/${message.channel_id}/message/${message.id}`);
		const id = searchId();
		if (id) {
			clearSearch();
		}
	};

	return (
		<aside class="search-results">
			<header>
				<Show when={!props.search.loading} fallback={<>Searching...</>}>
					{props.search.results?.approximate_total ?? 0} results
				</Show>
				<button
					onClick={() => {
						const id = searchId();
						if (id) {
							clearSearch();
						}
					}}
				>
					Clear
				</button>
			</header>
			<Show when={!props.search.loading}>
				<ul>
					<For each={props.search.results?.messages}>
						{(message, index) => {
							const prev = () => {
								const i = index();
								if (i > 0) return props.search.results!.messages[i - 1];
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
		</aside>
	);
};

export const SearchResultItem = (props: {
	message: Message;
	prevMessage?: Message;
	onResultClick: (message: Message) => void;
}) => {
	const channels2 = useChannels2();
	const channel = channels2.use(() => props.message.channel_id);
	const showHeader = () =>
		!props.prevMessage ||
		props.prevMessage.channel_id !== props.message.channel_id;

	return (
		<>
			<Show when={showHeader() && channel()}>
				<div style="padding: 4px 12px 0; font-weight: bold; color: var(--text-dim);">
					#{channel()!.name}
				</div>
			</Show>
			<li onClick={() => props.onResultClick(props.message)}>
				<MessageView message={props.message} separate={true} />
			</li>
		</>
	);
};
