import { For, Match, Show, Switch } from "solid-js/web";
import { type Attachment, useCtx } from "./context.ts";
import type { ThreadT } from "./types.ts";
import Editor, { createEditorState } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";
import { useApi } from "./api.tsx";
import { leading, throttle } from "@solid-primitives/scheduled";
import { createEffect, createSignal, on, onCleanup } from "solid-js";
import { getMessageContent, getMessageOverrideName } from "./util.tsx";

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
	// const typing = () => api.typing.get(props.thread.id);

	const sendTyping = leading(throttle, () => {
		ctx.client.http.POST("/api/v1/thread/{thread_id}/typing", {
			params: {
				path: { thread_id: props.thread.id },
			},
		});
	}, 8000);

	const getName = (user_id: string) => {
		const user = api.users.fetch(() => user_id);
		const member = api.room_members.fetch(
			() => props.thread.room_id,
			() => user_id,
		);

		const m = member();
		return (m?.membership === "Join" && m.override_name) ?? user()?.name;
	};

	const getNameNullable = (user_id?: string) => {
		if (user_id) return getName(user_id);
	};

	const getTyping = () => {
		// TODO: fix types here
		const fmt = new (Intl as any).ListFormat();
		const user_id = api.users.cache.get("@self")?.id;
		const user_ids = [...api.typing.get(props.thread.id)?.values() ?? []]
			.filter((i) => i !== user_id);
		return fmt.format(user_ids.map((i) => getName(i) ?? "someone"));
	};

	const [editorState, setEditorState] = createSignal();

	createEffect(on(() => props.thread.id, (tid) => {
		let state = ctx.thread_editor_state.get(tid);
		if (!state) {
			state = createEditorState(
				(text) => {
					ctx.dispatch({ do: "thread.send", thread_id: props.thread.id, text });
				},
				(has_content) => {
					if (has_content) {
						sendTyping();
					} else {
						sendTyping.clear();
					}
				},
			);
			ctx.thread_editor_state.set(props.thread.id, state);
		}
		console.log("editor: set state");
		setEditorState(state);
	}));

	return (
		<div class="input" style="position:relative">
			<div class="typing">
				<Show when={getTyping().length}>
					typing: {getTyping()}
				</Show>
			</div>
			<Show when={atts()?.length}>
				<div class="attachments">
					<header>attachments</header>
					<ul>
						<For each={atts()}>
							{(att) => <RenderUploadItem thread={props.thread} att={att} />}
						</For>
					</ul>
				</div>
			</Show>
			<Show when={reply_id()}>
				<div class="reply">
					<button
						class="cancel"
						onClick={() => ctx.thread_reply_id.delete(props.thread.id)}
					>
						cancel
					</button>
					<div class="info">
						replying to {getMessageOverrideName(reply()) ??
							getNameNullable(reply()?.author_id)}: {getMessageContent(reply())}
					</div>
				</div>
			</Show>
			<div class="text">
				<label class="upload">
					upload file
					<input
						multiple
						type="file"
						onInput={uploadFile}
						value="upload file"
					/>
				</label>
				<Editor
					thread_id={props.thread.id}
					state={editorState()}
					onUpload={handleUpload}
					placeholder="send a message..."
				/>
			</div>
		</div>
	);
}

function RenderUploadItem(props: { thread: ThreadT; att: Attachment }) {
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
		ctx.dispatch({ do: "upload.cancel", local_id, thread_id: props.thread.id });
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
