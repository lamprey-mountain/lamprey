import { type Command, EditorState, TextSelection } from "prosemirror-state";
import { DOMParser } from "prosemirror-model";
type NodeViewConstructor = any;
import {
	type EditorProps as ProsemirrorEditorProps,
	EditorView,
} from "prosemirror-view";
import { createEffect, onCleanup, onMount } from "solid-js";
import { useCtx } from "../../../context";
import { initTurndownService } from "../../../turndown.ts";
import { decorate } from "../../../markdown_utils.tsx";
import { schema as defaultSchema } from "./schema";
import { serializeToMarkdown } from "./serializer.ts";
import { convertEmojiInText } from "./emoji-plugin.ts";

const turndown = initTurndownService();

export type EditorOptions = {
	schema?: typeof defaultSchema;
	createState: (schema: typeof defaultSchema) => EditorState;
	nodeViews?: (view: EditorView) => Record<string, NodeViewConstructor>;
	handleKeyDown?: (view: EditorView, event: KeyboardEvent) => boolean;
	handleDOMEvents?: ProsemirrorEditorProps["handleDOMEvents"];
	autofocus?: boolean;
};

export type EditorViewProps = {
	placeholder?: string;
	disabled?: boolean;
	onUpload?: (file: File) => void;
	onSubmit?: (text: string) => boolean | Promise<boolean>;
	onChange?: (state: EditorState) => void;
	channelId?: string; // Needed for autocomplete
	submitOnEnter?: boolean;
	autofocus?: boolean;
};

function isInsideCodeBlock(state: EditorState): boolean {
	const { $from } = state.selection;
	for (let d = $from.depth; d > 0; d--) {
		if ($from.node(d).type.name === "code_block") return true;
	}
	return false;
}

// function isInsideCodeBlock(state: EditorState): boolean {
//   const { $from } = state.selection;
//   for (let d = $from.depth; d >= 0; d--) {
//     if ($from.node(d).type === state.schema.nodes.code_block) return true;
//   }
//   return false;
// }

export const createEditor = (opts: EditorOptions) => {
	const schema = opts.schema ?? defaultSchema;
	let editorRef!: HTMLDivElement;
	let view: EditorView | undefined;

	let currentProps: EditorViewProps = {};

	const submitCommand: Command = (state, dispatch) => {
		const res = currentProps.onSubmit?.(serializeToMarkdown(state.doc).trim());
		if (res instanceof Promise) {
			res.then((shouldClear) => {
				if (shouldClear) {
					view?.dispatch(
						view.state.tr.deleteRange(0, view.state.doc.nodeSize - 2),
					);
				}
			});
		} else if (res) {
			dispatch?.(state.tr.deleteRange(0, state.doc.nodeSize - 2));
		}
		return true;
	};

	return {
		schema,
		setState(state?: EditorState) {
			view?.updateState(state ?? opts.createState(schema));
		},
		focus() {
			view?.focus();
		},
		get view() {
			return view;
		},
		View(props: EditorViewProps) {
			const ctx = useCtx();

			createEffect(() => {
				currentProps = props;
			});

			onMount(() => {
				view = new EditorView(editorRef!, {
					domParser: DOMParser.fromSchema(schema),
					state: opts.createState(schema),
					decorations(state) {
						return decorate(state, props.placeholder);
					},
					nodeViews: opts.nodeViews?.(view!),
					handleDOMEvents: opts.handleDOMEvents,
					handlePaste(view, event, slice) {
						const files = Array.from(event.clipboardData?.files ?? []);
						if (files.length) {
							for (const file of files) props.onUpload?.(file);
							return true;
						}

						const isInternal = event.clipboardData?.types.includes(
							"application/x-prosemirror-slice",
						);
						if (isInternal) {
							return false;
						}

						const html = event.clipboardData?.getData("text/html");
						const plainText = event.clipboardData?.getData("text/plain");

						const str = html ? turndown.turndown(html) : (plainText ??
							slice.content.textBetween(0, slice.content.size, "\n"));

						const tr = view.state.tr;
						if (
							!tr.selection.empty &&
							/^(https?:\/\/|mailto:)\S+$/i.test(str.trim())
						) {
							const url = str.trim();
							const { from, to } = tr.selection;
							tr.insertText(`](${url})`, to);
							tr.insertText("[", from);
							tr.setSelection(
								TextSelection.create(tr.doc, tr.mapping.map(to)),
							);
							view.dispatch(
								tr.scrollIntoView().setMeta("paste", true).setMeta(
									"uiEvent",
									"paste",
								),
							);
							return true;
						}

						const { content, hasEmoji } = convertEmojiInText(
							schema,
							str,
						);

						if (hasEmoji) {
							const { from, to } = view.state.selection;
							view.dispatch(
								view.state.tr.replaceWith(from, to, content)
									.scrollIntoView()
									.setMeta("paste", true),
							);
							return true;
						}

						view.dispatch(
							view.state.tr.replaceSelectionWith(schema.text(str))
								.scrollIntoView()
								.setMeta("paste", true),
						);
						return true;
					},
					handleKeyDown(view, event) {
						if (opts.handleKeyDown?.(view, event)) return true;

						if (event.key === "Enter" && !event.shiftKey) {
							if (isInsideCodeBlock(view.state)) {
								view.dispatch(view.state.tr.insertText("\n").scrollIntoView());
								return true;
							}
							if (props.submitOnEnter ?? true) {
								return submitCommand(view.state, view.dispatch);
							} else {
								// submitOnEnter is false, insert newline instead
								view.dispatch(view.state.tr.insertText("\n").scrollIntoView());
								return true;
							}
						}

						if (event.key === "Enter" && event.ctrlKey) {
							return submitCommand(view.state, view.dispatch);
						}

						return false;
					},
					editable: () => !(props.disabled ?? false),
					dispatchTransaction(tr) {
						const newState = view!.state.apply(tr);
						view!.updateState(newState);
						props.onChange?.(newState);
					},
				});

				if (props.autofocus ?? opts.autofocus ?? true) {
					view.focus();
				}
			});

			onCleanup(() => {
				view?.destroy();
			});

			createEffect(() => {
				// update when placeholder changes too
				props.placeholder;

				view?.setProps({
					editable: () => !(props.disabled ?? false),
				});
			});

			return (
				<div
					class="editor"
					classList={{ "disabled": props.disabled ?? false }}
					tabindex={0}
					ref={editorRef!}
					role="textbox"
					aria-label="chat input"
					aria-placeholder={props.placeholder}
					aria-multiline="true"
				>
				</div>
			);
		},
	};
};
