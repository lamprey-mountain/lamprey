import { createUniqueId, For, Match, Show, Switch } from "solid-js";
import { ChannelIcon } from "@/avatar/ChannelIcon";
import { Avatar } from "@/components/shared/User";
import type { AutocompleteItem } from "@/contexts/autocomplete";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useAutocompleteData } from "@/hooks/useAutocompleteData";
import { getTwemoji } from "@/lib/emoji";
import { getEmojiUrl } from "@/media/util";

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
	const listboxId = createUniqueId();
	const optionId = (i: number) => `${listboxId}-opt-${i}`;

	return (
		<Show when={state.visible && state.kind && filtered().length > 0}>
			<div
				class="autocomplete"
				role="listbox"
				id={listboxId}
				aria-label={`${state.kind?.type ?? "autocomplete"} suggestions`}
			>
				<header>
					<Show when={state.query} fallback={`list ${state.kind?.type}s`}>
						filter {state.kind?.type} matching "{state.query}"
					</Show>
				</header>
				<For each={filtered()}>
					{(result, i) => (
						<div
							id={optionId(i())}
							class="item"
							role="option"
							tabindex="-1"
							classList={{ hovered: i() === state.activeIndex }}
							aria-selected={i() === state.activeIndex}
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
									{(() => {
										const emoji = result.obj as Extract<
											AutocompleteItem,
											{ type: "emoji" }
										> & { char: string };
										return <span innerHTML={getTwemoji(emoji.char)}></span>;
									})()}
								</Match>
								<Match
									when={
										state.kind?.type === "emoji" && !isEmojiWithChar(result.obj)
									}
								>
									{(() => {
										const emoji = result.obj as Extract<
											AutocompleteItem,
											{ type: "emoji" }
										>;
										return (
											<img src={getEmojiUrl(emoji.id)} class="emoji-img" />
										);
									})()}
								</Match>
								<Match when={isCommand(result.obj)}>
									{(() => {
										const cmd = result.obj as Extract<
											AutocompleteItem,
											{ type: "command" }
										>;
										return (
											<div class="command">
												<div class="name">/{cmd.command}</div>
												<div class="description dim">{cmd.description}</div>
											</div>
										);
									})()}
								</Match>
								<Match when={isMentionUser(result.obj)}>
									{(() => {
										const userItem = result.obj as Extract<
											AutocompleteItem,
											{ type: "user" }
										>;
										return (
											<div class="mention-user">
												<Avatar user={userItem.user} pad={0} />
												<span>{userItem.name}</span>
											</div>
										);
									})()}
								</Match>
								<Match when={isMentionRole(result.obj)}>
									{(() => {
										const roleItem = result.obj as Extract<
											AutocompleteItem,
											{ type: "role" }
										>;
										return (
											<div class="mention-role">
												<span class="role-badge">#</span>
												<span>{roleItem.name}</span>
											</div>
										);
									})()}
								</Match>
								<Match when={isMentionEveryone(result.obj)}>
									<div class="everyone-mention">
										<span>@everyone</span>
									</div>
								</Match>
								<Match when={isChannel(result.obj)}>
									{(() => {
										const channelItem = result.obj as Extract<
											AutocompleteItem,
											{ type: "channel" }
										>;
										return (
											<>
												<ChannelIcon
													channel={channelItem.channel}
													style="width: 20px; height: 20px;"
												/>
												<span>{channelItem.name}</span>
											</>
										);
									})()}
								</Match>
								<Match when={true}>
									{("label" in result.obj
										? (result.obj as { label?: string; name?: string }).label
										: undefined) ?? (result.obj as { name?: string }).name}
								</Match>
							</Switch>
						</div>
					)}
				</For>
			</div>
		</Show>
	);
};
