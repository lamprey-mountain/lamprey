import { leading, throttle } from "@solid-primitives/scheduled";
import type { EditorState } from "prosemirror-state";
import type { Channel } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	onCleanup,
	onMount,
} from "solid-js";
import { For, Match, render, Show, Switch } from "solid-js/web";
import { uuidv7 } from "uuidv7";
import {
	useApi2,
	useChannels2,
	useMessages2,
	useRoomMembers2,
	useUsers2,
} from "@/api";
import icDelete from "../../../assets/delete.png";
import icEdit from "../../../assets/edit.png";
import cancelIc from "../../../assets/x.png";
import { EmojiButton } from "../../../atoms/EmojiButton.tsx";
import { createTooltip } from "../../../atoms/Tooltip.tsx";
import { useChannel } from "../../../channelctx.tsx";
import { type Attachment, useCtx } from "../../../context.ts";
import { useAutocomplete } from "../../../contexts/autocomplete";
import { useCurrentUser } from "../../../contexts/currentUser.tsx";
import { useFormattingToolbar } from "../../../contexts/formatting-toolbar";
import { useModals } from "../../../contexts/modal.tsx";
import { useUploads } from "../../../contexts/uploads.tsx";
import { useMessageSubmit } from "../../../hooks/useMessageSubmit.ts";
import { usePermissions } from "../../../hooks/usePermissions.ts";
import { getThumbFromId } from "../../../media/util.tsx";
import type { MessageT, ThreadT } from "../../../types.ts";
import { getMessageOverrideName } from "../../../utils/general";
import { createEditor } from "../editor/Editor.tsx";

type InputProps = {
	channel: Channel;
};

export function Input(props: InputProps) {
	const channels2 = useChannels2();
	const messagesService = useMessages2();
	const users2 = useUsers2();
	const roomMembers2 = useRoomMembers2();
	const store = useApi2();
	const [ch, chUpdate] = useChannel()!;
	const submit = useMessageSubmit(props.channel.id);
	const reply_id = () => ch.reply_id;
	const reply = () => messagesService.cache.get(reply_id()!);
	const uploads = useUploads();
	const currentUser = useCurrentUser();

	function handleUpload(file: File) {
		console.log(file);
		const local_id = uuidv7();
		uploads.init(local_id, props.channel.id, file);
	}

	function uploadFile(e: InputEvent) {
		const target = e.target! as HTMLInputElement;
		const files = Array.from(target.files!);
		for (const file of files) {
			handleUpload(file);
		}
	}

	const atts = () => ch.attachments;

	const sendTyping = leading(
		throttle,
		() => {
			channels2.typing(props.channel.id);
		},
		8000,
	);

	const getName = (user_id: string) => {
		const user = users2.cache.get(user_id);
		const room_id = props.channel.room_id;
		if (!room_id) {
			return user?.name;
		}

		const member = roomMembers2.cache.get(`${room_id}:${user_id}`);
		const m = member;
		return (
			(((m as any)?.membership as any) === "Join" && m?.override_name) ??
			user?.name
		);
	};
	const fmt = new (Intl as any).ListFormat();

	const typingUsers = createMemo(() => {
		const user_id = currentUser()?.id;
		const user_ids = [
			...(store.typing.get(props.channel.id)?.values() ?? []),
		].filter((i) => i !== user_id);
		return user_ids;
	});

	let slowmodeRef!: HTMLDivElement;

	const slowmodeShake = () => {
		const SCALEX = 1.5;
		const SCALEY = 0.4;
		const FRAMES = 10;
		const rnd = (sx: number, sy: number) =>
			`${Math.random() * sx - sx / 2}px ${Math.random() * sy - sy / 2}px`;
		const translations = new Array(FRAMES)
			.fill(0)
			.map((_, i) => rnd(i * SCALEX, i * SCALEY))
			.reverse();
		const reduceMotion = false; // TODO
		slowmodeRef.animate(
			{
				translate: reduceMotion ? [] : translations,
				color: ["red", ""],
			},
			{ duration: 200, easing: "linear" },
		);
	};

	const onSubmit = (text: string) => {
		if (slowmodeActive()) {
			slowmodeShake();
			return false;
		}
		return submit(text, bypassSlowmode());
	};

	const onEmojiPick = (emoji: string, _keepOpen?: boolean) => {
		const editorState = ch.editor_state;
		if (editorState) {
			const { from, to } = editorState.selection;
			const customMatch = emoji.match(/^<(a?):([^:]+):([^>]+)>$/);
			let tr;
			if (customMatch) {
				const animated = customMatch[1] === "a";
				const name = customMatch[2];
				const id = customMatch[3];
				tr = editorState.tr.replaceWith(
					from,
					to,
					editor.schema.nodes.emojiCustom.create({ id, name, animated }),
				);
			} else {
				tr = editorState.tr.insertText(emoji, from, to);
			}
			const newState = editorState.apply(tr);
			chUpdate("editor_state", newState);
		}
	};

	const onChange = (state: EditorState) => {
		chUpdate("editor_state", state);
		const hasContent = state.doc.textContent.trim().length > 0;
		if (hasContent) {
			sendTyping();
		} else {
			sendTyping.clear();
		}
	};

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const editor = createEditor({
		channelId: () => props.channel.id ?? "",
		roomId: () => props.channel.room_id ?? "",
		toolbar,
		autocomplete,
		keymap: {
			ArrowUp: (state) => {
				if (state.doc.textContent.length > 0) {
					return false; // not empty, do default behavior
				}

				const ranges = messagesService._ranges.get(props.channel.id);
				if (!ranges) return false;

				const self_id = currentUser()?.id;
				if (!self_id) return false;

				for (let i = ranges.live.items.length - 1; i >= 0; i--) {
					const msg = ranges.live.items[i];
					if (
						msg.author_id === self_id &&
						((msg as any).type as any) === "DefaultMarkdown"
					) {
						chUpdate("editingMessage", {
							message_id: msg.id,
							selection: "end",
						});
						return true; // handled
					}
				}

				return false;
			},
		},
	});

	onMount(() => {
		chUpdate("input_focus", () => editor.focus());
		onCleanup(() => {
			chUpdate("input_focus", undefined);
		});
	});

	createEffect(() => {
		const state = ch.editor_state;
		editor.setState(state);
		editor.focus();
	});

	createEffect(() => {
		const expireAt = props.channel.slowmode_message_expire_at;
		if (expireAt) {
			const currentExpireAt = ch.slowmode_expire_at;
			const newExpireAt = new Date(expireAt);
			if (
				!currentExpireAt ||
				currentExpireAt.getTime() !== newExpireAt.getTime()
			) {
				chUpdate("slowmode_expire_at", newExpireAt);
			}
		}
	});

	const perms = usePermissions(
		() => currentUser()?.id ?? "",
		() => props.channel.room_id ?? undefined,
		() => props.channel.id,
	);

	const locked = () => {
		return (
			!perms.has("MessageCreate") ||
			(props.channel.locked && !perms.has("ThreadManage"))
		);
	};

	const bypassSlowmode = (): boolean =>
		perms.has("ChannelManage") ||
		perms.has("ThreadManage") ||
		perms.has("MemberTimeout");

	const [remainingTime, setRemainingTime] = createSignal(0);
	const slowmodeRemaining = () => remainingTime();
	const slowmodeActive = () => slowmodeRemaining() > 0;

	createEffect(() => {
		const expireAt = ch.slowmode_expire_at;
		if (expireAt) {
			const updateTimer = () => {
				const now = new Date().getTime();
				const remaining = expireAt.getTime() - now;
				setRemainingTime(Math.max(0, remaining));
			};

			updateTimer();
			const interval = setInterval(updateTimer, 1000);
			onCleanup(() => clearInterval(interval));
		} else {
			setRemainingTime(0);
		}
	});

	const slowmodeFormatted = () => {
		const remainingMs = slowmodeRemaining();
		if (remainingMs <= 0 || bypassSlowmode()) {
			const channelSlowmode = props.channel.slowmode_message;
			if (channelSlowmode) {
				const mins = Math.floor(channelSlowmode / 60);
				const secs = channelSlowmode % 60;
				const time =
					mins === 0
						? `slowmode set to ${secs}s`
						: `slowmode set to ${mins}m${secs.toString().padStart(2, "0")}s`;
				return `slowmode set to ${time}${
					bypassSlowmode() ? " (bypassed)" : ""
				}`;
			} else return "no slowmode";
		}
		const seconds = Math.ceil(remainingMs / 1000);
		const mins = Math.floor(seconds / 60);
		const secs = seconds % 60;
		return `${mins}:${secs.toString().padStart(2, "0")}`;
	};

	const anchor =
		(): import("../../../api/services/MessagesService").MessageListAnchor => {
			const a = ch.anchor;
			const r = ch.read_marker_id;
			if (a) return a;
			if (r) return { type: "context", limit: 50, message_id: r };
			return { type: "backwards", limit: 50 };
		};
	const messages = messagesService.useList(() => props.channel.id, anchor);

	const jumpToLatest = () => {
		// messages are approx. 20 px high, show 3 pages of messages
		const SLICE_LEN = Math.max(Math.ceil(globalThis.innerHeight / 20) * 3, 50);

		chUpdate("anchor", {
			type: "backwards",
			limit: SLICE_LEN,
		});
	};

	const jumpToReplySource = () => {
		const source = ch.reply_jump_source;
		if (source) {
			chUpdate("anchor", {
				type: "context",
				limit: 50,
				message_id: source,
			});
			chUpdate("highlight", source);
			chUpdate("reply_jump_source", undefined);
		}
	};

	return (
		<div
			class="message-input"
			classList={{
				locked: locked() ?? false,
			}}
		>
			<Show when={atts()?.length}>
				<div class="attachments">
					<header>
						{atts()?.length}{" "}
						{atts()?.length === 1 ? "attachment" : "attachments"}
					</header>
					<ul>
						<For each={atts()}>
							{(att) => (
								<RenderUploadItem thread_id={props.channel.id} att={att} />
							)}
						</For>
					</ul>
				</div>
			</Show>
			<Switch>
				<Match when={messages()?.has_forward}>
					<button class="jump-to-latest" onClick={jumpToLatest}>
						you are viewing older messages &bull; click to jump to present
					</button>
				</Match>
				<Match when={ch.reply_jump_source}>
					<button class="jump-to-latest" onClick={jumpToReplySource}>
						you are viewing a reply &bull; click to jump to source
					</button>
				</Match>
			</Switch>
			<Show when={reply()}>
				<InputReply thread={props.channel} reply={reply()!} />
			</Show>
			<div class="text">
				<label class="upload">
					+
					<input
						multiple
						type="file"
						onInput={uploadFile}
						value="upload file"
						disabled={locked() ?? false}
					/>
				</label>
				<editor.View
					onSubmit={onSubmit}
					onChange={onChange}
					onUpload={handleUpload}
					channelId={props.channel.id}
					placeholder={
						(locked() ?? false)
							? "you cannot send messages here"
							: `send a message...`
					}
					disabled={locked() ?? false}
				/>
				<EmojiButton picked={onEmojiPick} />
			</div>
			<footer>
				<Show when={typingUsers().length}>
					<div class="typing">
						{/* TODO: bold names */}
						{fmt.format(typingUsers().map((i) => getName(i) || "someone"))}{" "}
						{typingUsers().length === 1 ? "is" : "are"} typing
					</div>
				</Show>
				<div style="flex:1"></div>

				<Show when={props.channel.slowmode_message || slowmodeActive()}>
					{/* TODO: icon for slowmode indicator*/}
					<div class="slowmode" ref={slowmodeRef}>
						{slowmodeFormatted()}
					</div>
				</Show>
			</footer>
		</div>
	);
}

export function RenderUploadItem(props: {
	thread_id: string;
	att: Attachment;
}) {
	const ctx = useCtx();
	const uploads = useUploads();
	const [, modalCtl] = useModals();
	const thumbUrl =
		props.att.status === "uploaded"
			? getThumbFromId(props.att.media.id, 64)
			: URL.createObjectURL(props.att.file);

	if (props.att.status !== "uploaded") {
		onCleanup(() => {
			URL.revokeObjectURL(thumbUrl);
		});
	}

	function renderInfo(att: Attachment) {
		if (att.status === "uploading") {
			if (att.progress === 1) {
				return `processing`;
			} else {
				const percent = (att.progress * 100).toFixed(2);
				return `${percent}%`;
			}
		} else {
			return "";
			// return <AttachmentView media={att.media} size={64} />;
		}
	}

	function getProgress(att: Attachment) {
		if (att.status === "uploading") {
			return att.progress;
		} else {
			return 1;
		}
	}

	function removeAttachment(local_id: string) {
		uploads.cancel(local_id, props.thread_id);
	}

	function pause() {
		uploads.pause(props.att.local_id);
	}

	function resume() {
		uploads.resume(props.att.local_id);
	}

	const filename =
		props.att.status === "uploaded"
			? props.att.media.filename
			: (props.att.filename ?? props.att.file.name);

	return (
		<>
			<div class="upload-item">
				<div
					class="thumb"
					style={{ "background-image": `url(${thumbUrl})` }}
				></div>
				<div class="info">
					<svg class="progress" viewBox="0 0 1 1" preserveAspectRatio="none">
						<rect class="bar" height="1" width={getProgress(props.att)}></rect>
					</svg>
					<div style="display: flex">
						<div style="flex: 1;white-space:nowrap;text-overflow:ellipsis;overflow:hidden">
							{filename}
							<span style="color:#888;margin-left:.5ex">
								{renderInfo(props.att)}
							</span>
						</div>
						<menu style="display:flex">
							<Switch>
								<Match
									when={props.att.status === "uploading" && props.att.paused}
								>
									<button onClick={resume}>⬆️</button>
								</Match>
								<Match when={props.att.status === "uploading"}>
									<button onClick={pause}>⏸️</button>
								</Match>
							</Switch>
							<Show when={props.att.status === "uploaded"}>
								<button
									onClick={() =>
										modalCtl.open({
											type: "attachment",
											channel_id: props.thread_id,
											local_id: props.att.local_id,
										})
									}
								>
									<img class="icon" src={icEdit} />
								</button>
							</Show>
							<button onClick={() => removeAttachment(props.att.local_id)}>
								<img class="icon" src={icDelete} />
							</button>
						</menu>
					</div>
				</div>
			</div>
		</>
	);
}

const InputReply = (props: { thread: ThreadT; reply: MessageT }) => {
	const users2 = useUsers2();
	const roomMembers2 = useRoomMembers2();
	const tip = createTooltip({ tip: () => "remove reply" });
	const [_ch, chUpdate] = useChannel()!;
	const getName = (user_id: string) => {
		const user = users2.cache.get(user_id);
		const room_id = props.thread.room_id;
		if (!room_id) {
			return user?.name;
		}
		const member = roomMembers2.cache.get(`${room_id}:${user_id}`);

		const m = member;
		return (
			(((m as any)?.membership as any) === "Join" && m?.override_name) ??
			user?.name
		);
	};

	const getNameNullable = (user_id?: string) => {
		if (user_id) return getName(user_id);
	};

	return (
		<div class="reply">
			<button
				class="cancel"
				onClick={() => chUpdate("reply_id", undefined)}
				ref={tip.content}
			>
				<img class="icon" src={cancelIc} />
			</button>
			<div class="info">
				replying to{" "}
				<b>
					{getMessageOverrideName(props.reply) ??
						getNameNullable(props.reply?.author_id)}
				</b>
			</div>
		</div>
	);
};
