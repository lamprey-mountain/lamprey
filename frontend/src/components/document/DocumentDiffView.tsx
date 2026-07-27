// TODO: deduplicate this code with DocumentEditor

import type { Parser as MarkdownParser } from "@lamprey/markdown";
import { DOMParser } from "prosemirror-model";
import { EditorState } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { createEffect, createMemo, createResource, onCleanup } from "solid-js";
import { useApi } from "@/api";
import { parser as markdownParser } from "@/lib/markdown";
import {
	createDiffPlugin,
	diffPluginKey,
} from "../features/editor/diff-plugin";
import { createMarkdownHighlightPlugin } from "../features/editor/markdown-highlight-plugin";
import {
	createPlaceholderPlugin,
	placeholderPluginKey,
} from "../features/editor/mod";
import { createEditorNodeViews } from "../features/editor/node-views";
import { schema } from "../features/editor/schema";
import { useDocument } from "./context";
import { computeDiffMarks, serdocToDoc } from "./diff";
import type { ChangesetSelection } from "./types";

const domParser = DOMParser.fromSchema(schema);

let md: MarkdownParser;
markdownParser.then((p) => (md = p));

export type DocumentDiffViewProps = {
	channelId: string;
	changeset: ChangesetSelection;
	placeholder?: string;
};

// FIXME: if two history items are selected, it shouldn't matter which one is hovered and which one is selected
// diff highlighting seems to only work in one direction?

export const DocumentDiffView = (props: DocumentDiffViewProps) => {
	const api = useApi();
	const doc = useDocument();

	const [newSerdoc] = createResource(
		() => [props.channelId, props.changeset.end_seq] as const,
		async ([channelId, endSeq]) =>
			api.documents.getRevisionContent(channelId, `${channelId}@${endSeq}`),
	);

	const [oldSerdoc] = createResource(
		() => [props.channelId, props.changeset.start_seq] as const,
		async ([channelId, startSeq]) => {
			if (startSeq > 0) {
				return await api.documents.getRevisionContent(
					channelId,
					`${channelId}@${startSeq - 1}`,
				);
			} else {
				// empty doc for first changeset
				return { components: [] };
			}
		},
	);

	const diffMarks = createMemo(() => {
		const ns = newSerdoc();
		const os = oldSerdoc();
		if (!ns) return [];
		if (!os) return [];
		// PERF: computeDiffMarks + the later createEffect cause serdoc to be converted
		// into a prosemirror document twice. maybe i should createMemo serdocToDoc for old
		// and new serdoc?
		const marks = computeDiffMarks(os, ns);
		return marks;
	});

	const createState = () => {
		const doc = domParser.parse(document.createElement("div"));

		return EditorState.create({
			doc,
			schema,
			plugins: [
				createPlaceholderPlugin(),
				createDiffPlugin(diffMarks),
				createMarkdownHighlightPlugin(),
			],
		});
	};

	const init = (el: HTMLDivElement) => {
		const state = createState();
		const view = new EditorView(el, {
			state,
			domParser,
			nodeViews: createEditorNodeViews()(),
			editable: () => false,
			dispatchTransaction(tr) {
				const newState = view.state.apply(tr);
				view.updateState(newState);

				if (md) {
					// PERF: reuse parsed
					const parsed = md.parse(newState.doc.textContent);
					doc.setHeadings(parsed.headers());
				}
			},
		});

		onCleanup(() => view.destroy());

		// reactively update placeholder
		createEffect(() => {
			const tr = view.state.tr.setMeta(placeholderPluginKey, props.placeholder);
			view.dispatch(tr);
		});

		// reactively update diff marks
		createEffect(() => {
			const tr = view.state.tr.setMeta(diffPluginKey, { marks: diffMarks() });
			view.dispatch(tr);
		});

		// reactively update doc
		createEffect(() => {
			const ns = newSerdoc();
			if (!ns) return;

			const doc = serdocToDoc(ns);
			const tr = view.state.tr.replaceWith(
				0,
				view.state.doc.content.size,
				doc.content,
			);
			view.dispatch(tr);
		});
	};

	return (
		<div
			class="editor disabled diff-view"
			ref={init}
			role="textbox"
			aria-label="document diff view"
			aria-placeholder={props.placeholder}
			aria-multiline="true"
		></div>
	);
};
