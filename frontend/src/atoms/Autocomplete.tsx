import type { Channel } from "sdk";
import { For, Match, Show, Switch } from "solid-js";
import { ChannelIcon } from "../avatar/ChannelIcon";
import type { AutocompleteItem } from "../contexts/autocomplete";
import { useAutocomplete } from "../contexts/autocomplete";
import { getTwemoji } from "../emoji";
import { useAutocompleteData } from "../hooks/useAutocompleteData";
import { getEmojiUrl } from "../media/util";
import { Avatar } from "../User";

function isEmojiWithChar(
	item: AutocompleteItem,
): item is AutocompleteItem & { char: string } {
	return "char" in item;
}

function isCommand(
	item: AutocompleteItem,
): item is Extract<AutocompleteItem, { type: "command" }> {
	return item.type === "command";
}

function isMentionUser(
	item: AutocompleteItem,
): item is Extract<AutocompleteItem, { type: "user" }> {
	return item.type === "user";
}

function isMentionRole(
	item: AutocompleteItem,
): item is Extract<AutocompleteItem, { type: "role" }> {
	return item.type === "role";
}

function isMentionEveryone(
	item: AutocompleteItem,
): item is Extract<AutocompleteItem, { type: "everyone" }> {
	return item.type === "everyone";
}

function isChannel(
	item: AutocompleteItem,
): item is Extract<AutocompleteItem, { type: "channel" }> {
	return item.type === "channel";
}

export const Autocomplete = () => {
	const { state, select, setIndex } = useAutocomplete();
	const { filtered } = useAutocompleteData();

	return (
		<Show when={state.visible && state.kind && filtered().length > 0}>
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
								<Match
									when={
										state.kind?.type === "emoji" && isEmojiWithChar(result.obj)
									}
								>
									<span innerHTML={getTwemoji(result.obj.char)}></span>
								</Match>
								<Match
									when={
										state.kind?.type === "emoji" && !isEmojiWithChar(result.obj)
									}
								>
									<img src={getEmojiUrl(result.obj.id)} class="emoji-img" />
								</Match>
								<Match when={isCommand(result.obj)}>
									<div class="command">
										<div class="name">/{result.obj.command}</div>
										<div class="description dim">{result.obj.description}</div>
									</div>
								</Match>
								<Match when={isMentionUser(result.obj)}>
									<div class="mention-user">
										<Avatar user={result.obj.user} pad={0} />
										<span>{result.obj.name}</span>
									</div>
								</Match>
								<Match when={isMentionRole(result.obj)}>
									<div class="mention-role">
										<span class="role-badge">#</span>
										<span>{result.obj.name}</span>
									</div>
								</Match>
								<Match when={isMentionEveryone(result.obj)}>
									<div class="everyone-mention">
										<span>@everyone</span>
									</div>
								</Match>
								<Match when={isChannel(result.obj)}>
									<ChannelIcon
										channel={result.obj}
										style="width: 20px; height: 20px;"
									/>
									<span>{result.obj.name}</span>
								</Match>
								<Match when={true}>
									{"label" in result.obj
										? ((result.obj as any).label ?? (result.obj as any).name)
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
