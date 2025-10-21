import { createResource, createSignal, For, Show } from "solid-js";
import twemoji from "twemoji";
import fuzzysort from "fuzzysort";
import shortJoypixels from "emojibase-data/en/shortcodes/joypixels.json";
import shortEmojibase from "emojibase-data/en/shortcodes/emojibase.json";
import emojis from "emojibase-data/en/compact.json";
import { Search } from "./atoms/Search";

type Emoji = {
	group?: number;
	label: string;
	hexcode: string;
	order: number;
	unicode: string;
	tags?: string[];
	shortcode?: string | string[];
};

const parseEmoji = async () => {
	const groups: Emoji[][] = [[], [], [], [], [], [], [], [], [], []];
	for (let emoji of emojis as Emoji[]) {
		if (emoji.group === 2) continue;
		groups[emoji.group ?? 8].push(emoji);
	}
	return groups;
};

const getGroupName = (id: number) => {
	switch (id) {
		case 0:
			return "Faces";
		case 1:
			return "People";
		case 2:
			return "???";
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

const getShortcode = (hex: string) => {
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
	const [search, setSearch] = createSignal("");
	const [hover, setHover] = createSignal<Emoji>();
	const [groupsResource] = createResource(parseEmoji);

	const filtered = () => {
		const groups = groupsResource();
		if (!groups) return [];
		const s = search();
		if (!s) return groups;

		return groups.map((i) =>
			fuzzysort
				// .go(search, i, { keys: ["label", "shortcode"], threshold: -1000 })
				.go(s, i, { key: "label", threshold: -1000 })
				.map((j) => j.obj)
				.sort(
					(a, b) =>
						parseInt(a.hexcode, 16) - parseInt(b.hexcode, 16) ||
						(a.order > b.order ? 1 : -1),
				)
		);
	};

	const handleSubmit = async (value: string, e: KeyboardEvent) => {
		if (e.ctrlKey) {
			props.selected(value, e.shiftKey);
		} else {
			const f = filtered();
			if (!f) return;
			const group = f.find((i) => i.length);
			const emoji = group?.[0]?.unicode;
			if (emoji) props.selected(emoji, e.shiftKey);
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
				{
					/* TODO: icons for categories
				<div>history</div>
				<div>emoji_emotions</div>
				<div>emoji_people</div>
				<div>park</div>
				<div>emoji_food_beverage</div>
				<div>snowmobile</div>
				<div>emoji_events</div>
				<div>emoji_objects</div>
				<div>emoji_symbols</div>
				<div>flag</div>
					*/
				}
				<div>0</div>
				<div>1</div>
				<div>2</div>
				<div>3</div>
				<div>4</div>
				<div>5</div>
				<div>6</div>
				<div>7</div>
				<div>8</div>
				<div>9</div>
			</div>
			<div class="emojis">
				<Show
					when={!groupsResource.loading}
					fallback={
						<div style="display: flex; align-items: center; justify-content: center; width 100%; height: 100%">
							getting emoji...
						</div>
					}
				>
					<For each={filtered()}>
						{(emojis, i) => (
							<Show when={emojis && emojis.length > 0}>
								<div class="label">{getGroupName(i())}</div>
								<div class="group">
									<For each={emojis}>
										{(emoji) => (
											<div
												class="emojiwrap"
												onMouseOver={() => setHover(emoji)}
												onFocus={() => setHover(emoji)}
												onClick={(e) =>
													props.selected(emoji.unicode, e.shiftKey)}
												innerHTML={getTwemoji(emoji.unicode)}
											>
											</div>
										)}
									</For>
								</div>
							</Show>
						)}
					</For>
					<Show when={filtered()?.every((i) => i.length === 0)}>
						<div style="display: flex; align-items: center; justify-content: center; width 100%; height: 100%">
							no emoji :(
						</div>
					</Show>
				</Show>
			</div>
			<div class="preview">
				<Show when={hover()}>
					{(h) => (
						<>
							<div innerHTML={getTwemoji(h().unicode)}></div>
							<b>:{getShortcode(h().hexcode)}:</b>
							{/* <span style="color: var(--fg-dim)">{h().tags?.join(", ")}</span> */}
						</>
					)}
				</Show>
			</div>
		</div>
	);
};
