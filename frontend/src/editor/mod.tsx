import { type Command, EditorState, TextSelection } from "prosemirror-state";
import { DOMParser, type NodeViewConstructor } from "prosemirror-model";
import {
	type EditorProps as ProsemirrorEditorProps,
	EditorView,
} from "prosemirror-view";
import { createEffect, onCleanup, onMount } from "solid-js";
import { useCtx } from "../context";
import { useAutocomplete } from "../contexts/mod.tsx";
import { initTurndownService } from "../turndown.ts";
import { decorate, md } from "../markdown.tsx";
import { handleAutocomplete } from "./editor-utils.ts";
import { schema as defaultSchema } from "./schema";

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
	onSubmit?: (text: string) => boolean;
	onChange?: (state: EditorState) => void;
	channelId?: string; // Needed for autocomplete
	submitOnEnter?: boolean;
	autofocus?: boolean;
};

function isInsideCodeBlock(state: EditorState): boolean {
	const pos = state.selection.from;
	const textBefore = state.doc.textBetween(0, pos, "\n");
	const lines = textBefore.split("\n");
	let count = 0;
	for (const line of lines) {
		if (line.trim().startsWith("```")) {
			count++;
		}
	}
	return count % 2 === 1;
}

export const createEditor = (opts: EditorOptions) => {
	const schema = opts.schema ?? defaultSchema;
	let editorRef!: HTMLDivElement;
	let view: EditorView | undefined;

	let currentProps: EditorViewProps = {};

	const submitCommand: Command = (state, dispatch) => {
		const shouldClear = currentProps.onSubmit?.(state.doc.textContent.trim());
		if (shouldClear) {
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
			const autocompleteCtx = useAutocomplete();

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
						const str = html
							? turndown.turndown(html)
							: (plainText !== null && plainText !== undefined
								? plainText
								: slice.content.textBetween(0, slice.content.size, "\n"));

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

						const tokens = md.lexer(str);
						const hasSpecial = (t: any): boolean => {
							if (t.type === "mention") return true;
							if (t.tokens) return t.tokens.some(hasSpecial);
							return false;
						};
						const hasStructured = tokens.some(hasSpecial);

						if (!html && !hasStructured) {
							view.dispatch(
								view.state.tr.replaceSelectionWith(schema.text(str))
									.scrollIntoView()
									.setMeta("paste", true),
							);
							return true;
						}

						const div = document.createElement("div");
						div.style.whiteSpace = "pre-wrap";
						div.innerHTML = md.parser(tokens).trimEnd();
						const newSlice = DOMParser.fromSchema(schema).parseSlice(div, {
							preserveWhitespace: "full",
						});
						view.dispatch(
							view.state.tr.replaceSelection(newSlice).scrollIntoView()
								.setMeta("paste", true),
						);
						return true;
					},
					handleKeyDown(view, event) {
						if (opts.handleKeyDown?.(view, event)) return true;

						if (event.key === "Enter" && !event.shiftKey) {
							if (isInsideCodeBlock(view.state)) {
								view.dispatch(view.state.tr.insertText("\n"));
								return true;
							}
							if (props.submitOnEnter ?? true) {
								return submitCommand(view.state, view.dispatch);
							} else {
								// submitOnEnter is false, insert newline instead
								view.dispatch(view.state.tr.insertText("\n"));
								return true;
							}
						}

						if (event.key === "Enter" && event.ctrlKey) {
							return submitCommand(view.state, view.dispatch);
						}

						return handleAutocomplete(
							view,
							event,
							ctx,
							autocompleteCtx,
							schema,
							props.channelId || "",
						);
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
