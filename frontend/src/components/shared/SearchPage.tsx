import { useNavigate, useSearchParams } from "@solidjs/router";
import type { Message } from "sdk";
import { For, Show } from "solid-js";
import { useSearch } from "@/contexts/search";
import { MessageToolbarMount } from "../features/chat/MessageToolbar";
import { MessageToolbarProvider } from "../features/chat/message-toolbar-context";
import {
	SearchResultItem,
	SearchResultsHeader,
} from "../features/chat/SearchResults";
import { SearchInput } from "../features/search";

type SearchParams = {
	q: string;
};

export const SearchPage = () => {
	const [search] = useSearchParams<SearchParams>();
	const searchCtx = useSearch();
	const navigate = useNavigate();

	const searchId = "global";

	const clearSearch = () => {
		searchCtx.setStates(searchId, undefined as any);
	};

	const onResultClick = (message: Message) => {
		navigate(`/channel/${message.channel_id}/message/${message.id}`);
		clearSearch();
	};

	return (
		<>
			<header class="header chat-header">
				<b>Search</b>
			</header>
			<div class="search-page">
				<SearchInput autofocus value={search.q ?? ""} />

				<Show when={searchCtx.states["global"]}>
					{(s) => (
						<MessageToolbarProvider>
							<SearchResultsHeader
								search={s()}
								searchId={searchId}
								clearSearch={clearSearch}
							/>
							<div class="search-results">
								<Show when={!s().loading}>
									<ul>
										<For each={s().results?.messages}>
											{(message, index) => {
												const prev = () => {
													const i = index();
													if (i > 0) return s().results?.messages[i - 1];
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
							</div>
							<MessageToolbarMount />
						</MessageToolbarProvider>
					)}
				</Show>
			</div>
		</>
	);
};
