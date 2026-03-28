import { EditorState, Plugin, PluginKey } from "prosemirror-state";
import { DOMParser } from "prosemirror-model";
type NodeViewConstructor = any;
import {
	Decoration,
	DecorationSet,
	type EditorProps as ProsemirrorEditorProps,
	EditorView,
} from "prosemirror-view";
import { onCleanup, onMount } from "solid-js";
import { schema as defaultSchema } from "./schema";
import { pastePluginKey, submitPluginKey } from "./core-plugins.ts";

const placeholderPluginKey = new PluginKey<string>("placeholder");

export function createPlaceholderPlugin() {
	return new Plugin<string>({
		key: placeholderPluginKey,
		state: {
			init: () => "",
			apply(tr, prev) {
				const meta = tr.getMeta(placeholderPluginKey);
				if (meta !== undefined) return meta;
				return prev;
			},
		},
		props: {
			decorations(state) {
				const text = placeholderPluginKey.getState(state);
				if (!text) return DecorationSet.empty;
				const isEmpty = !state.doc.firstChild?.content.size;
				if (!isEmpty) return DecorationSet.empty;

				const widget = Decoration.widget(1, () => {
					const span = document.createElement("span");
					span.className = "placeholder";
					span.textContent = text;
					return span;
				});
				return DecorationSet.create(state.doc, [widget]);
			},
		},
	});
}

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

type ViewInstance = {
	view: EditorView;
	placeholderPlugin: ReturnType<typeof createPlaceholderPlugin>;
	props: EditorViewProps;
	pendingUpdate?: boolean;
};

const viewInstances = new WeakMap<EditorView, ViewInstance>();

function scheduleUpdate(instance: ViewInstance, newProps: EditorViewProps) {
	if (instance.pendingUpdate) return;
	instance.pendingUpdate = true;

	queueMicrotask(() => {
		instance.pendingUpdate = false;
		const { view, placeholderPlugin, props: prevProps } = instance;
		if (view.isDestroyed) return;

		const needsUpdate = prevProps.onSubmit !== newProps.onSubmit ||
			prevProps.submitOnEnter !== newProps.submitOnEnter ||
			prevProps.onUpload !== newProps.onUpload ||
			prevProps.placeholder !== newProps.placeholder;

		if (needsUpdate) {
			const tr = view.state.tr;
			tr.setMeta(submitPluginKey, {
				onSubmit: newProps.onSubmit,
				submitOnEnter: newProps.submitOnEnter,
			});
			tr.setMeta(pastePluginKey, {
				onUpload: newProps.onUpload,
			});
			if (newProps.placeholder !== undefined) {
				tr.setMeta(placeholderPlugin, newProps.placeholder ?? "");
			}
			view.dispatch(tr);

			instance.props = newProps;
		}
	});
}

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
				const placeholderPlugin = createPlaceholderPlugin();
				view = new EditorView(editorRef!, {
					domParser: DOMParser.fromSchema(schema),
					state: opts.createState(schema),
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

				// Initialize plugin state on mount
				const tr = view.state.tr;
				tr.setMeta(submitPluginKey, {
					onSubmit: props.onSubmit,
					submitOnEnter: props.submitOnEnter ?? true,
				});
				tr.setMeta(pastePluginKey, {
					onUpload: props.onUpload,
				});
				if (props.placeholder) {
					tr.setMeta(placeholderPlugin, props.placeholder);
				}
				view.dispatch(tr);

				viewInstances.set(view, { view, placeholderPlugin, props });
			});

			onCleanup(() => {
				if (view) {
					viewInstances.delete(view);
				}
				view?.destroy();
			});

			// Update props on every render - queued via microtask to avoid
			// dispatching during render
			if (view) {
				const instance = viewInstances.get(view);
				if (instance) {
					view.setProps({
						editable: () => !(props.disabled ?? false),
					});
					scheduleUpdate(instance, props);
				}
			}

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
