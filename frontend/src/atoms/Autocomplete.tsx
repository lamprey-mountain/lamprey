import { For, Match, Show, Switch } from "solid-js";
import { useAutocomplete } from "../contexts/autocomplete";
import { type Channel, type EmojiCustom } from "sdk";
import { getEmojiUrl } from "../media/util";
import { Avatar } from "../User";
import { type EmojiData, getTwemoji } from "../emoji";
import { type Command } from "../contexts/slash-commands";
import { useAutocompleteData } from "../hooks/useAutocompleteData";
import type { AutocompleteMentionItem } from "../contexts/autocomplete";
import { ChannelIcon } from "../avatar/ChannelIcon";

export const Autocomplete = () => {
	const { state, select, setIndex } = useAutocomplete();
	const { filtered } = useAutocompleteData();

	return (
		<Show
			when={state.visible && state.kind &&
				filtered().length > 0}
		>
			<div class="autocomplete">
				<header>
					<Show when={state.query} fallback={`list ${state.kind?.type}s`}>
						filter {state.kind?.type} matching "{state.query}"
					</Show>
				</header>
				<For each={filtered()}>
					{(result, i) => (
						<div
							class="item"
							classList={{ hovered: i() === state.activeIndex }}
							onMouseEnter={() => setIndex(i())}
							onMouseDown={(e) => {
								e.preventDefault();
								setIndex(i());
								select();
							}}
						>
							<Switch>
								<Match when={"char" in result.obj}>
									<span
										innerHTML={getTwemoji((result.obj as EmojiData).char)}
									>
									</span>
								</Match>
								<Match
									when={state.kind?.type === "emoji" && !("char" in result.obj)}
								>
									<img
										src={getEmojiUrl((result.obj as EmojiCustom).id)}
										class="emoji-img"
									/>
								</Match>
								<Match when={state.kind?.type === "command"}>
									<div class="command">
										<div class="name">/{(result.obj as Command).name}</div>
										<div class="description dim">
											{(result.obj as Command).description}
										</div>
									</div>
								</Match>
								<Match
									when={state.kind?.type === "mention" &&
										(result.obj as AutocompleteMentionItem).type === "user"}
								>
									<div class="mention-user">
										<Avatar
											user={(result.obj as AutocompleteMentionItem).user}
											pad={0}
										/>
										<span>{(result.obj as AutocompleteMentionItem).name}</span>
									</div>
								</Match>
								<Match
									when={state.kind?.type === "mention" &&
										(result.obj as AutocompleteMentionItem).type === "role"}
								>
									<div class="mention-role">
										<span class="role-badge">#</span>
										<span>{(result.obj as AutocompleteMentionItem).name}</span>
									</div>
								</Match>
								<Match
									when={state.kind?.type === "mention" &&
										(result.obj as AutocompleteMentionItem).type === "everyone"}
								>
									<div class="everyone-mention">
										<span>@everyone</span>
									</div>
								</Match>
								<Match when={state.kind?.type === "channel"}>
									<ChannelIcon
										channel={result.obj as Channel}
										style="width: 20px; height: 20px;"
									/>
									<span>{(result.obj as Channel).name}</span>
								</Match>
								<Match when={true}>
									{"label" in result.obj
										? result.obj.label ?? result.obj.name
										: result.obj.name}
								</Match>
							</Switch>
						</div>
					)}
				</For>
			</div>
		</Show>
	);
};
