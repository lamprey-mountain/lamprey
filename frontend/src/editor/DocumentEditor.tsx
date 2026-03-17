import * as Y from "yjs";
import { type Command, EditorState, TextSelection } from "prosemirror-state";
import { DOMParser } from "prosemirror-model";
import {
	initProseMirrorDoc,
	redo,
	undo,
	ySyncPlugin,
	yUndoPlugin,
} from "y-prosemirror";
import { keymap } from "prosemirror-keymap";
import { render } from "solid-js/web";
import { getEmojiUrl } from "../media/util.tsx";
import { md } from "../markdown_utils.tsx";
import { useApi } from "../api.tsx";
import { MessageSync } from "sdk";
import { cursorPlugin } from "./editor-cursors.ts";
import {
	createEditor as createBaseEditor,
	type EditorViewProps,
} from "./mod.tsx";
import { schema } from "./schema.ts";
import {
	base64UrlDecode,
	base64UrlEncode,
	createListContinueCommand,
	createWrapCommand,
} from "./editor-utils.ts";
import { type Api } from "../api.tsx";
import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import { useFormattingToolbar } from "../contexts/formatting-toolbar.tsx";
import { setFormattingToolbarView } from "../contexts/FormattingToolbar.tsx";
import { EditorView } from "prosemirror-view";
import {
	createDiffPlugin,
	type DiffMark,
	diffPluginKey,
} from "./diff-plugin.ts";
import { createToolbarPlugin } from "./toolbar-plugin.ts";

let isApplyingFormat = false;
export const setIsApplyingFormat = (value: boolean) => {
	isApplyingFormat = value;
};

const UserMention = (
	props: { api: Api; userId: string; channelId: string },
) => {
	const channel = props.api.channels.fetch(() => props.channelId);
	const user = props.api.users.fetch(() => props.userId);
	const roomMember = props.api.room_members.fetch(
		() => channel()?.room_id!,
		() => props.userId,
	);

	const name = () => {
		return roomMember()?.override_name ?? user()?.name ?? props.userId;
	};

	return <span class="mention-user">@{name()}</span>;
};

const ChannelMention = (props: { api: Api; channelId: string }) => {
	const channel = props.api.channels.fetch(() => props.channelId);
	return (
		<span class="mention-channel">#{channel()?.name ?? props.channelId}</span>
	);
};

const RoleMention = (props: { api: Api; roleId: string }) => {
	const role = () => props.api.roles.cache.get(props.roleId);
	return <span class="mention-role">@{role()?.name ?? "..."}</span>;
};

const Emoji = (props: { id: string; name: string }) => {
	const url = getEmojiUrl(props.id);
	return (
		<img
			class="emoji"
			src={url}
			alt={`:${props.name}:`}
			title={`:${props.name}:`}
		/>
	);
};

type EditorProps = {
	initialContent?: string;
	keymap?: { [key: string]: Command };
	initialSelection?: "start" | "end";
	mentionRenderer?: (node: HTMLElement, userId: string) => void;
	mentionChannelRenderer?: (node: HTMLElement, channelId: string) => void;
	diffMarks?: DiffMark[];
	diffMode?: () => boolean; // when true, editor is readonly and cursors are hidden
};

export const createEditor = (
	opts: EditorProps,
): {
	schema: any;
	setState: (state?: any) => void;
	focus: () => void;
	view: any;
	View: (props: any) => any;
	subscribe: (channelId: string, branchId: string) => void;
	isSubscribed: () => boolean;
	setDiffMarks: (marks: any) => void;
	createReadonlyState: (content: string) => any;
	createReadonlyStateFromHtml: (html: string) => any;
} => {
	const api = useApi();
	const toolbarPlugin = createToolbarPlugin();

	const [isSubscribed, setIsSubscribed] = createSignal(false);
	const [currentChannelId, setCurrentChannelId] = createSignal(
		"no channel id!",
	);
	const [currentBranchId, setCurrentBranchId] = createSignal("no branch id!");
	const [diffMarks, setDiffMarksSignal] = createSignal<DiffMark[]>(
		opts.diffMarks ?? [],
	);

	const createYDoc = () => {
		const ydoc = new Y.Doc();
		ydoc.on("update", (update, origin) => {
			if (origin && origin.key === "server") return;

			api.client.send({
				type: "DocumentEdit",
				channel_id: currentChannelId(),
				branch_id: currentBranchId(),
				update: base64UrlEncode(update),
			});
		});
		return ydoc;
	};

	let ydoc = createYDoc();

	const onSync = ([msg]: [MessageSync, unknown]) => {
		if (msg.type === "DocumentEdit") {
			if (
				msg.channel_id === currentChannelId() &&
				msg.branch_id === currentBranchId()
			) {
				const update = msg.update instanceof Uint8Array
					? msg.update
					: base64UrlDecode(msg.update);
				Y.applyUpdate(ydoc, update, { key: "server" });
			}
		} else if (msg.type === "DocumentSubscribed") {
			if (
				msg.channel_id === currentChannelId() &&
				msg.branch_id === currentBranchId()
			) {
				setIsSubscribed(true);
			}
		}
	};

	api.events.on("sync", onSync);

	const createState = () => {
		let type = ydoc.get("doc", Y.XmlFragment);
		let mapping = initProseMirrorDoc(type, schema).mapping;

		let doc;
		if (opts.initialContent) {
			const div = document.createElement("div");
			div.innerHTML = md.parser(md.lexer(opts.initialContent));
			doc = DOMParser.fromSchema(schema).parse(div);
		}

		let selection;
		if (doc && opts.initialSelection) {
			let pos = 1;
			if (opts.initialSelection === "end") {
				pos = doc.content.size - 1;
			}
			selection = TextSelection.create(doc, pos);
		}

		return EditorState.create({
			doc,
			selection,
			schema,
			plugins: [
				ySyncPlugin(type, { mapping }),
				cursorPlugin(
					api,
					currentChannelId(),
					currentBranchId(),
					isSubscribed,
					() => !(opts.diffMode?.() ?? false),
				),
				yUndoPlugin(),
				createDiffPlugin(() => diffMarks()),
				keymap({
					"Ctrl-z": undo,
					"Ctrl-Shift-z": redo,
					"Ctrl-y": redo,
					"Ctrl-b": createWrapCommand("**"),
					"Ctrl-i": createWrapCommand("*"),
					"Ctrl-`": createWrapCommand("`"),
					"Ctrl-m": (_state) => {
						return false;
					},
					"Shift-Enter": (state, dispatch) => {
						dispatch?.(state.tr.insertText("\n"));
						return true;
					},
					"Enter": (state, dispatch) => {
						return createListContinueCommand()(state, dispatch);
					},
					"Backspace": (state, dispatch) => {
						const sel = state.tr.selection;
						if (sel.empty) {
							const pos = sel.$anchor.pos - 1;
							if (pos >= 0) {
								dispatch?.(state.tr.deleteRange(pos, pos + 1));
							}
						} else {
							dispatch?.(state.tr.deleteSelection());
						}
						return true;
					},
					...opts.keymap,
				}),
				toolbarPlugin,
			],
		});
	};

	const editor = createBaseEditor({
		schema,
		createState: () => createState(),
		nodeViews: (view) => ({
			// TODO: use actual types
			mention: (node: any) => {
				const dom = document.createElement("span");
				dom.classList.add("mention");
				let dispose: () => void;
				if (opts.mentionRenderer) {
					opts.mentionRenderer(dom, node.attrs.user);
				} else {
					dispose = render(
						() => (
							<UserMention
								api={api}
								userId={node.attrs.user}
								channelId={currentChannelId()}
							/>
						),
						dom,
					);
				}
				return {
					dom,
					destroy: () => {
						dispose?.();
					},
				};
			},
			mentionChannel: (node: any) => {
				const dom = document.createElement("span");
				dom.classList.add("mention");
				let dispose: () => void;
				if (opts.mentionChannelRenderer) {
					opts.mentionChannelRenderer(dom, node.attrs.channel);
				} else {
					dispose = render(
						() => (
							<ChannelMention
								api={api}
								channelId={node.attrs.channel}
							/>
						),
						dom,
					);
				}
				return {
					dom,
					destroy: () => {
						dispose?.();
					},
				};
			},
			mentionRole: (node: any) => {
				const dom = document.createElement("span");
				dom.classList.add("mention");
				const dispose = render(
					() => (
						<RoleMention
							api={api}
							roleId={node.attrs.role}
						/>
					),
					dom,
				);
				return {
					dom,
					destroy: () => {
						dispose?.();
					},
				};
			},
			emoji: (node: any) => {
				const dom = document.createElement("span");
				dom.classList.add("mention");
				const dispose = render(
					() => (
						<Emoji
							id={node.attrs.id}
							name={node.attrs.name}
						/>
					),
					dom,
				);
				return {
					dom,
					destroy: () => {
						dispose?.();
					},
				};
			},
		}),
	});

	const subscribe = (channelId: string, branchId: string) => {
		// don't resubscribe if nothing changed
		if (
			currentChannelId() === channelId &&
			currentBranchId() === branchId
		) {
			return;
		}

		// reset document state
		ydoc = createYDoc();
		editor.setState(createState());

		setCurrentChannelId(channelId);
		setCurrentBranchId(branchId);
		setIsSubscribed(false);

		api.client.send({
			type: "DocumentSubscribe",
			channel_id: channelId,
			branch_id: branchId,
			state_vector: base64UrlEncode(Y.encodeStateVector(ydoc)),
		});
	};

	const setDiffMarks = (marks: DiffMark[]) => {
		setDiffMarksSignal(marks);
		if (editor.view) {
			const tr = editor.view.state.tr;
			tr.setMeta(diffPluginKey, { marks });
			editor.view.dispatch(tr);
		}
	};

	// Create a plain state without Yjs sync (for viewing historical revisions)
	const createReadonlyState = (content: string) => {
		let doc;
		if (content) {
			const div = document.createElement("div");
			const html = md.parser(md.lexer(content));
			div.innerHTML = html;
			doc = DOMParser.fromSchema(schema).parse(div);
		}

		return EditorState.create({
			doc,
			schema,
			plugins: [
				createDiffPlugin(() => diffMarks()),
			],
		});
	};

	// Create a plain state from HTML (for viewing historical revisions from serdoc)
	const createReadonlyStateFromHtml = (html: string) => {
		let doc;
		if (html) {
			const div = document.createElement("div");
			div.innerHTML = html;
			doc = DOMParser.fromSchema(schema).parse(div);
		}

		return EditorState.create({
			doc,
			schema,
			plugins: [
				createDiffPlugin(() => diffMarks()),
			],
		});
	};

	return {
		...editor,
		subscribe,
		isSubscribed,
		setDiffMarks,
		createReadonlyState,
		createReadonlyStateFromHtml,
		get view() {
			return editor.view;
		},
		View: (props: EditorViewProps) => {
			return (
				<editor.View
					{...props}
					disabled={props.disabled ?? (opts.diffMode?.() ?? false)}
				/>
			);
		},
	};
};
