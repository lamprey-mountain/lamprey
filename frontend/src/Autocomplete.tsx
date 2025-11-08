import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	For,
	Match,
	on,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import { useCtx } from "./context";
import { useApi } from "./api";
import { go } from "fuzzysort";
import { type Channel, type EmojiCustom, type User } from "sdk";
import { getEmojiUrl } from "./media/util";
import twemoji from "twemoji";
import { type Command, commands } from "./slash-commands";
import { canUseCommand } from "./hooks/useCommandPermissions";

type Emoji = {
	group?: number;
	label: string;
	hexcode: string;
	order: number;
	unicode: string;
	tags?: string[];
	shortcode?: string | string[];
};

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

	const [unicodeEmoji] = createResource(async () => {
		const [
			{ default: emojis },
			{ default: shortJoypixels },
			{ default: shortEmojibase },
		] = await Promise.all([
			import("emojibase-data/en/compact.json"),
			import("emojibase-data/en/shortcodes/joypixels.json"),
			import("emojibase-data/en/shortcodes/emojibase.json"),
		]);

		const getShortcodes = (hex: string) => {
			const codes1 = (shortJoypixels as Record<string, string | string[]>)[hex];
			const codes2 = (shortEmojibase as Record<string, string | string[]>)[hex];
			const all_codes = [];
			if (codes1) {
				all_codes.push(...(Array.isArray(codes1) ? codes1 : [codes1]));
			}
			if (codes2) {
				all_codes.push(...(Array.isArray(codes2) ? codes2 : [codes2]));
			}
			return all_codes;
		};

		return (emojis as any[]).map((e: any) => ({
			char: e.unicode,
			name: e.label,
			id: `unicode:${e.label.replace(/ /g, "_")}`,
			shortcodes: getShortcodes(e.hexcode) ?? [],
		}));
	});

	const [allUsers, setAllUsers] = createSignal<User[]>([]);
	const [allChannels, setAllChannels] = createSignal<Channel[]>([]);
	const [allEmoji, setAllEmoji] = createSignal<(EmojiCustom | UnicodeEmoji)[]>(
		[],
	);
	const [allCommands, setAllCommands] = createSignal<Command[]>([]);

	createEffect(on(ctx.autocomplete, (state) => {
		if (state?.type === "mention") {
			const channelId = state.channelId;
			const channel = api.channels.cache.get(channelId);
			const roomId = channel?.room_id;

			const threadMembers = api.thread_members.list(() => channelId)();
			const roomMembers = roomId
				? api.room_members.list(() => roomId)()
				: undefined;

			const userIds = new Set<string>();
			threadMembers?.items.forEach((m) => userIds.add(m.user_id));
			roomMembers?.items.forEach((m) => userIds.add(m.user_id));

			const users = [...userIds].map((id) => api.users.cache.get(id)).filter(
				Boolean,
			) as User[];
			setAllUsers(users);
		} else if (state?.type === "channel") {
			const channels = [...api.channels.cache.values()].filter(
				(c) => c.type !== "Category",
			);
			setAllChannels(channels);
		} else if (state?.type === "emoji") {
			const channelId = state.channelId;
			const channel = api.channels.cache.get(channelId);
			const roomId = channel?.room_id;

			const customEmojiResource = roomId
				? api.emoji.list(() => roomId!)
				: undefined;
			const customEmoji = customEmojiResource
				? customEmojiResource()
				: undefined;

			const combined = [];
			if (customEmoji?.items) {
				combined.push(...customEmoji.items);
			}
			if (unicodeEmoji()) {
				combined.push(...(unicodeEmoji() as any));
			}
			setAllEmoji(combined);
		} else if (state?.type === "command") {
			const channel = api.channels.cache.get(state.channelId);
			const filteredCommands = commands.filter((cmd) =>
				canUseCommand(api, channel?.room_id, channel, cmd.name)
			);
			setAllCommands(filteredCommands);
		}
	}));

	const [filtered, setFiltered] = createSignal<
		Fuzzysort.KeyResult<User | Channel | EmojiCustom | UnicodeEmoji | Command>[]
	>([]);
	const [hoveredIndex, setHoveredIndex] = createSignal(0);
	const hovered = () => filtered()[hoveredIndex()]?.obj;

	createEffect(() => {
		const state = ctx.autocomplete();
		let results: Fuzzysort.KeyResults<
			User | Channel | EmojiCustom | UnicodeEmoji | Command
		>;
		if (state?.type === "mention") {
			results = go(state.query, allUsers(), {
				key: "name",
				limit: 10,
				all: true,
			});
		} else if (state?.type === "channel") {
			results = go(state.query, allChannels(), {
				key: "name",
				limit: 10,
				all: true,
			});
		} else if (state?.type === "emoji") {
			results = go(state.query, allEmoji(), {
				keys: ["name", "shortcodes"],
				limit: 10,
				all: true,
			});
		} else if (state?.type === "command") {
			results = go(state.query, allCommands(), {
				key: "name",
				limit: 10,
				all: true,
			});
		} else {
			results = [];
		}
		setFiltered(results as any);
		if (hoveredIndex() >= results.length) {
			setHoveredIndex(0);
		}
	});

	const select = (
		item: User | Channel | EmojiCustom | UnicodeEmoji | Command,
	) => {
		const state = ctx.autocomplete();
		if (state) {
			if (state.type === "emoji") {
				const name = item.name;
				const id = item.id;
				const char = "char" in item ? item.char : undefined;
				state.onSelect(id, name, char);
			} else if (state.type === "command") {
				state.onSelect(item.name);
			} else {
				state.onSelect(item.id, item.name);
			}
			ctx.setAutocomplete(null);
		}
	};

	const onKeyDown = (e: KeyboardEvent) => {
		if (!ctx.autocomplete()) return;

		if (e.key === "ArrowUp") {
			e.preventDefault();
			e.stopPropagation();
			setHoveredIndex((i) => (i - 1 + filtered().length) % filtered().length);
		} else if (e.key === "ArrowDown") {
			e.preventDefault();
			e.stopPropagation();
			setHoveredIndex((i) => (i + 1) % filtered().length);
		} else if (e.key === "Enter" || e.key === "Tab") {
			e.preventDefault();
			e.stopPropagation();
			const item = hovered();
			if (item) {
				select(item);
			}
		} else if (e.key === "Escape") {
			e.preventDefault();
			e.stopPropagation();
			ctx.setAutocomplete(null);
		}
	};

	createEffect(() => {
		if (ctx.autocomplete()) {
			document.addEventListener("keydown", onKeyDown, { capture: true });
			onCleanup(() => {
				document.removeEventListener("keydown", onKeyDown, { capture: true });
			});
		}
	});

	return (
		<Show when={ctx.autocomplete() && filtered().length > 0}>
			<div class="autocomplete">
				<For each={filtered()}>
					{(result, i) => (
						<div
							class="item"
							classList={{ hovered: i() === hoveredIndex() }}
							onMouseEnter={() => setHoveredIndex(i())}
							onMouseDown={(e) => {
								e.preventDefault();
								select(result.obj);
							}}
						>
							<Switch>
								<Match when={"char" in result.obj}>
									<span
										innerHTML={getTwemoji((result.obj as UnicodeEmoji).char)}
									>
									</span>
								</Match>
								<Match when={"media_id" in result.obj}>
									<img
										src={getEmojiUrl((result.obj as EmojiCustom).id)}
										class="emoji-img"
									/>
								</Match>
								<Match when={ctx.autocomplete()?.type === "command"}>
									<div class="command">
										<div class="name">{(result.obj as Command).name}</div>
										<div class="description dim">
											{(result.obj as Command).description}
										</div>
									</div>
								</Match>
								<Match when={true}>
									{result.obj.name}
								</Match>
							</Switch>
						</div>
					)}
				</For>
			</div>
		</Show>
	);
};
