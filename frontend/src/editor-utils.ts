import { Command, TextSelection } from "prosemirror-state";

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

// TODO: refactor out nodeViews here
// TODO: refactor out handlePaste here
// TODO: refactor out autocomplete here
