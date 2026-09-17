import { DOMParser } from "prosemirror-model";
import { EditorState } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { createEffect, createSignal, onMount } from "solid-js";
import { Icon } from "@/atoms/Icon";
import { icSearch } from "@/utils/icons";
import {
	createPlaceholderPlugin,
	placeholderPluginKey,
} from "../editor/plugin-placeholder";
import type { SearchConfig } from "./context";

export type SearchInputProps = {
	config: SearchConfig;
	// channel?: ThreadT;
	// room?: RoomT;
	autofocus?: boolean;
	disabled?: boolean;
	placeholder?: string;
	value?: string;
};

export const SearchInput = (props: SearchInputProps) => {
	const [editorRef, setEditorRef] = createSignal<HTMLElement>();

	onMount(() => {
		const schema = props.config.schema;
		const domParser = DOMParser.fromSchema(schema);

		const state = EditorState.create({
			schema,
			doc: schema.topNodeType.create(),
			plugins: [createPlaceholderPlugin()],
		});

		const view = new EditorView(editorRef()!, {
			state,
			domParser,
			// nodeViews: opts.nodeViews?.(view!),
			// handleDOMEvents: opts.handleDOMEvents,
			editable: () => !(props.disabled ?? false),
			// dispatchTransaction(tr) {
			// 	const newState = view?.state.apply(tr);
			// 	if (!newState || !view) return;
			// 	view.updateState(newState);
			// 	props.onChange?.(newState);
			// },
		});

		createEffect(() => {
			view.dispatch(
				view.state.tr.setMeta(placeholderPluginKey, props.placeholder),
			);
		});
	});

	return (
		<div class="search2-container">
			<div
				class="search2-input"
				ref={setEditorRef}
				role="textbox"
				aria-label="search input"
				aria-placeholder={props.placeholder}
				aria-multiline="false"
				aria-disabled={props.disabled ?? false}
			></div>
			<Icon src={icSearch} alt="" />
		</div>
	);
};
