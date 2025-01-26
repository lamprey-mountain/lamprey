import {
	createEffect,
	For,
	Match,
	on,
	Show,
	Switch,
	useContext,
} from "solid-js";
import Editor from "./Editor.tsx";

import { Attachment, chatctx, ThreadState } from "./context.ts";
import { createList } from "./list.tsx";
import { RoomT, ThreadT } from "./types.ts";
import { uuidv7 } from "uuidv7";
import { throttle } from "@solid-primitives/scheduled";
import { renderAttachment } from "./Message.tsx";
import { renderTimelineItem } from "./Messages.tsx";

type ChatProps = {
	thread: ThreadT;
	room: RoomT;
};

export const ChatMain = (props: ChatProps) => {
	const ctx = useContext(chatctx)!;

	let paginating = false;
	const slice = () => ctx.data.slices[props.thread.id];
	const tl = () => ctx.data.timelines[props.thread.id];
	const ts = () =>
		ctx.data.thread_state[props.thread.id] as ThreadState | undefined;
	const reply = () => ctx.data.messages[ts()?.reply_id!];
	const hasSpaceTop = () => tl()?.[0]?.type === "hole" || slice()?.start > 0;
	const hasSpaceBottom = () =>
		tl()?.at(-1)?.type === "hole" || slice()?.end < tl()?.length;

	function init() {
		const thread_id = props.thread.id;
		const read_id = props.thread.last_read_id ?? undefined;
		ctx.dispatch({ do: "thread.init", thread_id, read_id });
	}

	init();
	createEffect(init);

	const list = createList({
		items: () => ts()?.timeline ?? [],
		autoscroll: () => !hasSpaceBottom(),
		topQuery: ".message > .content",
		bottomQuery: ":nth-last-child(1 of .message) > .content",
		async onPaginate(dir) {
			if (paginating) return;
			paginating = true;
			const thread_id = props.thread.id;
			if (dir === "forwards") {
				await ctx.dispatch({ do: "paginate", dir: "f", thread_id });
				const isAtEnd = ctx.data.slices[thread_id].end ===
					ctx.data.timelines[thread_id].length;
				if (isAtEnd) {
					ctx.dispatch({ do: "thread.mark_read", thread_id, delay: true });
				}
			} else {
				await ctx.dispatch({ do: "paginate", dir: "b", thread_id });
			}
			// setTimeout(() => paginating = false, 1000);
			paginating = false;
		},
		onContextMenu(e: MouseEvent) {
			e.stopPropagation();
			const target = e.target as HTMLElement;
			const media_el = target.closest("a, img, video, audio") as HTMLElement;
			const message_el = target.closest("li[data-message-id]") as HTMLElement;
			const message_id = message_el?.dataset.messageId;
			if (!message_id || (media_el && message_el.contains(media_el))) {
				ctx.dispatch({
					do: "menu",
					menu: null,
				});
				return;
			}
			e.preventDefault();
			ctx.dispatch({
				do: "menu",
				menu: {
					type: "message",
					x: e.x,
					y: e.y,
					message: ctx.data.messages[message_id],
				},
			});
		},
	});

	createEffect(() => {
		list.scrollPos();
		throttle(() => {
			init(); // FIXME: don't init on all scroll
			ctx.dispatch({
				do: "thread.scroll_pos",
				thread_id: props.thread.id,
				pos: list.scrollPos(),
				is_at_end: list.isAtBottom(),
			});
		});
	});

	createEffect(async () => {
		if (slice()?.start === undefined) {
			if (paginating) return;
			paginating = true;
			await ctx.dispatch({
				do: "paginate",
				dir: "b",
				thread_id: props.thread.id,
			});
			list.scrollTo(999999);
			paginating = false;
		}
	});

	createEffect(on(() => props.thread, () => {
		// TODO: restore scroll position
		queueMicrotask(() => {
			const pos = ts()!.scroll_pos;
			if (!pos) return list.scrollTo(999999);
			list.scrollTo(pos);
		});
	}));

	// TODO: handle this with onSubmit if possible
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

	function removeAttachment(local_id: string) {
		ctx.dispatch({
			do: "thread.attachments",
			thread_id: props.thread.id,
			attachments: [...ts()!.attachments].filter((i) =>
				i.local_id !== local_id
			),
		});
	}

	function renderAttachmentInfo(att: Attachment) {
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

	function renderAttachment2(att: Attachment) {
		return (
			<>
				<div>
					{renderAttachmentInfo(att)}
				</div>
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

	return (
		<div class="chat">
			<list.List>
				{(item) => renderTimelineItem(props.thread, item)}
			</list.List>
			<div class="input">
				<Show when={ts()?.reply_id}>
					<div class="reply">
						<button
							class="cancel"
							onClick={() =>
								ctx.dispatch({
									do: "thread.reply",
									thread_id: props.thread.id,
									reply_id: null,
								})}
						>
							cancel
						</button>
						<div class="info">
							replying to {reply()?.override_name ?? reply()?.author.name}:{" "}
							{reply()?.content}
						</div>
					</div>
				</Show>
				<Show when={ts()?.attachments.length}>
					<ul class="attachments">
						<For each={ts()!.attachments}>
							{(att) => (
								<li>
									{renderAttachment2(att)}
								</li>
							)}
						</For>
					</ul>
				</Show>
				<Show when={ts()}>
					<Editor
						state={ts()!.editor_state}
						onUpload={handleUpload}
						placeholder="send a message..."
					/>
				</Show>
			</div>
		</div>
	);
};
