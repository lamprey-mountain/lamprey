import type { Transaction } from "prosemirror-state";
import { createSignal, onMount } from "solid-js";
import type { Channel, Message } from "ts-sdk";
import { useMessages } from "@/api";
import { EmojiButton } from "@/atoms/EmojiButton";
import {
	useAutocomplete,
	useChannel,
	useFormattingToolbar,
} from "@/contexts/mod";
import { isMarkdown } from "../chat/Message";
import { createEditor } from "../editor/Editor";

export function CommentEditor(props: { message: Message; channel: Channel }) {
	const messagesService = useMessages();
	const [ch, chUpdate] = useChannel();
	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();
	const [draft, setDraft] = createSignal(
		isMarkdown(props.message.latest_version.type)
			? (props.message.latest_version.content ?? "")
			: "",
	);

	const onEmojiPick = (emoji: string, _keepOpen?: boolean) => {
		const editorState = ch.editor_state;
		if (editorState) {
			const { from, to } = editorState.selection;
			const customMatch = emoji.match(/^<:([^:]+):([^>]+)>$/);
			let tr: Transaction;
			if (customMatch) {
				const name = customMatch[1];
				const id = customMatch[2];
				tr = editorState.tr.replaceWith(
					from,
					to,
					editor.schema.nodes.emojiCustom.create({ id, name }),
				);
			} else {
				tr = editorState.tr.insertText(emoji, from, to);
			}
			const newState = editorState.apply(tr);
			chUpdate("editor_state", newState);
		}
	};

	const editor = createEditor({
		channelId: () => props.message.channel_id,
		roomId: () => props.message.room_id!,
		toolbar,
		autocomplete,
		initialContent: () => draft(),
		initialSelection: "end",
	});

	const save = (content: string) => {
		const currentContent = isMarkdown(props.message.latest_version.type)
			? (props.message.latest_version.content ?? "")
			: "";

		if (content.trim() === currentContent.trim()) {
			chUpdate("editingMessage", undefined);
			return true;
		}
		if (content.trim().length === 0) {
			chUpdate("editingMessage", undefined);
			return true;
		}
		messagesService
			.edit(props.message.channel_id, props.message.id, content)
			.catch((e) => {
				console.error("failed to edit comment", e);
			});
		chUpdate("editingMessage", undefined);
		return true;
	};

	const cancel = () => {
		chUpdate("editingMessage", undefined);
	};

	let containerRef: HTMLDivElement | undefined;
	onMount(() => {
		containerRef?.addEventListener(
			"keydown",
			(e) => {
				if (e.key === "Escape") {
					e.stopPropagation();
					cancel();
				}
			},
			{ capture: true },
		);
		editor.focus();
	});

	return (
		<div class="comment-editor" ref={containerRef}>
			<div class="text">
				<editor.View
					onSubmit={save}
					onChange={(state) => {
						const text = state.doc.textContent;
						setDraft(text);
					}}
					channelId={props.channel.id}
					submitOnEnter={false}
				/>
				<EmojiButton picked={onEmojiPick} />
			</div>
			<div class="edit-info dim">
				escape to{" "}
				<button type="button" class="button" onClick={cancel}>
					cancel
				</button>{" "}
				• enter to{" "}
				<button type="button" class="button" onClick={() => save(draft())}>
					save
				</button>
			</div>
		</div>
	);
}
