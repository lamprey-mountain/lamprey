import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import twemoji from "twemoji";
import fuzzysort from "fuzzysort";
import { Search } from "./atoms/Search";
import icEmojiActivities from "./assets/emoji-activities.png";
import icEmojiFaces from "./assets/emoji-faces.png";
import icEmojiFlags from "./assets/emoji-flags.png";
import icEmojiFood from "./assets/emoji-food.png";
import icEmojiNature from "./assets/emoji-nature.png";
import icEmojiObjects from "./assets/emoji-objects.png";
import icEmojiPeople from "./assets/emoji-people.png";
import icEmojiPlaces from "./assets/emoji-places.png";
import icEmojiSymbols from "./assets/emoji-symbols.png";
import { useApi } from "./api";
import { getThumbFromId } from "./media/util";
import { RoomIcon } from "./User";
import type { EmojiCustom, Room } from "sdk";

type Emoji = {
	group?: number;
	label: string;
	hexcode: string;
	order: number;
	unicode: string;
	tags?: string[];
	shortcode?: string | string[];
};

type UnifiedEmoji = {
	type: "standard" | "custom";
	label: string;
	unicode?: string;
	hexcode?: string;
	order?: number;
	id?: string;
	media_id?: string;
	animated?: boolean;
	room_id?: string;
};

type EmojiGroup = {
	id: string | number;
	name: string;
	icon?: string;
	room?: Room;
	emojis: UnifiedEmoji[];
};

const parseEmoji = async (): Promise<EmojiGroup[]> => {
	const { default: emojis } = await import("emojibase-data/en/compact.json");
	const groups: Emoji[][] = [[], [], [], [], [], [], [], [], [], []];
	for (let emoji of emojis as Emoji[]) {
		if (emoji.group === 2) continue;
		groups[emoji.group ?? 8].push(emoji);
	}
	return groups.map((emojis, i) => ({
		id: i,
		name: getGroupName(i) || "Unknown",
		icon: getGroupIcon(i),
		emojis: emojis.map((e) => ({
			type: "standard",
			label: e.label,
			unicode: e.unicode,
			hexcode: e.hexcode,
			order: e.order,
		})),
	})).filter((g) => g.name !== "Unknown");
};

const getGroupIcon = (id: number) => {
	switch (id) {
		case 0:
			return icEmojiFaces;
		case 1:
			return icEmojiPeople;
		case 3:
			return icEmojiNature;
		case 4:
			return icEmojiFood;
		case 5:
			return icEmojiPlaces;
		case 6:
			return icEmojiActivities;
		case 7:
			return icEmojiObjects;
		case 8:
			return icEmojiSymbols;
		case 9:
			return icEmojiFlags;
	}
};

const getGroupName = (id: number) => {
	switch (id) {
		case 0:
			return "Faces";
		case 1:
			return "People";
		case 2:
			// this category is skin tones and hair modifiers, so we ignore it
			return;
		case 3:
			return "Animals & nature";
		case 4:
			return "Food & Drink";
		case 5:
			return "Travel & Places";
		case 6:
			return "Activities";
		case 7:
			return "Objects";
		case 8:
			return "Symbols";
		case 9:
			return "Flags";
	}
};

const getShortcode = async (hex: string) => {
	const [{ default: shortJoypixels }, { default: shortEmojibase }] =
		await Promise.all([
			import("emojibase-data/en/shortcodes/joypixels.json"),
			import("emojibase-data/en/shortcodes/emojibase.json"),
		]);
	const codes = (shortJoypixels as Record<string, string | string[]>)[hex] ??
		(shortEmojibase as Record<string, string | string[]>)[hex];
	return Array.isArray(codes) ? codes[0] : codes;
};

const getTwemoji = (unicode: string) => {
	return twemoji.parse(unicode, {
		base: "https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/",
		attributes: () => ({ loading: "lazy" }),
		folder: "svg",
		ext: ".svg",
	});
};

type EmojiPickerProps = {
	selected: (value: string | null, shiftKey: boolean) => void;
};

export const EmojiPicker = (props: EmojiPickerProps) => {
	const api = useApi();
	const [search, setSearch] = createSignal("");
	const [hover, setHover] = createSignal<UnifiedEmoji>();

	const rooms = api.rooms.list();
	const [groupsResource] = createResource(async () => {
		const standard = await parseEmoji();
		return standard;
	});

	const [customGroupsResource] = createResource(
		() => rooms()?.items,
		async (roomItems) => {
			await api.emoji.listAllCustom(roomItems.map((r) => r.id));
			return roomItems.map((room) => {
				const emojis = [...api.emoji.cache.values()].filter((e) => {
					if (e.owner?.owner === "Room") {
						return e.owner.room_id === room.id;
					}
					return false;
				});

				if (emojis.length === 0) return null;

				return {
					id: `room-${room.id}`,
					name: room.name,
					room,
					emojis: emojis.map((e) => ({
						type: "custom",
						label: e.name,
						id: e.id,
						media_id: e.media_id,
						animated: e.animated,
						room_id: room.id,
					})),
				} as EmojiGroup;
			}).filter((r) => r !== null) as EmojiGroup[];
		},
	);

	const allGroups = createMemo(() => {
		const standard = groupsResource() || [];
		const custom = customGroupsResource() || [];
		return [...custom, ...standard];
	});

	const [shortcode] = createResource(hover, async (h) => {
		if (!h) return "";
		if (h.type === "custom") return h.label;
		return getShortcode(h.hexcode!);
	});

	const filtered = () => {
		const groups = allGroups();
		const s = search();
		if (!s) return groups;

		return groups.map((group) => {
			const results = fuzzysort.go(s, group.emojis, {
				key: "label",
				threshold: -1000,
			});
			return {
				...group,
				emojis: results
					.map((j) => j.obj)
					.sort((a, b) => {
						if (a.type === "standard" && b.type === "standard") {
							return (
								parseInt(a.hexcode!, 16) - parseInt(b.hexcode!, 16) ||
								(a.order! > b.order! ? 1 : -1)
							);
						}
						return a.label.localeCompare(b.label);
					}),
			};
		}).filter((g) => g.emojis.length > 0);
	};

	const handleSubmit = async (value: string, e: KeyboardEvent) => {
		if (e.ctrlKey) {
			props.selected(value, e.shiftKey);
		} else {
			const f = filtered();
			if (!f) return;
			const group = f.find((i) => i.emojis.length);
			const emoji = group?.emojis[0];
			if (emoji) {
				if (emoji.type === "standard") {
					props.selected(emoji.unicode!, e.shiftKey);
				} else {
					props.selected(`<:${emoji.label}:${emoji.id}>`, e.shiftKey);
				}
			}
		}
	};

	let emojisContainerRef!: HTMLDivElement;

	const scrollToCategory = (id: string | number) => {
		const el = document.getElementById(`emoji-cat-${id}`);
		if (el) {
			el.scrollIntoView({ behavior: "smooth", block: "start" });
		}
	};

	return (
		<div class="emoji-picker" onClick={(e) => e.stopPropagation()}>
			<header>
				<Search
					placeholder="shift for multiple, ctrl for raw text"
					size="input"
					value={search}
					onValue={setSearch}
					submitted={handleSubmit}
					escaped={() => props.selected(null, false)}
				/>
				{/* TODO: (low priority) skin tone */}
				<div
					style="font-size: 24px; height: 28px; width: 28px; margin-left: 8px; cursor: pointer"
					hidden
					innerHTML={getTwemoji("")}
				>
				</div>
			</header>
			<div class="categories">
				<For each={allGroups()}>
					{(cat) => (
						<button onClick={() => scrollToCategory(cat.id)}>
							<Show
								when={cat.room}
								fallback={<img class="icon" src={cat.icon} />}
							>
								<RoomIcon room={cat.room!} />
							</Show>
						</button>
					)}
				</For>
			</div>
			<div class="emojis" ref={emojisContainerRef!}>
				<For each={filtered()}>
					{(group) => (
						<div id={`emoji-cat-${group.id}`}>
							<div class="label">{group.name}</div>
							<div class="group">
								<For each={group.emojis}>
									{(emoji) => (
										<div
											class="emojiwrap"
											onMouseOver={() => setHover(emoji)}
											onFocus={() => setHover(emoji)}
											onClick={(e) => {
												if (emoji.type === "standard") {
													props.selected(emoji.unicode!, e.shiftKey);
												} else {
													props.selected(
														`<:${emoji.label}:${emoji.id}>`,
														e.shiftKey,
													);
												}
											}}
											innerHTML={emoji.type === "standard"
												? getTwemoji(emoji.unicode!)
												: `<img src="${
													getThumbFromId(emoji.media_id!, 64)
												}" class="custom-emoji" />`}
										>
										</div>
									)}
								</For>
							</div>
						</div>
					)}
				</For>
				<Show
					when={!groupsResource.loading &&
						filtered().every((i) => i.emojis.length === 0)}
				>
					<div style="display: flex; align-items: center; justify-content: center; width 100%; height: 100%">
						no emoji :(
					</div>
				</Show>
			</div>
			<div class="preview">
				<Show when={hover()}>
					{(h) => (
						<>
							<div
								innerHTML={h().type === "standard"
									? getTwemoji(h().unicode!)
									: `<img src="${
										getThumbFromId(h().media_id!, 64)
									}" class="custom-emoji" />`}
							>
							</div>
							<b>:{shortcode()}:</b>
							{/* <span style="color: var(--fg-dim)">{h().tags?.join(", ")}</span> */}
						</>
					)}
				</Show>
			</div>
		</div>
	);
};
