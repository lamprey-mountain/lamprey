import { createResource, createSignal, For, Show } from "solid-js";
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
	const { default: emojis } = await import("emojibase-data/en/compact.json");
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
	const [search, setSearch] = createSignal("");
	const [hover, setHover] = createSignal<Emoji>();
	const [groupsResource] = createResource(parseEmoji);
	const [shortcode] = createResource(hover, async (h) => {
		if (!h) return "";
		return getShortcode(h.hexcode);
	});

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
				<For
					each={[
						{ id: 0, icon: icEmojiFaces },
						{ id: 1, icon: icEmojiPeople },
						{ id: 3, icon: icEmojiNature },
						{ id: 4, icon: icEmojiFood },
						{ id: 5, icon: icEmojiPlaces },
						{ id: 6, icon: icEmojiActivities },
						{ id: 7, icon: icEmojiObjects },
						{ id: 8, icon: icEmojiSymbols },
						{ id: 9, icon: icEmojiFlags },
					]}
				>
					{(cat) => (
						<button onClick={() => alert("todo")}>
							<img class="icon" src={cat.icon} />
						</button>
					)}
				</For>
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
							<b>:{shortcode()}:</b>
							{/* <span style="color: var(--fg-dim)">{h().tags?.join(", ")}</span> */}
						</>
					)}
				</Show>
			</div>
		</div>
	);
};
