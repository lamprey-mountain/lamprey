import {
	Command,
	EditorState,
	Plugin,
	PluginKey,
	TextSelection,
	type Transaction,
} from "prosemirror-state";
import { DOMSerializer, ResolvedPos } from "prosemirror-model";
import { liftTarget } from "prosemirror-transform";
import { schema } from "./schema.ts";
import { serializeToMarkdown } from "./export-utils.ts";

interface LineBounds {
	start: number;
	end: number;
	text: string;
	isFirstLine: boolean;
	isLastLine: boolean;
}

function getLineBoundaries($pos: ResolvedPos): LineBounds {
	const parent = $pos.parent;
	const text = parent.textBetween(0, parent.content.size, null, "\n");
	const offset = $pos.parentOffset;
	const start = text.lastIndexOf("\n", offset - 1);
	const lineStart = start === -1 ? 0 : start + 1;
	const end = text.indexOf("\n", offset);
	const lineEnd = end === -1 ? text.length : end;

	return {
		start: $pos.start() + lineStart,
		end: $pos.start() + lineEnd,
		text: text.slice(lineStart, lineEnd),
		isFirstLine: lineStart === 0,
		isLastLine: lineEnd === text.length,
	};
}

function isolateLine(tr: Transaction, bounds: LineBounds): number {
	console.log("editor split", bounds);

	if (!bounds.isLastLine) {
		console.log("editor split isnt last");
		const end = tr.mapping.map(bounds.end);
		tr.delete(end, end + 1).split(end);
	}

	if (!bounds.isFirstLine) {
		console.log("editor split isnt first");
		const start = tr.mapping.map(bounds.start);
		tr.delete(start - 1, start).split(start - 1);
	}

	return tr.mapping.map(bounds.start);
}

// FIXME: always convert "> foo" into blockquotes
// FIXME: always convert "```" into codeblocks
// right now some things, like pasting, fails
// i probably need to refactor this from being like input rules into something else entirely?
export function createMarkdownInputRulesPlugin() {
	return new Plugin({
		key: new PluginKey("markdownHybrid"),
		props: {
			handleKeyDown(view, event) {
				const { state, dispatch } = view;
				const { $from, empty } = state.selection;

				// if text is selected, let the keymap handle the deletion
				if (!empty) return false;

				const bounds = getLineBoundaries($from);
				const isAtLineStart = $from.pos === bounds.start;

				if (event.key === "Backspace") {
					// removing a code block
					if ($from.parent.type === schema.nodes.code_block) {
						if (
							$from.parentOffset === 0 ||
							($from.parentOffset < 4 && bounds.text.startsWith("```"))
						) {
							dispatch(
								state.tr.setBlockType(
									$from.pos,
									$from.pos,
									schema.nodes.paragraph,
								),
							);
							return false;
						}
					}

					// deleting/exiting part of a blockquote
					if (
						isAtLineStart && $from.depth >= 2 &&
						$from.node($from.depth - 1).type === schema.nodes.blockquote
					) {
						let tr = state.tr;
						const isolatedStart = isolateLine(tr, bounds);

						// resolve positions
						const $pos = tr.doc.resolve(isolatedStart);
						const range = $pos.blockRange();
						const target = range ? liftTarget(range) : null;

						// FIXME: deleting blockquote with no line content
						// FIXME: merging blockquotes that are next to each other
						if (range && target !== null) {
							// this already splits content from its parent, so isolateLine may be redundant?
							tr.lift(range, target);
							const postLiftPos = tr.mapping.map(isolatedStart);
							tr.setBlockType(postLiftPos, postLiftPos, schema.nodes.paragraph);
							dispatch(tr.scrollIntoView());
							return true;
						}
					}
				}

				if (event.key === "Enter") {
					let tr = state.tr;

					// create code block
					const codeMatch = bounds.text.match(/^```+(\w*)$/);
					if (codeMatch && $from.parent.type === schema.nodes.paragraph) {
						const lang = codeMatch[1] || "";
						const isolatedStart = isolateLine(tr, bounds);
						const isolatedEnd = isolatedStart + bounds.text.length;
						const $pos = tr.doc.resolve(isolatedStart);
						tr.setBlockType(
							$pos.before(),
							$pos.after(),
							schema.nodes.code_block,
							{
								language: lang,
							},
						);
						tr.insertText("\n", isolatedEnd);
						tr.setSelection(TextSelection.create(tr.doc, isolatedEnd + 1));
						dispatch(tr.scrollIntoView());
						return true;
					}

					// closing a code block
					if (
						$from.parent.type === schema.nodes.code_block &&
						// FIXME: code blocks must close with the same fence as it opened with
						bounds.text.trim().startsWith("```")
					) {
						const insertPos = tr.mapping.map($from.after($from.depth));
						tr.insert(insertPos, schema.nodes.paragraph.create());
						tr.setSelection(TextSelection.create(tr.doc, insertPos + 1));
						dispatch(tr.scrollIntoView());
						return true;
					}
				}

				// handling tab inside of codeblocks
				if (
					event.key === "Tab" && $from.parent.type === schema.nodes.code_block
				) {
					event.preventDefault();
					dispatch(state.tr.insertText("\t"));
					return true;
				}

				return false;
			},

			handleTextInput(view, from, _to, text) {
				if (text !== " ") return false;

				const { state, dispatch } = view;
				const $from = state.doc.resolve(from);
				const bounds = getLineBoundaries($from);

				// convert > into blockquote elements
				if (
					bounds.text === ">" && $from.parent.type === schema.nodes.paragraph
				) {
					let tr = state.tr;
					if (!bounds.isFirstLine) {
						tr.delete(bounds.start - 1, bounds.start).split(bounds.start - 1);
					}
					const start = tr.mapping.map(bounds.start);
					tr.delete(start, tr.mapping.map(from));
					const range = tr.doc.resolve(start).blockRange();
					if (range) {
						dispatch(
							tr.wrap(range, [{ type: schema.nodes.blockquote }])
								.scrollIntoView(),
						);
						return true;
					}
				}
				return false;
			},

			clipboardTextSerializer: (slice) => serializeToMarkdown(slice.content),
			clipboardSerializer: DOMSerializer.fromSchema(schema),
		},
	});
}

export const joinBlockquoteBackward: Command = (state, dispatch) => {
	const { $from, empty } = state.selection;

	// Only trigger if selection is empty and at the start of a paragraph
	if (
		!empty || $from.parentOffset > 0 || $from.parent.type.name !== "paragraph"
	) {
		return false;
	}

	const tr = state.tr;
	const $before = state.doc.resolve($from.before());
	const nodeBefore = $before.nodeBefore;

	// SCENARIO 1: [blockquote(p1), |p2]
	// We are at the start of p2, which is a sibling of the blockquote.
	if (nodeBefore?.type.name === "blockquote") {
		if (dispatch) {
			// To merge p2's text into p1:
			// We must delete: p1_close (1), bq_close (1), and p2_open (1) = 3 tokens.
			const startOfP2Text = $from.pos;
			tr.delete(startOfP2Text - 3, startOfP2Text);
			dispatch(tr.scrollIntoView());
		}
		return true;
	}

	// SCENARIO 2: [blockquote(p1, |p2)]
	// We are inside the blockquote, p2 follows p1.
	if ($from.depth >= 2 && $from.node(1).type.name === "blockquote") {
		const index = $from.index(1); // Index of current child within blockquote
		if (index > 0) {
			const prevSibling = $from.node(1).child(index - 1);
			// Only merge if the previous sibling is also a paragraph
			if (prevSibling.type.name === "paragraph") {
				if (dispatch) {
					// To merge p2's text into p1:
					// We must delete: p1_close (1) and p2_open (1) = 2 tokens.
					const startOfP2Text = $from.pos;
					tr.delete(startOfP2Text - 2, startOfP2Text);
					dispatch(tr.scrollIntoView());
				}
				return true;
			}
		}
	}

	return false;
};

export const joinBlockquoteForward: Command = (state, dispatch) => {
	const { $from, empty } = state.selection;

	// Only trigger if selection is empty and at the end of a paragraph
	if (
		!empty ||
		$from.parentOffset !== $from.parent.content.size ||
		$from.parent.type.name !== "paragraph"
	) {
		return false;
	}

	const tr = state.tr;
	const $after = state.doc.resolve($from.after());
	const nodeAfter = $after.nodeAfter;

	// SCENARIO 1: [p1|] [blockquote(p2)]
	// We are at the end of a top-level paragraph followed by a blockquote
	if (nodeAfter?.type.name === "blockquote") {
		if (dispatch) {
			// To merge p2's text into p1:
			// We must delete: p1_close (1), bq_open (1), and p2_open (1) = 3 tokens.
			const endOfP1Text = $from.pos;
			tr.delete(endOfP1Text, endOfP1Text + 3);
			dispatch(tr.scrollIntoView());
		}
		return true;
	}

	// SCENARIO 2: [blockquote(p1|, p2)]
	// We are inside the blockquote, at the end of p1, followed by p2
	if ($from.depth >= 2 && $from.node(1).type.name === "blockquote") {
		const index = $from.index(1);
		const parentBQ = $from.node(1);

		// If there is another child after this one inside the blockquote
		if (index < parentBQ.childCount - 1) {
			const nextSibling = parentBQ.child(index + 1);
			if (nextSibling.type.name === "paragraph") {
				if (dispatch) {
					// To merge p2's text into p1:
					// We must delete: p1_close (1) and p2_open (1) = 2 tokens.
					const endOfP1Text = $from.pos;
					tr.delete(endOfP1Text, endOfP1Text + 2);
					dispatch(tr.scrollIntoView());
				}
				return true;
			}
		}
	}

	return false;
};
