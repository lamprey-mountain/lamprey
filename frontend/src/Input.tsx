import { For, Match, Show, Switch } from "solid-js/web";
import { Attachment, useCtx } from "./context.ts";
import { ThreadT } from "./types.ts";
import Editor, { createEditorState } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";
import { renderAttachment } from "./Message.tsx";
import { useApi } from "./api.tsx";
import { leading, throttle } from "@solid-primitives/scheduled";

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
		const files = target.files!;
		for (const file of files) {
			handleUpload(file);
		}
	}

	const atts = () => ctx.thread_attachments.get(props.thread.id);
	const typing = () => api.typing.get(props.thread.id);

	const sendTyping = leading(throttle, () => {
		ctx.client.http.PUT("/api/v1/thread/{thread_id}/typing", {
			params: {
				path: { thread_id: props.thread.id },
			},
		});
	}, 8000);

	const editor_state = () => {
		let state = ctx.thread_editor_state.get(props.thread.id)!;
		// if (!state) {
		if (true) {
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
		return state;
	};

	const getName = (user_id: string) => {
		const user = api.users.fetch(() => user_id);
		const member = api.room_members.fetch(
			() => props.thread.room_id,
			() => user_id,
		);

		const m = member();
		return m?.membership === "Join" && m.override_name || user()?.name;
	};

	const getTyping = () => {
		const fmt = new Intl.ListFormat();
		const user_id = api.users.cache.get("@self")?.id;
		const user_ids = [...api.typing.get(props.thread.id)?.values() ?? []]
			.filter((i) => i !== user_id);
		return fmt.format(user_ids.map((i) => getName(i) ?? "someone"));
	};

	return (
		<div class="input">
			<div class="typing">
				<Show when={getTyping().length}>
					typing: {getTyping()}
				</Show>
			</div>
			<Show when={atts()?.length}>
				<ul class="attachments">
					<For each={atts()}>
						{(att) => (
							<li>
								{renderAttachment2(props.thread, att)}
							</li>
						)}
					</For>
				</ul>
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
						replying to {reply()?.override_name ?? reply()?.author.name}:{" "}
						{reply()?.content}
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
					state={editor_state()}
					onUpload={handleUpload}
					placeholder="send a message..."
				/>
			</div>
		</div>
	);
}

function renderAttachment2(thread: ThreadT, att: Attachment) {
	const ctx = useCtx();

	function renderInfo(att: Attachment) {
		if (att.status === "uploading") {
			if (att.progress === att.file.size) {
				return `processing...`;
			} else {
				const percent = ((att.progress / att.file.size) * 100).toFixed(2);
				return `uploading (${percent}%)`;
			}
		} else {
			return renderAttachment(att.media);
		}
	}

	function removeAttachment(local_id: string) {
		ctx.dispatch({ do: "upload.cancel", local_id, thread_id: thread.id });
	}

	return (
		<>
			<div>{renderInfo(att)}</div>
			<button onClick={() => removeAttachment(att.local_id)}>
				cancel/remove
			</button>
			<Switch>
				<Match when={att.status === "uploading" && att.paused}>
					<button
						onClick={() =>
							ctx.dispatch({ do: "upload.resume", local_id: att.local_id })}
					>
						resume
					</button>
				</Match>
				<Match when={att.status === "uploading"}>
					<button
						onClick={() =>
							ctx.dispatch({ do: "upload.pause", local_id: att.local_id })}
					>
						pause
					</button>
				</Match>
			</Switch>
		</>
	);
}
