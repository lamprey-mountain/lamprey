import { Command, TextSelection } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { Schema } from "prosemirror-model";
import type { ChatCtx } from "./context";

/** create a command that wraps or unwraps selected text with some characters */
export function createWrapCommand(wrap: string): Command {
	const len = wrap.length;

	return (state, dispatch) => {
		const { from, to } = state.selection;
		const tr = state.tr;

		const isWrapped = (
			tr.doc.textBetween(from - len, from) === wrap &&
			tr.doc.textBetween(to, to + len) === wrap
		) || (
			false
			// FIXME: fails?
			// tr.doc.textBetween(from, from + len) === wrap &&
			// tr.doc.textBetween(to - len, to) === wrap
		);

		if (isWrapped) {
			tr.delete(to, to + len);
			tr.delete(from - len, from);
		} else {
			tr.insertText(wrap, to);
			tr.insertText(wrap, from);
			tr.setSelection(TextSelection.create(tr.doc, from + len, to + len));
		}

		dispatch?.(tr);
		return true;
	};
}

export function handleAutocomplete(
	view: EditorView,
	event: KeyboardEvent,
	ctx: ChatCtx,
	schema: Schema,
	channelId: string,
): boolean {
	const LINE_HEIGHT = 18;
	const refElement = () => {
		const cursorPos = view.coordsAtPos(view.state.selection.from);
		return {
			getBoundingClientRect() {
				return {
					x: cursorPos.left,
					y: cursorPos.bottom - LINE_HEIGHT,
					left: cursorPos.left,
					right: cursorPos.right,
					top: cursorPos.bottom - LINE_HEIGHT,
					bottom: cursorPos.bottom,
					width: 0,
					height: LINE_HEIGHT,
				};
			},
		};
	};

	if (event.key === "/") {
		const state = view.state;
		if (state.selection.from === 1) {
			ctx.setAutocomplete({
				type: "command",
				query: "",
				ref: refElement() as any,
				onSelect: (command: string) => {
					const state = view.state;
					const from = 0;
					const to = state.selection.to;

					let tr = state.tr.replaceWith(
						from,
						to,
						schema.text(`/${command} `),
					);
					const posAfter = tr.mapping.map(to);
					tr = tr.setSelection(
						TextSelection.create(tr.doc, posAfter + 1),
					);

					view.dispatch(tr);
					ctx.setAutocomplete(null);
				},
				channelId: channelId || "",
			});
		}

		return false;
	}

	if (event.key === "@") {
		ctx.setAutocomplete({
			type: "mention",
			query: "",
			ref: refElement() as any,
			onSelect: (userId: string, _userName: string) => {
				const state = view.state;
				const from = Math.max(0, state.selection.from - 1);
				const to = state.selection.to;

				let mentionStart = from;
				while (mentionStart > 0) {
					const char = state.doc.textBetween(
						mentionStart - 1,
						mentionStart,
					);
					if (char === "@" || /\w/.test(char)) {
						mentionStart--;
					} else {
						break;
					}
				}

				let tr = state.tr.replaceWith(
					mentionStart,
					to,
					schema.nodes.mention.create({ user: userId }),
				);
				const posAfter = tr.mapping.map(to);
				tr = tr.insert(posAfter, schema.text(" ", []));
				tr = tr.setSelection(
					TextSelection.create(tr.doc, posAfter + 1),
				);

				view.dispatch(tr);

				ctx.setAutocomplete(null);
			},
			channelId: channelId || "",
		});

		return false;
	}

	if (event.key === "#") {
		ctx.setAutocomplete({
			type: "channel",
			query: "",
			ref: refElement() as any,
			onSelect: (channelId: string, _channelName: string) => {
				const state = view.state;
				const from = Math.max(0, state.selection.from - 1);
				const to = state.selection.to;

				let mentionStart = from;
				while (mentionStart > 0) {
					const char = state.doc.textBetween(
						mentionStart - 1,
						mentionStart,
					);
					if (char === "#" || /\w/.test(char)) {
						mentionStart--;
					} else {
						break;
					}
				}

				let tr = state.tr.replaceWith(
					mentionStart,
					to,
					schema.nodes.mentionChannel.create({ channel: channelId }),
				);
				const posAfter = tr.mapping.map(to);
				tr = tr.insert(posAfter, schema.text(" ", []));
				tr = tr.setSelection(
					TextSelection.create(tr.doc, posAfter + 1),
				);

				view.dispatch(tr);

				ctx.setAutocomplete(null);
			},
			channelId: channelId || "",
		});

		return false;
	}

	if (event.key === ":") {
		ctx.setAutocomplete({
			type: "emoji",
			query: "",
			ref: refElement() as any,
			onSelect: (id: string, name: string, char?: string) => {
				const state = view.state;
				const from = Math.max(0, state.selection.from - 1);
				const to = state.selection.to;

				let mentionStart = from;
				while (mentionStart > 0) {
					const char = state.doc.textBetween(
						mentionStart - 1,
						mentionStart,
					);
					if (char === ":" || /[\w_]/.test(char)) {
						mentionStart--;
					} else {
						break;
					}
				}

				let tr = state.tr;
				if (char) { // unicode
					tr = tr.replaceWith(mentionStart, to, schema.text(char));
				} else { // custom
					tr = tr.replaceWith(
						mentionStart,
						to,
						schema.nodes.emoji.create({ id, name }),
					);
				}

				const posAfter = tr.mapping.map(to);
				tr = tr.insert(posAfter, schema.text(" ", []));
				tr = tr.setSelection(
					TextSelection.create(tr.doc, posAfter + 1),
				);

				view.dispatch(tr);
				ctx.setAutocomplete(null);
			},
			channelId: channelId || "",
		});
		return false;
	}

	// autocomplete navigation and selection
	if (ctx?.autocomplete()) {
		if (
			event.key === "ArrowUp" || event.key === "ArrowDown" ||
			event.key === "Enter" || event.key === "Tab" ||
			event.key === "Escape"
		) {
			// handled by the autocomplete component
			return false;
		}
	}

	if (ctx?.autocomplete()) {
		if (event.key === " " || event.key === "Enter") {
			ctx.setAutocomplete(null);
		} else {
			const state = view.state;
			const cursorPos = state.selection.from;

			const triggerChar = ctx.autocomplete()!.type === "mention"
				? "@"
				: ctx.autocomplete()!.type === "channel"
				? "#"
				: ctx.autocomplete()!.type === "command"
				? "/"
				: ":";
			let mentionStart = -1;

			// search backward for trigger symbol
			for (let i = cursorPos - 1; i >= 0; i--) {
				const char = state.doc.textBetween(i, i + 1);
				if (char === triggerChar) {
					mentionStart = i;
					break;
				}

				// invalid characters for a mention query
				if (char === " " || char === "\n" || char === "\t") {
					ctx.setAutocomplete(null);
					return false;
				}
			}

			if (!ctx.autocomplete()) {
				return false;
			}

			if (mentionStart === -1) {
				ctx.setAutocomplete(null);
				return false;
			}

			const currentQuery = state.doc.textBetween(
				mentionStart + 1,
				cursorPos,
			);

			let newQuery;
			if (event.key === "Backspace") {
				if (cursorPos <= mentionStart + 1) {
					ctx.setAutocomplete(null);
					return false;
				}
				newQuery = currentQuery.slice(0, -1);
			} else if (
				event.key.length === 1 && !event.ctrlKey && !event.metaKey &&
				!event.altKey
			) {
				newQuery = currentQuery + event.key;
			} else {
				return false;
			}

			ctx.setAutocomplete({
				...ctx.autocomplete()!,
				query: newQuery,
				ref: refElement() as any,
			});
		}
	}

	return false;
}

/** decode unpadded url safe base64 */
export function base64UrlDecode(str: string): Uint8Array {
	str = str.replace(/-/g, "+").replace(/_/g, "/");

	const pad = str.length % 4;
	if (pad) {
		str += "=".repeat(4 - pad);
	}

	const binary = atob(str);
	const bytes = new Uint8Array(binary.length);

	for (let i = 0; i < binary.length; i++) {
		bytes[i] = binary.charCodeAt(i);
	}

	return bytes;
}

export function base64UrlEncode(bytes: Uint8Array): string {
	let binary = "";
	const len = bytes.byteLength;
	for (let i = 0; i < len; i++) {
		binary += String.fromCharCode(bytes[i]);
	}
	return btoa(binary)
		.replace(/\+/g, "-")
		.replace(/\//g, "_")
		.replace(/=+$/, "");
}

/**
 * Detects if the current line is part of a list (ordered, unordered, blockquote, or todo)
 * and returns the list type and prefix if applicable
 */
export function getListPrefix(
	line: string,
): {
	type: "ordered" | "unordered" | "blockquote" | "todo";
	prefix: string;
	number?: number;
	checked?: boolean;
} | null {
	// Check for ordered list: digits followed by a dot and space
	const orderedMatch = line.match(/^(\s*)(\d+)\.(\s+)/);
	if (orderedMatch) {
		const prefix = orderedMatch[0];
		const number = parseInt(orderedMatch[2], 10);
		return { type: "ordered", prefix, number };
	}

	// Check for todo list: dash/asterisk/plus followed by space, [ ] or [x], and space
	const todoMatch = line.match(/^(\s*)([-*+])(\s+)\[([ x])\](\s+)/);
	if (todoMatch) {
		const prefix = todoMatch[0];
		const checked = todoMatch[4] === "x";
		return { type: "todo", prefix, checked };
	}

	// Check for unordered list: dash, asterisk, or plus followed by space
	const unorderedMatch = line.match(/^(\s*)([-*+])(\s+)/);
	if (unorderedMatch) {
		return { type: "unordered", prefix: unorderedMatch[0] };
	}

	// Check for blockquote: greater than symbol followed by space
	const blockquoteMatch = line.match(/^(\s*)>(\s+)/);
	if (blockquoteMatch) {
		return { type: "blockquote", prefix: blockquoteMatch[0] };
	}

	return null;
}

/**
 * Creates a command to handle Enter key in lists
 */
export function createListContinueCommand(): Command {
	return (state, dispatch) => {
		const { from, to } = state.selection;
		const $from = state.selection.$from;

		// We need to find the start and end of the current "line" within the block
		// ProseMirror blocks (like paragraphs) can contain newlines if whitespace: "pre"
		const parent = $from.parent;
		if (!parent.isTextblock) {
			return false;
		}

		const parentStart = $from.start();
		// Use a custom leafText function to ensure atomic nodes (like mentions) are treated as length 1
		// This matches the document structure indices
		const text = state.doc.textBetween(
			parentStart,
			$from.end(),
			"\n",
			"\ufffc",
		);

		const offsetInParent = from - parentStart;
		const lastNewline = text.lastIndexOf("\n", offsetInParent - 1);
		const nextNewline = text.indexOf("\n", offsetInParent);

		const lineStart = parentStart + (lastNewline === -1 ? 0 : lastNewline + 1);
		const lineEnd = parentStart +
			(nextNewline === -1 ? text.length : nextNewline);

		const currentLine = state.doc.textBetween(
			lineStart,
			lineEnd,
			undefined,
			"\ufffc",
		);
		const listPrefix = getListPrefix(currentLine);

		if (!listPrefix) {
			// Not in a list, just insert newline
			dispatch?.(state.tr.insertText("\n"));
			return true;
		}

		// Get the position of the cursor within the line
		const cursorInLine = from - lineStart;

		// Check if the line has content after the cursor
		const lineAfterCursor = currentLine.substring(cursorInLine);

		// Check if the line has content before the cursor (after removing the prefix)
		const lineBeforeCursor = currentLine.substring(0, cursorInLine);
		const contentAfterPrefix = lineBeforeCursor.substring(
			listPrefix.prefix.length,
		);

		// If both before and after cursor are empty (just the prefix), remove the list item
		const isLineEmpty = contentAfterPrefix.trim() === "" &&
			lineAfterCursor.trim() === "";

		if (isLineEmpty) {
			// Remove the entire line (list prefix and all)
			// This effectively "exits" the list for the current line
			const tr = state.tr.delete(lineStart, lineEnd);

			// If we are at the very start of the block/doc, we might want to ensure a newline exists if we are breaking out?
			// But deleting the only line leaves an empty block/doc which is valid.
			// If we are after a newline, deleting the content leaves us after that newline (empty line).
			// This is the desired behavior for "breaking out of a list":
			// 1. Item
			// 2. [Cursor] -> Enter ->
			// 1. Item
			// [Cursor]

			dispatch?.(tr);
			return true;
		} else {
			// Continue the list with the next item
			let newPrefix = listPrefix.prefix;

			if (listPrefix.type === "ordered") {
				// Increment the number for ordered lists
				const nextNumber = (listPrefix.number || 0) + 1;
				newPrefix = newPrefix.replace(/(\d+)/, String(nextNumber));
			} else if (listPrefix.type === "todo") {
				// Keep the same todo format (unchecked by default for new items)
				// The prefix already includes the checkbox, e.g., "- [ ] "
			}

			// Insert newline and the new prefix
			const tr = state.tr.insertText(`\n${newPrefix}`, to);

			// Move cursor to after the new prefix
			const newPos = to + 1 + newPrefix.length;
			tr.setSelection(TextSelection.create(tr.doc, newPos));

			dispatch?.(tr);
			return true;
		}
	};
}
