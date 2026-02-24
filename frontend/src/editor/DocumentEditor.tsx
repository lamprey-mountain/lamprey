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
import { md } from "../markdown.tsx";
import { useApi } from "../api.tsx";
import { MessageSync } from "sdk";
import { cursorPlugin } from "./editor-cursors.ts";
import { createEditor as createBaseEditor } from "./mod.tsx";
import { schema } from "./schema.ts";
import {
	base64UrlDecode,
	base64UrlEncode,
	createListContinueCommand,
	createWrapCommand,
} from "./editor-utils.ts";
import { type Api } from "../api.tsx";
import { Accessor, createEffect, createSignal, on } from "solid-js";

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
};

export const createEditor = (
	opts: EditorProps,
	channelId: string,
	branchId: string,
) => {
	const api = useApi();

	const [isSubscribed, setIsSubscribed] = createSignal(false);

	const ydoc = new Y.Doc();
	const type = ydoc.get("prosemirror", Y.XmlFragment);
	const { mapping } = initProseMirrorDoc(type, schema);

	const onSync = ([msg]: [MessageSync, unknown]) => {
		if (msg.type === "DocumentEdit") {
			if (msg.channel_id === channelId && msg.branch_id === branchId) {
				const update = base64UrlDecode(msg.update);
				Y.applyUpdate(ydoc, update, { key: "server" });
				setIsSubscribed(true);
			}
		}
	};

	api.events.on("sync", onSync);

	const subscribe = () => {
		setIsSubscribed(false);
		api.client.send({
			type: "DocumentSubscribe",
			channel_id: channelId,
			branch_id: branchId,
			state_vector: base64UrlEncode(Y.encodeStateVector(ydoc)),
		});
	};

	subscribe();

	ydoc.on("update", (update, origin) => {
		if (origin && origin.key === "server") return;

		api.client.send({
			type: "DocumentEdit",
			channel_id: channelId,
			branch_id: branchId,
			update: base64UrlEncode(update),
		});
	});

	const createState = () => {
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
				cursorPlugin(api, channelId, branchId, isSubscribed),
				yUndoPlugin(),
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
						// Handled by base but we can keep createListContinueCommand if we want it here
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
			],
		});
	};

	const editor = createBaseEditor({
		schema,
		createState: () => createState(),
		nodeViews: (view) => ({
			mention: (node) => {
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
								channelId={channelId}
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
			mentionChannel: (node) => {
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
			mentionRole: (node) => {
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
			emoji: (node) => {
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

	return {
		...editor,
		subscribe(newChannelId: string, newBranchId: string) {
			channelId = newChannelId;
			branchId = newBranchId;
			subscribe();
		},
	};
};
