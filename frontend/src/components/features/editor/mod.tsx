import { EditorState } from "prosemirror-state";
import { DOMParser } from "prosemirror-model";
type NodeViewConstructor = any;
import {
	type EditorProps as ProsemirrorEditorProps,
	EditorView,
} from "prosemirror-view";
import { createEffect, onCleanup, onMount } from "solid-js";
import { decorate } from "../../../markdown_utils.tsx";
import { schema as defaultSchema } from "./schema";
import { pastePluginKey, submitPluginKey } from "./core-plugins.ts";

export type EditorOptions = {
	schema?: typeof defaultSchema;
	createState: (schema: typeof defaultSchema) => EditorState;
	nodeViews?: (view: EditorView) => Record<string, NodeViewConstructor>;
	handleDOMEvents?: ProsemirrorEditorProps["handleDOMEvents"];
	autofocus?: boolean;
};

export type EditorViewProps = {
	placeholder?: string;
	disabled?: boolean;
	onUpload?: (file: File) => void;
	onSubmit?: (text: string) => boolean | Promise<boolean>;
	onChange?: (state: EditorState) => void;
	channelId?: string;
	submitOnEnter?: boolean;
	autofocus?: boolean;
};

export const createEditor = (opts: EditorOptions) => {
	const schema = opts.schema ?? defaultSchema;
	let editorRef!: HTMLDivElement;
	let view: EditorView | undefined;

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
			onMount(() => {
				view = new EditorView(editorRef!, {
					domParser: DOMParser.fromSchema(schema),
					state: opts.createState(schema),
					decorations(state) {
						return decorate(state, props.placeholder);
					},
					nodeViews: opts.nodeViews?.(view!),
					handleDOMEvents: opts.handleDOMEvents,
					editable: () => !(props.disabled ?? false),
					dispatchTransaction(tr) {
						const newState = view!.state.apply(tr);
						view!.updateState(newState);
						console.log(
							"editor new doc",
							newState.doc.toJSON(),
							newState.selection.toJSON(),
						);
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
				if (!view) return;

				view.setProps({
					editable: () => !(props.disabled ?? false),
				});

				// Reactively sync component callbacks/state into the plugins without re-creating them
				const tr = view.state.tr;
				tr.setMeta(submitPluginKey, {
					onSubmit: props.onSubmit,
					submitOnEnter: props.submitOnEnter,
				});
				tr.setMeta(pastePluginKey, {
					onUpload: props.onUpload,
				});

				view.dispatch(tr);
			});

			return (
				<div
					class="editor"
					classList={{ disabled: props.disabled ?? false }}
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
