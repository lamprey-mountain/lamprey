import { createMemo, createSignal, onMount } from "solid-js";
import { useMessages } from "@/api";
import {
	useAutocomplete,
	useFormattingToolbar,
	useOptionalChannel,
} from "@/contexts/mod";
import type { MessageT } from "@/types";
import { createEditor } from "../editor/Editor";
import { serializeToMarkdown } from "../editor/serializer";
import { asMarkdown2, isMarkdown, isMarkdown2 } from "./util";

export const MessageEditor = (props: { message: MessageT }) => {
	const messagesService = useMessages();
	const [ch, chUpdate] = useOptionalChannel();

	const content = createMemo(() => {
		const v = asMarkdown2(props.message.latest_version);
		if (v) {
			return v.content ?? "";
		} else {
			return "";
		}
	});
	const [draft, setDraft] = createSignal(content());

	if (!ch || !chUpdate) {
		return <div class="message-editor">Error: No channel context</div>;
	}

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const editor = createEditor({
		channelId: () => props.message.channel_id ?? "",
		roomId: () => props.message.room_id ?? "",
		toolbar,
		autocomplete,
		initialContent: () => draft(),
		initialSelection: ch.editingMessage?.selection,
		keymap: {
			ArrowUp: (state) => {
				if (state.selection.from !== 1) return false;

				const ranges = messagesService._ranges.get(props.message.channel_id);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex(
					(m) => m.id === props.message.id,
				);
				if (currentIndex === -1) return false;

				for (let i = currentIndex - 1; i >= 0; i--) {
					const msg = messages[i];
					if (isMarkdown(msg.latest_version.type)) {
						chUpdate("editingMessage", {
							message_id: msg.id,
							selection: "end",
						});
						return true;
					}
				}

				return false;
			},
			ArrowDown: (state) => {
				if (state.selection.to !== state.doc.content.size - 1) return false;

				const ranges = messagesService._ranges.get(props.message.channel_id);
				if (!ranges) return false;

				const messages = ranges.live.items;
				const currentIndex = messages.findIndex(
					(m) => m.id === props.message.id,
				);
				if (currentIndex === -1) return false;

				for (let i = currentIndex + 1; i < messages.length; i++) {
					const msg = messages[i];
					if (isMarkdown(msg.latest_version.type)) {
						chUpdate("editingMessage", {
							message_id: msg.id,
							selection: "start",
						});
						return true;
					}
				}

				chUpdate("editingMessage", undefined);
				ch.input_focus?.();
				return true;
			},
		},
	});

	const save = async (content: string) => {
		const oldContent = isMarkdown2(props.message.latest_version)
			? (props.message.latest_version.content ?? "")
			: "";
		if (content.trim() === oldContent.trim()) {
			chUpdate("editingMessage", undefined);
			return;
		}
		if (content.trim().length === 0) {
			chUpdate("editingMessage", undefined);
			return;
		}
		try {
			await messagesService.edit(
				props.message.channel_id,
				props.message.id,
				content,
			);
		} catch (e) {
			console.error("failed to edit message", e);
		}
		chUpdate("editingMessage", undefined);
	};

	const cancel = () => {
		chUpdate("editingMessage", undefined);
		ch.input_focus?.();
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
		<div class="message-editor" ref={containerRef}>
			<editor.View
				placeholder="edit message..."
				onSubmit={(text) => {
					save(text);
					return true;
				}}
				onChange={(state) => {
					const text = serializeToMarkdown(state.doc);
					setDraft(text);
				}}
			/>
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
};
