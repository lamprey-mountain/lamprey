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
