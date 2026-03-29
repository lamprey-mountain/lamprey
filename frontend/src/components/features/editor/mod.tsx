import { DOMParser } from "prosemirror-model";
import { type EditorState, Plugin, PluginKey } from "prosemirror-state";

type NodeViewConstructor = any;

import {
	Decoration,
	DecorationSet,
	EditorView,
	type EditorProps as ProsemirrorEditorProps,
} from "prosemirror-view";
import { onCleanup, onMount } from "solid-js";
import { pastePluginKey, submitPluginKey } from "./core-plugins.ts";
import { schema as defaultSchema } from "./schema";

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
	lastOnSubmit: EditorViewProps["onSubmit"];
	lastSubmitOnEnter: EditorViewProps["submitOnEnter"];
	lastOnUpload: EditorViewProps["onUpload"];
	lastPlaceholder: EditorViewProps["placeholder"];
};

const viewInstances = new WeakMap<EditorView, ViewInstance>();

function scheduleUpdate(instance: ViewInstance, newProps: EditorViewProps) {
	if (instance.pendingUpdate) return;
	instance.pendingUpdate = true;

	queueMicrotask(() => {
		instance.pendingUpdate = false;
		const { view, placeholderPlugin } = instance;
		if (view.isDestroyed) return;

		const needsUpdate =
			instance.lastOnSubmit !== newProps.onSubmit ||
			instance.lastSubmitOnEnter !== newProps.submitOnEnter ||
			instance.lastOnUpload !== newProps.onUpload ||
			instance.lastPlaceholder !== newProps.placeholder;

		if (needsUpdate) {
			const tr = view.state.tr;
			tr.setMeta(submitPluginKey, {
				onSubmit: newProps.onSubmit,
				submitOnEnter: newProps.submitOnEnter ?? true,
			});
			tr.setMeta(pastePluginKey, {
				onUpload: newProps.onUpload,
			});
			if (newProps.placeholder !== undefined) {
				tr.setMeta(placeholderPlugin, newProps.placeholder ?? "");
			}
			view.dispatch(tr);

			instance.props = newProps;
			instance.lastOnSubmit = newProps.onSubmit;
			instance.lastSubmitOnEnter = newProps.submitOnEnter ?? true;
			instance.lastOnUpload = newProps.onUpload;
			instance.lastPlaceholder = newProps.placeholder;
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
			let placeholderPlugin:
				| ReturnType<typeof createPlaceholderPlugin>
				| undefined;
			let initialized = false;
			let lastDisabled = props.disabled ?? false;

			onMount(() => {
				placeholderPlugin = createPlaceholderPlugin();
				view = new EditorView(editorRef!, {
					domParser: DOMParser.fromSchema(schema),
					state: opts.createState(schema),
					nodeViews: opts.nodeViews?.(view!),
					handleDOMEvents: opts.handleDOMEvents,
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

				viewInstances.set(view, {
					view,
					placeholderPlugin,
					props,
					lastOnSubmit: props.onSubmit,
					lastSubmitOnEnter: props.submitOnEnter ?? true,
					lastOnUpload: props.onUpload,
					lastPlaceholder: props.placeholder,
				});
				initialized = true;
			});

			onCleanup(() => {
				if (view) {
					viewInstances.delete(view);
				}
				view?.destroy();
			});

			// Update props on every render - queued via microtask to avoid
			// dispatching during render
			if (initialized && view) {
				const instance = viewInstances.get(view);
				if (instance) {
					// Only update if props actually changed
					if (lastDisabled !== (props.disabled ?? false)) {
						lastDisabled = props.disabled ?? false;
						view.setProps({
							editable: () => !lastDisabled,
						});
					}
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
				></div>
			);
		},
	};
};
