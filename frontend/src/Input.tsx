import { For, Match, render, Show, Switch } from "solid-js/web";
import { type Attachment, useCtx } from "./context.ts";
import type { MessageT, ThreadT } from "./types.ts";
import { createEditor } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";
import { useApi } from "./api.tsx";
import { leading, throttle } from "@solid-primitives/scheduled";
import {
	createEffect,
	createMemo,
	createSignal,
	onCleanup,
	onMount,
} from "solid-js";
import { getMessageOverrideName } from "./util.tsx";
import { EditorState } from "prosemirror-state";
import { usePermissions } from "./hooks/usePermissions.ts";
import cancelIc from "./assets/x.png";
import { createTooltip } from "./Tooltip.tsx";
import { EmojiButton } from "./atoms/EmojiButton.tsx";
import { Channel } from "sdk";
import icDelete from "./assets/delete.png";
import { useChannel } from "./channelctx.tsx";
import { handleSubmit } from "./dispatch/submit.ts";
import { useUploads } from "./contexts/uploads.tsx";

type InputProps = {
	channel: Channel;
};

export function Input(props: InputProps) {
	const ctx = useCtx();
	const api = useApi();
	const [ch, chUpdate] = useChannel()!;
	const reply_id = () => ch.reply_id;
	const reply = () => api.messages.cache.get(reply_id()!);
	const uploads = useUploads();

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

	const sendTyping = leading(throttle, () => {
		api.channels.typing(props.channel.id);
	}, 8000);

	const getName = (user_id: string) => {
		const user = api.users.fetch(() => user_id);
		const room_id = props.channel.room_id;
		if (!room_id) {
			return user()?.name;
		}

		const member = api.room_members.fetch(
			() => room_id,
			() => user_id,
		);

		const m = member();
		return (m?.membership === "Join" && m.override_name) ?? user()?.name;
	};
	const fmt = new (Intl as any).ListFormat();

	const typingUsers = createMemo(() => {
		const user_id = api.users.cache.get("@self")?.id;
		const user_ids = [...api.typing.get(props.channel.id)?.values() ?? []]
			.filter((i) => i !== user_id);
		return user_ids;
	});

	let slowmodeRef!: HTMLDivElement;

	const slowmodeShake = () => {
		const SCALEX = 1.5;
		const SCALEY = .4;
		const FRAMES = 10;
		const rnd = (sx: number, sy: number) =>
			`${Math.random() * sx - sx / 2}px ${Math.random() * sy - sy / 2}px`;
		const translations = new Array(FRAMES).fill(0).map((_, i) =>
			rnd(i * SCALEX, i * SCALEY)
		).reverse();
		const reduceMotion = false; // TODO
		slowmodeRef.animate({
			translate: reduceMotion ? [] : translations,
			color: ["red", ""],
		}, { duration: 200, easing: "linear" });
	};

	const onSubmit = (text: string) => {
		if (slowmodeActive()) {
			slowmodeShake();
			return false;
		}
		handleSubmit(
			ctx,
			[ch, chUpdate],
			props.channel.id,
			text,
			null as any,
			api,
			undefined,
			bypassSlowmode(),
		);
		return true;
	};

	const onEmojiPick = (emoji: string) => {
		const editorState = ch.editor_state;
		if (editorState) {
			const { from, to } = editorState.selection;
			const tr = editorState.tr.insertText(emoji, from, to);
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

	function EditorUserMention(props: { id: string }) {
		const user = api.users.fetch(() => props.id);
		return <span class="mention-user">@{user()?.name ?? props.id}</span>;
	}

	function EditorChannelMention(props: { id: string }) {
		const channel = createMemo(() => api.channels.cache.get(props.id));
		return <span class="mention-channel">#{channel()?.name ?? props.id}</span>;
	}

	const editor = createEditor({
		keymap: {
			ArrowUp: (state) => {
				if (state.doc.textContent.length > 0) {
					return false; // not empty, do default behavior
				}

				const ranges = api.messages.cacheRanges.get(props.channel.id);
				if (!ranges) return false;

				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;

				for (let i = ranges.live.items.length - 1; i >= 0; i--) {
					const msg = ranges.live.items[i];
					if (msg.author_id === self_id && msg.type === "DefaultMarkdown") {
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
		mentionRenderer: (node, userId) => {
			render(() => <EditorUserMention id={userId} />, node);
		},
		mentionChannelRenderer: (node, channelId) => {
			render(() => <EditorChannelMention id={channelId} />, node);
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
				!currentExpireAt || currentExpireAt.getTime() !== newExpireAt.getTime()
			) {
				chUpdate("slowmode_expire_at", newExpireAt);
			}
		}
	});

	const perms = usePermissions(
		() => api.users.cache.get("@self")?.id ?? "",
		() => props.channel.room_id ?? undefined,
		() => props.channel.id,
	);

	const locked = () => {
		return !perms.has("MessageCreate") ||
			(props.channel.locked && !perms.has("ThreadLock"));
	};

	const bypassSlowmode = () =>
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
				const time = mins === 0
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

	const anchor = (): import("./api/messages.ts").MessageListAnchor => {
		const a = ch.anchor;
		const r = ch.read_marker_id;
		if (a) return a;
		if (r) return { type: "context", limit: 50, message_id: r };
		return { type: "backwards", limit: 50 };
	};
	const messages = api.messages.list(() => props.channel.id, anchor);

	const jumpToLatest = () => {
		// messages are approx. 20 px high, show 3 pages of messages
		const SLICE_LEN = Math.ceil(globalThis.innerHeight / 20) * 3;

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
				locked: locked(),
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
				<Match
					when={ch.reply_jump_source}
				>
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
						disabled={locked()}
					/>
				</label>
				<editor.View
					onSubmit={onSubmit}
					onChange={onChange}
					onUpload={handleUpload}
					channelId={props.channel.id}
					placeholder={locked()
						? "you cannot send messages here"
						: `send a message...`}
					disabled={locked()}
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

export function RenderUploadItem(
	props: { thread_id: string; att: Attachment },
) {
	const ctx = useCtx();
	const uploads = useUploads();
	const thumbUrl = URL.createObjectURL(props.att.file);
	onCleanup(() => {
		URL.revokeObjectURL(thumbUrl);
	});

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

	return (
		<>
			<div class="upload-item">
				<div class="thumb" style={{ "background-image": `url(${thumbUrl})` }}>
				</div>
				<div class="info">
					<svg class="progress" viewBox="0 0 1 1" preserveAspectRatio="none">
						<rect class="bar" height="1" width={getProgress(props.att)}></rect>
					</svg>
					<div style="display: flex">
						<div style="flex: 1;white-space:nowrap;text-overflow:ellipsis;overflow:hidden">
							{props.att.file.name}
							<span style="color:#888;margin-left:.5ex">
								{renderInfo(props.att)}
							</span>
						</div>
						<menu>
							<Switch>
								<Match
									when={props.att.status === "uploading" && props.att.paused}
								>
									<button onClick={resume}>
										⬆️
									</button>
								</Match>
								<Match when={props.att.status === "uploading"}>
									<button onClick={pause}>⏸️</button>
								</Match>
							</Switch>
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
	const api = useApi();
	const tip = createTooltip({ tip: () => "remove reply" });
	const [_ch, chUpdate] = useChannel()!;
	const getName = (user_id: string) => {
		const user = api.users.fetch(() => user_id);
		const room_id = props.thread.room_id;
		if (!room_id) {
			return user()?.name;
		}
		const member = api.room_members.fetch(
			() => room_id,
			() => user_id,
		);

		const m = member();
		return (m?.membership === "Join" && m.override_name) ?? user()?.name;
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
