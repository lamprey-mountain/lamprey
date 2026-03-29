import type { Schema } from "prosemirror-model";
import { Plugin, PluginKey } from "prosemirror-state";
import type { EditorView } from "prosemirror-view";

export const emojiPluginKey = new PluginKey("emoji");

const segmenter =
	typeof (Intl as any).Segmenter !== "undefined"
		? new (Intl as any).Segmenter("en", { granularity: "grapheme" })
		: null;

export const EMOJI_TEST = /\p{Emoji_Presentation}|\p{Extended_Pictographic}/u;

export function convertEmojiInText(schema: Schema, text: string) {
	const content: any[] = [];
	let textBuffer = "";
	let hasEmoji = false;

	for (const { segment } of segmenter.segment(text)) {
		if (EMOJI_TEST.test(segment)) {
			if (textBuffer) {
				content.push(schema.text(textBuffer));
				textBuffer = "";
			}
			content.push(schema.nodes.emojiUnicode.create({ char: segment }));
			hasEmoji = true;
		} else {
			textBuffer += segment;
		}
	}

	if (textBuffer) content.push(schema.text(textBuffer));
	return { content, hasEmoji };
}

export function createEmojiPlugin(): Plugin {
	return new Plugin({
		key: emojiPluginKey,
		props: {
			handleTextInput(
				view: EditorView,
				from: number,
				to: number,
				text: string,
			): boolean {
				if (!EMOJI_TEST.test(text)) return false;

				const { content } = convertEmojiInText(view.state.schema, text);
				const tr = view.state.tr.replaceWith(from, to, content);
				view.dispatch(tr.scrollIntoView());
				return true;
			},
		},
	});
}
