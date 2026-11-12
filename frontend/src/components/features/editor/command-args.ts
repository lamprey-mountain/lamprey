import type { Node as PMNode } from "prosemirror-model";
import type { EditorState } from "prosemirror-state";
import type { SlashCommands } from "@/lib/commands/registry";
import { tokenizeArgs, unescapeArg } from "@/lib/commands/tokenize";
import type { Command, OptionDef } from "@/lib/commands/types";

export type TextSegment = {
	textStart: number;
	textEnd: number;
	docStart: number;
	docEnd: number;
	mention: boolean;
};

export type EditorText = {
	text: string;
	segments: TextSegment[];
	/** doc position at the end of the first block's content */
	textEnd: number;
};

export type ArgSlot = {
	option: OptionDef;
	index: number;
	/** doc position where this argument's value starts (including quotes) */
	start: number;
	/** doc position where it ends (exclusive, including quotes) */
	end: number;
	/** doc positions of the value itself, excluding surrounding quotes */
	valueStart: number;
	valueEnd: number;
	/** offsets of the same ranges in EditorText.text */
	textStart: number;
	textEnd: number;
	textValueStart: number;
	textValueEnd: number;
	value: string;
	/** whether a token exists for this argument, even if its value is empty */
	filled: boolean;
	/** the token is a mention atom node */
	mention: boolean;
	quoted: boolean;
	/** implicitly spans whitespace without quotes */
	multiWord: boolean;
	/** text between the start of the value and the cursor */
	query: string;
};

export type CommandArgs = {
	command: Command;
	/** the typed command path, e.g. `/room settings` */
	name: string;
	/** doc position right after the command name */
	nameEnd: number;
	/** doc position at the end of the text */
	textEnd: number;
	text: string;
	segments: TextSegment[];
	slots: ArgSlot[];
};

export function getEditorText(state: EditorState): EditorText | undefined {
	const first = state.doc.firstChild;
	if (!first?.isTextblock) return undefined;

	const segments: TextSegment[] = [];
	let text = "";

	first.forEach((node, offset) => {
		const docStart = 1 + offset;
		const value = serializeAtom(node);
		if (!value) return;

		segments.push({
			textStart: text.length,
			textEnd: text.length + value.length,
			docStart,
			docEnd: docStart + node.nodeSize,
			mention: node.type.name === "mention",
		});
		text += value;
	});

	return { text, segments, textEnd: 1 + first.content.size };
}

export function getCommandArgs(
	state: EditorState,
	commands: SlashCommands | undefined,
): CommandArgs | undefined {
	if (!commands) return undefined;

	const editorText = getEditorText(state);
	if (!editorText) return undefined;

	const match = commands.match(editorText.text);
	if (!match) return undefined;

	return buildCommandArgs(editorText, match.command, match.consumed);
}

/** split the text after a command into one slot per option, mirroring parseOptions */
export function buildCommandArgs(
	editorText: EditorText,
	command: Command,
	consumed: number,
): CommandArgs {
	const { text, segments, textEnd } = editorText;
	const textLength = text.length;
	const tokens = tokenizeArgs(text.slice(consumed));

	let idx = 0;
	const slots = command.options.map((option, index): ArgSlot => {
		const isLast = index === command.options.length - 1;

		if (option.kind === "string" && isLast) {
			const first = tokens[idx];
			const last = tokens[tokens.length - 1];
			const selected = tokens.slice(idx);
			idx = tokens.length;

			if (first && last) {
				return makeSlot({
					option,
					index,
					editorText,
					textStart: consumed + first.start,
					textEnd: consumed + last.end,
					textValueStart: consumed + first.valueStart,
					textValueEnd: consumed + last.valueEnd,
					value: selected.map((token) => token.value).join(" "),
					quoted: first.quoted,
					multiWord: !first.quoted && last.start > first.end,
					mention: isMentionRange(
						segments,
						consumed + first.start,
						consumed + last.end,
					),
				});
			}
			return emptySlot(option, index, textEnd, textLength);
		}

		const token = tokens[idx++];
		if (token) {
			const textStart = consumed + token.start;
			const textEndToken = consumed + token.end;
			return makeSlot({
				option,
				index,
				editorText,
				textStart,
				textEnd: textEndToken,
				textValueStart: consumed + token.valueStart,
				textValueEnd: consumed + token.valueEnd,
				value: token.value,
				quoted: token.quoted,
				multiWord: false,
				mention: isMentionRange(segments, textStart, textEndToken),
			});
		}
		return emptySlot(option, index, textEnd, textLength);
	});

	return {
		command,
		name: text.slice(0, consumed),
		nameEnd: toDocPos(segments, consumed),
		textEnd,
		text,
		segments,
		slots,
	};
}

/** find the argument the cursor is currently editing */
export function getActiveSlot(
	args: CommandArgs,
	cursor: number,
): ArgSlot | undefined {
	if (cursor <= args.nameEnd || cursor > args.textEnd) return undefined;

	const textCursor = toTextPos(args.segments, cursor);

	// cursor inside a value, or right at its end
	const filled = args.slots.find(
		(s) => s.filled && textCursor >= s.textStart && textCursor <= s.textEnd,
	);

	// otherwise: the next slot starting at or after the cursor
	let slot = filled ?? args.slots.find((s) => s.textStart >= textCursor);

	// cursor is in trailing whitespace after an already-filled argument
	slot ??= [...args.slots]
		.reverse()
		.find((s) => s.filled && s.textStart < textCursor);

	if (!slot) return undefined;

	const query =
		textCursor > slot.textValueStart
			? unescapeArg(
					args.text.slice(
						slot.textValueStart,
						Math.min(textCursor, slot.textValueEnd),
					),
				)
			: "";

	return { ...slot, query };
}

type SlotInit = {
	option: OptionDef;
	index: number;
	editorText: EditorText;
	textStart: number;
	textEnd: number;
	textValueStart: number;
	textValueEnd: number;
	value: string;
	quoted: boolean;
	multiWord: boolean;
	mention: boolean;
};

function makeSlot(init: SlotInit): ArgSlot {
	const { segments } = init.editorText;

	return {
		option: init.option,
		index: init.index,
		start: toDocPos(segments, init.textStart),
		end: toDocPos(segments, init.textEnd),
		valueStart: toDocPos(segments, init.textValueStart),
		valueEnd: toDocPos(segments, init.textValueEnd),
		textStart: init.textStart,
		textEnd: init.textEnd,
		textValueStart: init.textValueStart,
		textValueEnd: init.textValueEnd,
		value: init.value,
		filled: true,
		mention: init.mention,
		quoted: init.quoted,
		multiWord: init.multiWord,
		query: "",
	};
}

function emptySlot(
	option: OptionDef,
	index: number,
	textEnd: number,
	textLength: number,
): ArgSlot {
	return {
		option,
		index,
		start: textEnd,
		end: textEnd,
		valueStart: textEnd,
		valueEnd: textEnd,
		textStart: textLength,
		textEnd: textLength,
		textValueStart: textLength,
		textValueEnd: textLength,
		value: "",
		filled: false,
		mention: false,
		quoted: false,
		multiWord: false,
		query: "",
	};
}

function isMentionRange(
	segments: TextSegment[],
	textStart: number,
	textEnd: number,
): boolean {
	return segments.some(
		(segment) =>
			segment.mention &&
			textStart >= segment.textStart &&
			textEnd <= segment.textEnd,
	);
}

export function toDocPos(segments: TextSegment[], offset: number): number {
	for (const segment of segments) {
		if (offset <= segment.textStart) return segment.docStart;
		if (offset <= segment.textEnd) {
			if (offset === segment.textEnd) return segment.docEnd;
			return segment.docStart + (offset - segment.textStart);
		}
	}

	return segments[segments.length - 1]?.docEnd ?? 1;
}

export function toTextPos(segments: TextSegment[], docPos: number): number {
	for (const segment of segments) {
		if (docPos <= segment.docStart) return segment.textStart;
		if (docPos <= segment.docEnd) {
			if (docPos === segment.docEnd) return segment.textEnd;
			return segment.textStart + (docPos - segment.docStart);
		}
	}

	return segments[segments.length - 1]?.textEnd ?? 0;
}

function serializeAtom(node: PMNode): string {
	if (node.isText) return node.text ?? "";

	switch (node.type.name) {
		case "mention":
			return `<@${node.attrs.user}>`;
		case "mentionChannel":
			return `<#${node.attrs.channel}>`;
		case "mentionRole":
			return `<&${node.attrs.role}>`;
		case "mentionEveryone":
			return "@everyone";
		case "emojiCustom":
			return `<:${node.attrs.name}:${node.attrs.id}>`;
		case "emojiUnicode":
			return node.attrs.char;
		default:
			return node.textContent;
	}
}
