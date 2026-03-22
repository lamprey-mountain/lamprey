import {
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import { useCtx } from "./context";
import { useAutocomplete } from "./contexts/autocomplete";
import { useApi } from "./api";
import { go } from "fuzzysort";
import { type Channel, type EmojiCustom, type User } from "sdk";
import { getEmojiUrl } from "./media/util";
import { Avatar } from "./User";
import twemoji from "twemoji";
import { type Command, useSlashCommands } from "./contexts/slash-commands";
import { type EmojiData, emojiResource } from "./emoji";

type UnicodeEmoji = {
	char: string;
	name: string;
	id: string;
	shortcodes: string[];
};

const getTwemoji = (unicode: string) => {
	return twemoji.parse(unicode, {
		base: "https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/",
		attributes: () => ({ loading: "lazy" }),
		folder: "svg",
		ext: ".svg",
	});
};

export const Autocomplete = () => {
	const ctx = useCtx();
	const api = useApi();

	const { state, setResults, navigate, select, setIndex } = useAutocomplete();

	const [allUsers, setAllUsers] = createSignal<User[]>([]);
	const [allChannels, setAllChannels] = createSignal<Channel[]>([]);
	const [allEmoji, setAllEmoji] = createSignal<(EmojiCustom | EmojiData)[]>([]);
	const [allCommands, setAllCommands] = createSignal<Command[]>([]);

	// Fetch data based on autocomplete type
	createEffect(() => {
		const kind = state.kind;
		if (!kind) return;

		if (kind.type === "mention") {
			const channel = api.channels.cache.get(kind.channelId);
			const roomId = channel?.room_id;

			const threadMembers = api.thread_members.list(() => kind.channelId)();
			const roomMembers = roomId
				? api.room_members.list(() => roomId)()
				: undefined;

			const userIds = new Set<string>();
			threadMembers?.items.forEach((m) => userIds.add(m.user_id));
			roomMembers?.items.forEach((m) => userIds.add(m.user_id));

			// Build user list from cache or use member data as fallback
			const users = [...userIds].map((id) => {
				const cachedUser = api.users.cache.get(id);
				if (cachedUser && cachedUser.id) {
					return cachedUser;
				}
				// Fallback: create a minimal user object from the member data
				// Find the member to get any available name info
				const member = threadMembers?.items.find((m) => m.user_id === id) ||
					roomMembers?.items.find((m) => m.user_id === id);
				return {
					id: id,
					name: member?.override_name || id,
				} as User;
			});
			setAllUsers(users);
		} else if (kind.type === "channel") {
			const channel = api.channels.cache.get(kind.channelId);
			const roomId = channel?.room_id;

			const channels = [...api.channels.cache.values()].filter(
				(c) => c.type !== "Category" && c.room_id === roomId,
			);
			setAllChannels(channels);
		} else if (kind.type === "emoji") {
			const channel = api.channels.cache.get(kind.channelId);
			const roomId = channel?.room_id;

			const combined: (EmojiCustom | EmojiData)[] = [];
			if (roomId) {
				// Get custom emoji from cache for this room
				const roomEmoji = [...api.emoji.cache.values()].filter(
					(e) => e.owner?.owner === "Room" && e.owner.room_id === roomId,
				);
				combined.push(...roomEmoji);
			}
			const unicodeEmoji = emojiResource();
			if (unicodeEmoji) {
				combined.push(...unicodeEmoji);
			}
			setAllEmoji(combined);
		} else if (kind.type === "command") {
			const slashCommands = useSlashCommands();
			const allCommands = slashCommands.getAll();
			const channel = api.channels.cache.get(kind.channelId);

			const filteredCommands = allCommands.filter((cmd) => {
				if (cmd.canUse) {
					return cmd.canUse(api, channel?.room_id ?? undefined, channel!);
				}
				return true;
			});

			setAllCommands(filteredCommands);
		}
	});

	// Filter results based on query
	const filtered = createMemo(() => {
		const kind = state.kind;
		if (!kind) return [];

		const query = state.query;
		const type = kind.type;

		let results: Fuzzysort.KeyResults<
			User | Channel | EmojiCustom | EmojiData | Command
		>;

		if (type === "mention") {
			results = go(query, allUsers(), {
				key: "name",
				limit: 10,
				all: true,
			});
		} else if (type === "channel") {
			results = go(query, allChannels(), {
				key: "name",
				limit: 10,
				all: true,
			});
		} else if (type === "emoji") {
			// Normalize emoji for search - custom emoji use 'name', unicode use 'label'
			const normalizedEmoji = allEmoji().map((e) => ({
				...e,
				searchLabel: "label" in e ? e.label : e.name,
			}));
			results = go(query, normalizedEmoji, {
				keys: ["searchLabel", "shortcodes"],
				limit: 10,
				all: true,
			}) as any;
		} else if (type === "command") {
			results = go(query, allCommands(), {
				key: "name",
				limit: 10,
				all: true,
			}) as any;
		} else {
			results = [] as any;
		}

		return results;
	});

	createEffect(() => {
		setResults(filtered().map((i) => i.obj));
	});

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
