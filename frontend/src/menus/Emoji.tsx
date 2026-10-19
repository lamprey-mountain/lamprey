import { Show } from "solid-js";
import { useCtx } from "@/app/context";
import { getEmojiUrl } from "@/media/util";
import { Item, Menu } from "./Parts.tsx";

export function EmojiMenu(props: {
	/** reaction key param: `c:{id}` for custom, `t:{unicode}` for unicode */
	key: string;
	id?: string;
	name?: string;
	content?: string;
	animated?: boolean;
}) {
	const ctx = useCtx();

	const favorites = () => ctx.preferences().frontend.favorite_emoji ?? [];
	const isFavorite = () => favorites().includes(props.key);

	const toggleFavorite = () => {
		const favorite_emoji = isFavorite()
			? favorites().filter((key) => key !== props.key)
			: [...favorites(), props.key];

		const prefs = ctx.preferences();
		ctx.setPreferences({
			...prefs,
			frontend: {
				...prefs.frontend,
				favorite_emoji,
			},
		});
	};

	const copyImageUrl = () => {
		if (!props.id) return;
		navigator.clipboard.writeText(getEmojiUrl(props.id));
	};

	const copyId = () => {
		navigator.clipboard.writeText(props.id ?? props.content ?? props.key);
	};

	const logToConsole = () => {
		console.log({
			key: props.key,
			id: props.id,
			name: props.name,
			content: props.content,
			animated: props.animated,
		});
	};

	return (
		<Menu>
			<Item onClick={toggleFavorite}>
				{isFavorite() ? "remove from favorites" : "favorite"}
			</Item>
			<Show when={props.id}>
				<Item onClick={copyImageUrl}>copy image url</Item>
			</Show>
			<Item onClick={copyId}>{props.id ? "copy id" : "copy emoji"}</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
