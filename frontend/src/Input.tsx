import { For, Match, Show, Switch } from "solid-js/web";
import { type Attachment, useCtx } from "./context.ts";
import type { MessageT, ThreadT } from "./types.ts";
import { createEditor } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";
import { useApi } from "./api.tsx";
import { leading, throttle } from "@solid-primitives/scheduled";
import { createEffect, createMemo, onCleanup, onMount } from "solid-js";
import { getMessageOverrideName } from "./util.tsx";
import { EditorState } from "prosemirror-state";
import { usePermissions } from "./hooks/usePermissions.ts";
import cancelIc from "./assets/x.png";
import { createTooltip } from "./Tooltip.tsx";

type InputProps = {
	thread: ThreadT;
};

export function Input(props: InputProps) {
	const ctx = useCtx();
	const api = useApi();
	const reply_id = () => ctx.thread_reply_id.get(props.thread.id);
	const reply = () => api.messages.cache.get(reply_id()!);

	function handleUpload(file: File) {
		console.log(file);
		const local_id = uuidv7();
		ctx.dispatch({
			do: "upload.init",
			file,
			local_id,
			thread_id: props.thread.id,
		});
	}

	function uploadFile(e: InputEvent) {
		const target = e.target! as HTMLInputElement;
		const files = Array.from(target.files!);
		for (const file of files) {
			handleUpload(file);
		}
	}

	const atts = () => ctx.thread_attachments.get(props.thread.id);

	const sendTyping = leading(throttle, () => {
		ctx.client.http.POST("/api/v1/thread/{thread_id}/typing", {
			params: {
				path: { thread_id: props.thread.id },
			},
		});
	}, 8000);

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
	const fmt = new (Intl as any).ListFormat();

	const typingUsers = createMemo(() => {
		const user_id = api.users.cache.get("@self")?.id;
		const user_ids = [...api.typing.get(props.thread.id)?.values() ?? []]
			.filter((i) => i !== user_id);
		return user_ids;
	});

	const onSubmit = (text: string) => {
		ctx.dispatch({ do: "thread.send", thread_id: props.thread.id, text });
	};

	const onChange = (state: EditorState) => {
		ctx.thread_editor_state.set(props.thread.id, state);
		const hasContent = state.doc.textContent.trim().length > 0;
		if (hasContent) {
			sendTyping();
		} else {
			sendTyping.clear();
		}
	};

	const editor = createEditor({
		keymap: {
			ArrowUp: (state) => {
				if (state.doc.textContent.length > 0) {
					return false; // not empty, do default behavior
				}

				const ranges = api.messages.cacheRanges.get(props.thread.id);
				if (!ranges) return false;

				const self_id = api.users.cache.get("@self")?.id;
				if (!self_id) return false;

				for (let i = ranges.live.items.length - 1; i >= 0; i--) {
					const msg = ranges.live.items[i];
					if (msg.author_id === self_id && msg.type === "DefaultMarkdown") {
						ctx.editingMessage.set(props.thread.id, msg.id);
						return true; // handled
					}
				}

				return false;
			},
		},
	});

	onMount(() => {
		ctx.thread_input_focus.set(props.thread.id, () => editor.focus());
		onCleanup(() => {
			ctx.thread_input_focus.delete(props.thread.id);
		});
	});

	createEffect(() => {
		const state = ctx.thread_editor_state.get(props.thread.id);
		editor.setState(state);
		editor.focus();
	});

	const perms = usePermissions(
		() => api.users.cache.get("@self")?.id ?? "",
		() => props.thread.room_id ?? undefined,
		() => props.thread.id,
	);

	const locked = () => {
		return !perms.has("MessageCreate") ||
			(props.thread.locked && !perms.has("ThreadLock"));
	};

	return (
		<div
			class="message-input"
			classList={{
				locked: locked(),
			}}
		>
			<Show when={typingUsers().length}>
				<div class="typing">
					{fmt.format(typingUsers().map((i) => getName(i) || "someone"))}{" "}
					{typingUsers().length === 1 ? "is" : "are"} typing
				</div>
			</Show>
			<Show when={atts()?.length}>
				<div class="attachments">
					<header>
						{atts()?.length}{" "}
						{atts()?.length === 1 ? "attachment" : "attachments"}
					</header>
					<ul>
						<For each={atts()}>
							{(att) => (
								<RenderUploadItem thread_id={props.thread.id} att={att} />
							)}
						</For>
					</ul>
				</div>
			</Show>
			<Show when={reply()}>
				<InputReply thread={props.thread} reply={reply()!} />
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
					placeholder={locked()
						? "you cannot send mesages here"
						: `send a message...`}
					disabled={locked()}
				/>
			</div>
		</div>
	);
}

export function RenderUploadItem(
	props: { thread_id: string; att: Attachment },
) {
	const ctx = useCtx();
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
		ctx.dispatch({ do: "upload.cancel", local_id, thread_id: props.thread_id });
	}

	function pause() {
		ctx.dispatch({
			do: "upload.pause",
			local_id: props.att.local_id,
		});
	}

	function resume() {
		ctx.dispatch({
			do: "upload.resume",
			local_id: props.att.local_id,
		});
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
								🗑️
							</button>
						</menu>
					</div>
				</div>
			</div>
		</>
	);
}

const InputReply = (props: { thread: ThreadT; reply: MessageT }) => {
	const ctx = useCtx();
	const api = useApi();
	const tip = createTooltip({ tip: () => "remove reply" });
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
				onClick={() => ctx.thread_reply_id.delete(props.thread.id)}
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
