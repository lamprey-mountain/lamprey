import { For, Match, Show, Switch } from "solid-js";
import { useAutocomplete } from "./contexts/autocomplete";
import { type EmojiCustom, type User } from "sdk";
import { getEmojiUrl } from "./media/util";
import { Avatar } from "./User";
import { type EmojiData, getTwemoji } from "./emoji";
import { type Command } from "./contexts/slash-commands";
import { useAutocompleteData } from "./hooks/useAutocompleteData";

export const Autocomplete = () => {
	const { state, select, setIndex } = useAutocomplete();
	const { filtered } = useAutocompleteData();

	return (
		<Show
			when={state.visible && state.kind &&
				filtered().length > 0}
		>
			<div class="autocomplete">
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
										"avatar" in result.obj}
								>
									<div class="mention-user">
										<Avatar user={result.obj as User} pad={0} />
										<span>{(result.obj as User).name}</span>
									</div>
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
