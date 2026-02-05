import * as Y from "yjs";
import { type Command, EditorState, TextSelection } from "prosemirror-state";
import { Decoration, DecorationSet, EditorView } from "prosemirror-view";
import { DOMParser, Schema } from "prosemirror-model";
import {
	initProseMirrorDoc,
	redo,
	undo,
	ySyncPlugin,
	yUndoPlugin,
} from "y-prosemirror";
import { keymap } from "prosemirror-keymap";
import { createEffect, onCleanup, onMount } from "solid-js";
import { render } from "solid-js/web";
import { getEmojiUrl } from "./media/util.tsx";
import { initTurndownService } from "./turndown.ts";
import { decorate, md } from "./markdown.tsx";
import { useCtx } from "./context";
import {
	base64UrlDecode,
	base64UrlEncode,
	createWrapCommand,
	handleAutocomplete,
} from "./editor-utils.ts";
import { type Api, useApi } from "./api.tsx";
import { MessageSync } from "sdk";
import { cursorPlugin } from "./editor-cursors.ts";

const turndown = initTurndownService();

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

const schema = new Schema({
	nodes: {
		doc: {
			content: "block+",
		},
		paragraph: {
			content: "inline*",
			group: "block",
			whitespace: "pre",
			toDOM: () => ["p", 0],
			parseDOM: ["p", "x-html-import"].map((tag) => ({
				tag,
				preserveWhitespace: "full",
			})),
		},
		// maybe have special purpose blocks instead of pure markdown (markdown with incremental rich text)
		// blockquote: {},
		// table: {},
		// codeblock: {},
		// details: {},
		// media: {},
		mention: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				user: {},
			},
			leafText(node) {
				return `<@${node.attrs.user}>`;
			},
			toDOM: (
				n,
			) => ["span", { "data-user-id": n.attrs.user, "class": "mention" }],
			parseDOM: [{
				tag: "span.mention[data-user-id]",
				getAttrs: (el) => ({ user: (el as HTMLElement).dataset.userId }),
			}],
		},
		mentionChannel: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				channel: {},
			},
			leafText(node) {
				return `<#${node.attrs.channel}>`;
			},
			toDOM: (
				n,
			) => ["span", { "data-channel-id": n.attrs.channel, "class": "mention" }],
			parseDOM: [{
				tag: "span.mention[data-channel-id]",
				getAttrs: (el) => ({ channel: (el as HTMLElement).dataset.channelId }),
			}],
		},
		mentionRole: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				role: {},
			},
			leafText(node) {
				return `<@&${node.attrs.role}>`;
			},
			toDOM: (
				n,
			) => ["span", { "data-role-id": n.attrs.role, "class": "mention" }],
			parseDOM: [{
				tag: "span.mention[data-role-id]",
				getAttrs: (el) => ({ role: (el as HTMLElement).dataset.roleId }),
			}],
		},
		emoji: {
			group: "inline",
			atom: true,
			inline: true,
			selectable: false,
			attrs: {
				id: {},
				name: {},
			},
			leafText(node) {
				return `<:${node.attrs.name}:${node.attrs.id}>`;
			},
			toDOM: (
				n,
			) => ["span", {
				"data-emoji-id": n.attrs.id,
				"data-emoji-name": n.attrs.name,
			}, `:${n.attrs.name}:`],
			parseDOM: [{
				tag: "span[data-emoji-id][data-emoji-name]",
				getAttrs: (el) => ({
					id: (el as HTMLElement).dataset.emojiId,
					name: (el as HTMLElement).dataset.emojiName,
				}),
			}],
		},
		text: {
			group: "inline",
			inline: true,
		},
	},
});

type EditorProps = {
	initialContent?: string;
	keymap?: { [key: string]: Command };
	initialSelection?: "start" | "end";
	mentionRenderer?: (node: HTMLElement, userId: string) => void;
	mentionChannelRenderer?: (node: HTMLElement, channelId: string) => void;
};

type EditorViewProps = {
	placeholder?: string;
	disabled?: boolean;
	onUpload?: (file: File) => void;
	onSubmit: (text: string) => boolean;
	onChange?: (state: EditorState) => void;
	channelId?: string; // Needed for autocomplete
	submitOnEnter?: boolean;
};

export const createEditor = (
	opts: EditorProps,
	channelId: string,
	branchId: string,
) => {
	const ctx = useCtx();
	const api = useApi();

	const ydoc = new Y.Doc();
	const type = ydoc.get("prosemirror", Y.XmlFragment);
	const { doc, mapping } = initProseMirrorDoc(type, schema);

	const onSync = ([msg]: [MessageSync, unknown]) => {
		if (msg.type === "DocumentEdit") {
			if (msg.channel_id === channelId && msg.branch_id === branchId) {
				const update = base64UrlDecode(msg.update);
				Y.applyUpdate(ydoc, update, { key: "server" });
			}
		}
	};

	api.events.on("sync", onSync);

	const subscribe = () => {
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

	let editorRef!: HTMLDivElement;
	let view: EditorView | undefined;
	let onSubmit!: (content: string) => boolean | undefined;
	let submitOnEnter = true;

	const submitCommand: Command = (state, dispatch) => {
		const shouldClear = onSubmit?.(state.doc.textContent.trim());
		if (shouldClear) {
			dispatch?.(state.tr.deleteRange(0, state.doc.nodeSize - 2));
		}
		return true;
	};

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
				cursorPlugin(api, channelId, branchId),
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
					"Ctrl-Enter": submitCommand,
					"Enter": (state, dispatch) => {
						if (submitOnEnter) {
							return submitCommand(state, dispatch);
						}
						dispatch?.(state.tr.insertText("\n"));
						return true;
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

	return {
		setState(state?: EditorState) {
			view?.updateState(state ?? createState());
		},
		focus() {
			view?.focus();
		},
		View(props: EditorViewProps) {
			createEffect(() => {
				onSubmit = props.onSubmit;
				submitOnEnter = props.submitOnEnter ?? true;
			});

			onMount(() => {
				const ctx = useCtx(); // Access context inside mount since we're in a Solid component

				view = new EditorView(editorRef!, {
					domParser: DOMParser.fromSchema(schema),
					state: createState(),
					decorations(state) {
						return decorate(state, props.placeholder);
					},
					nodeViews: {
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
					},
					handlePaste(view, event, slice) {
						const files = Array.from(event.clipboardData?.files ?? []);
						if (files.length) {
							for (const file of files) props.onUpload?.(file);
							return true;
						}
						const str = slice.content.textBetween(0, slice.size);
						const tr = view.state.tr;
						if (
							/^(https?:\/\/|mailto:)\S+$/i.test(str) && !tr.selection.empty
						) {
							tr.insertText("[", tr.selection.from);
							tr.insertText(`](${str})`, tr.selection.to);
							tr.setSelection(TextSelection.create(tr.doc, tr.selection.to));
							view.dispatch(
								tr.scrollIntoView().setMeta("paste", true).setMeta(
									"uiEvent",
									"paste",
								),
							);
						} else {
							const textToParse = slice.content.textBetween(
								0,
								slice.content.size,
								"\n",
							);
							const div = document.createElement("div");
							div.innerHTML = md.parser(md.lexer(textToParse));
							const newSlice = DOMParser.fromSchema(schema).parseSlice(div);
							view.dispatch(
								view.state.tr.replaceSelection(newSlice).scrollIntoView()
									.setMeta("paste", true),
							);
						}
						return true;
					},
					handleKeyDown(view, event) {
						return handleAutocomplete(
							view,
							event,
							ctx,
							schema,
							props.channelId || "",
						);
					},
					transformPastedHTML(html) {
						const markdown = turndown.turndown(html);
						const div = document.createElement("div");
						div.innerHTML = md.parser(md.lexer(markdown));
						return div.innerHTML;
					},
					editable: () => !(props.disabled ?? false),
					dispatchTransaction(tr) {
						const newState = view!.state.apply(tr);
						view!.updateState(newState);
						props.onChange?.(newState);
					},
				});

				view.focus();
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
