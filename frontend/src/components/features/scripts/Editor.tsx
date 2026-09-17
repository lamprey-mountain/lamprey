import {
	autocompletion,
	CompletionContext,
	closeBrackets,
	// autocompletion, completionKeymap, closeBrackets,
	closeBracketsKeymap,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { javascript } from "@codemirror/lang-javascript";
import {
	bracketMatching,
	foldGutter,
	HighlightStyle,
	indentOnInput,
	syntaxHighlighting,
	// indentOnInput,
	// bracketMatching, foldGutter, foldKeymap
} from "@codemirror/language";
import { highlightSelectionMatches } from "@codemirror/search";
import { Compartment, EditorState, Extension } from "@codemirror/state";
import {
	crosshairCursor,
	Decoration,
	DecorationSet,
	drawSelection,
	dropCursor,
	EditorView,
	highlightActiveLine,
	highlightActiveLineGutter,
	highlightSpecialChars,
	keymap,
	lineNumbers,
	MatchDecorator,
	placeholder,
	rectangularSelection,
	ViewPlugin,
	ViewUpdate,
	WidgetType,
} from "@codemirror/view";
import {
	createEffect,
	createResource,
	createSignal,
	onCleanup,
	onMount,
} from "solid-js";
import type {
	MessageEnvelope,
	MessageSync,
	Script,
	Stream,
	WebtransportClient,
} from "ts-sdk";
import { yCollab } from "y-codemirror.next";
import * as Y from "yjs";
import { useApi } from "@/api";
import { getGetUrl } from "@/media/util";
import { base64UrlDecode, base64UrlEncode } from "../editor/editor-utils";
import { cursorPlugin } from "./codemirror-editor-cursors";
import { useScript } from "./context";
import { highlight, theme } from "./theme";
// import {lintKeymap} from "@codemirror/lint"

export const CodeEditor = (props: {
	script: Script;
	onChange?: (val: string) => void;
}) => {
	const api = useApi();
	const scriptContext = useScript();

	const getUrl = getGetUrl();

	const [loading, setLoading] = createSignal(true);

	let editorRef!: HTMLDivElement;
	let view: EditorView;
	const stateConfigCompartment = new Compartment();

	const [mediaContent] = createResource(
		() => props.script,
		async (s) => {
			if (s.latest_version.location.type === "Hosted") {
				return fetch(getUrl(s.latest_version.location.media)).then((r) =>
					r.text(),
				);
			}
			return undefined;
		},
	);

	const ydoc = new Y.Doc();
	let stream: Stream | null = null;

	const onSync = (sync: MessageSync, _raw: MessageEnvelope) => {
		if (sync.type === "DocumentEdit") {
			if (sync.channel_id !== scriptContext.channel_id) return;
			if (sync.branch_id !== props.script.id) return;
			const update = (
				(sync.update as unknown) instanceof Uint8Array
					? sync.update
					: base64UrlDecode(sync.update as unknown as string)
			) as Uint8Array;
			Y.applyUpdate(ydoc, update, { key: "server" });
		} else if (sync.type === "DocumentPresence") {
			api.events.emit("sync", [sync, _raw]);
		} else if (sync.type === "DocumentSubscribed") {
			if (sync.channel_id !== scriptContext.channel_id) return;
			if (sync.branch_id !== props.script.id) return;
			setLoading(false);
		}
	};

	// ydoc update listener
	createEffect(() => {
		if (props.script.latest_version.location.type !== "Document") return;

		const handler = (update: Uint8Array, origin: any) => {
			if (origin && origin.key === "server") return;

			const data = {
				type: "DocumentEdit" as const,
				channel_id: scriptContext.channel_id,
				branch_id: props.script.id,
				redex_id: props.script.id,
				update: base64UrlEncode(update),
			};

			if (stream) {
				stream.send(data);
			} else {
				api.client.send(data);
			}
		};

		ydoc.on("update", handler);
		onCleanup(() => ydoc.off("update", handler));
	});

	// manage subscriptions
	createEffect(() => {
		if (props.script.latest_version.location.type !== "Document") return;

		const channelId = scriptContext.channel_id;
		const branchId = props.script.id;
		const stateVector = base64UrlEncode(Y.encodeStateVector(ydoc));

		if (api.client.isWebtransport) {
			const wt = api.client as WebtransportClient;
			const newStream = wt.subscribeDocument({
				channel_id: channelId,
				branch_id: branchId,
				state_vector: stateVector,
				onSync,
			});
			stream = newStream;
			onCleanup(() => {
				newStream.close();
				stream = null;
			});
		} else {
			api.client.send({
				type: "Subscribe",
				documents: [],
			});
			api.client.send({
				type: "Subscribe",
				documents: [
					{
						channel_id: channelId,
						branch_id: branchId,
						state_vector: stateVector,
					},
				],
			});

			api.events.on("sync", ([sync, envelope]) => onSync(sync, envelope));
		}
	});

	createEffect(() => {
		if (props.script.latest_version.location.type !== "Document") {
			setLoading(mediaContent.loading);
		}
	});

	onMount(() => {
		const extensions = [
			drawSelection(),
			lineNumbers(),
			foldGutter(),
			highlightSpecialChars(),
			dropCursor(),
			EditorState.allowMultipleSelections.of(true),
			history(),
			indentOnInput(),
			bracketMatching(),
			closeBrackets(),
			autocompletion(),
			// FIXME: rectangular selection
			// rectangularSelection(),
			// crosshairCursor(),
			highlightActiveLine(),
			highlightSelectionMatches(),
			drawSelection(),
			keymap.of([
				...closeBracketsKeymap,
				...defaultKeymap,
				// ...searchKeymap,
				...historyKeymap,
				// ...foldKeymap,
				// ...completionKeymap,
				// ...lintKeymap
				{
					key: "Mod-s",
					// preventDefault: true,
					run(_view) {
						// const content = view.state.doc.toString();

						api.scripts
							.updateContentInnerAsync(
								props.script.channel_id,
								props.script.id,
								{
									format: "Javascript",
									location: { type: "Document" },
								},
							)
							.then((res) => {
								console.log("AAAA", res);
							});

						return true;
					},
				},
			]),
			theme,
			javascript(), // TODO(future): swap this depending on language
			syntaxHighlighting(highlight),
			stateConfigCompartment.of([
				EditorView.editable.of(!loading()),
				EditorState.readOnly.of(loading()),
			]),
			EditorView.updateListener.of((update) => {
				if (update.docChanged && props.onChange) {
					props.onChange(update.state.doc.toString());
				}
			}),
		];

		// TODO: manage documents/subscriptions in script context
		if (props.script.latest_version.location.type === "Document") {
			const ytype = ydoc.getText("doc");
			const undoManager = new Y.UndoManager(ytype);
			extensions.push(yCollab(ytype, null, { undoManager }));
			extensions.push(
				cursorPlugin(
					api,
					scriptContext.channel_id,
					props.script.id,
					ytype,
					() => stream,
				),
			);
		} else {
			// TODO(?): move mediaContent-specific logic here
		}

		view = new EditorView({
			parent: editorRef,
			extensions,
		});
	});

	// sync source text for media sources
	createEffect(() => {
		if (!view) return;

		if (props.script.latest_version.location.type === "Document") return;

		// TODO: show indicator when media changes, button to reload mediaContent
		const nextDoc = mediaContent();
		const currentDoc = view.state.doc.toString();

		if (nextDoc !== undefined && currentDoc !== nextDoc) {
			view.dispatch({
				changes: { from: 0, to: currentDoc.length, insert: nextDoc },
			});
		}
	});

	// disable editor when loading
	createEffect(() => {
		view.dispatch({
			effects: stateConfigCompartment.reconfigure([
				EditorView.editable.of(!loading()),
				EditorState.readOnly.of(loading()),
			]),
		});
	});

	return <div ref={editorRef!}></div>;
};
