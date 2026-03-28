import { For, Match, Show, Switch } from "solid-js";
import { useAutocomplete } from "../contexts/autocomplete";
import { type Channel, type EmojiCustom, type User } from "sdk";
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
								<Match when={"char" in (result.obj as any)}>
									<span
										innerHTML={getTwemoji((result.obj as any).char)}
									>
									</span>
								</Match>
								<Match
									when={state.kind?.type === "emoji" &&
										!("char" in (result.obj as any))}
								>
									<img
										src={getEmojiUrl((result.obj as any).id)}
										class="emoji-img"
									/>
								</Match>
								<Match when={state.kind?.type === "command"}>
									<div class="command">
										<div class="name">/{(result.obj as any).name}</div>
										<div class="description dim">
											{(result.obj as any).description}
										</div>
									</div>
								</Match>
								<Match
									when={state.kind?.type === "mention" &&
										(result.obj as any).type === "user"}
								>
									<div class="mention-user">
										<Avatar
											user={(result.obj as any).user}
											pad={0}
										/>
										<span>
											{(result.obj as any).name}
										</span>
									</div>
								</Match>
								<Match
									when={state.kind?.type === "mention" &&
										(result.obj as any).type === "role"}
								>
									<div class="mention-role">
										<span class="role-badge">#</span>
										<span>
											{(result.obj as any).name}
										</span>
									</div>
								</Match>
								<Match
									when={state.kind?.type === "mention" &&
										(result.obj as any).type === "everyone"}
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
									{"label" in (result.obj as any)
										? (result.obj as any).label ?? (result.obj as any).name
										: (result.obj as any).name}
								</Match>
							</Switch>
						</div>
					)}
				</For>
			</div>
		</Show>
	);
};
