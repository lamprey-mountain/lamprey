import type { EditorState } from "prosemirror-state";
import type { Channel } from "sdk";
import { createSignal, For, Show } from "solid-js";
import { uuidv7 } from "uuidv7";
import { useChannels } from "@/api";
import { useCtx } from "@/app/context";
import { createEditor } from "@/components/features/editor/Editor";
import { serializeToMarkdown } from "@/components/features/editor/serializer.ts";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useUploads } from "@/contexts/uploads";
import { RenderUploadItem } from "../features/chat/Input";

// FIXME: allow uploading attachments when creating a thread
export const Forum2CreateForm = (props: {
	channel: Channel;
	onCancel: () => void;
	onSuccess: () => void;
}) => {
	const ctx = useCtx();
	const channels2 = useChannels();
	const uploads = useUploads();
	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const [title, setTitle] = createSignal("");
	const [formEditorState, setFormEditorState] = createSignal<EditorState>();
	const [attachments, setAttachments] = createSignal<string[]>([]);
	const attachmentMap = () => ctx.uploads;

	const formEditor = createEditor({
		channelId: () => props.channel.id,
		roomId: () => props.channel.room_id ?? "",
		toolbar,
		autocomplete,
		initialContent: () => "",
	});

	function handleUpload(file: File) {
		const local_id = uuidv7();
		uploads.init(local_id, props.channel.id, file);
		setAttachments((prev) => [...prev, local_id]);
	}

	function uploadFile(e: InputEvent) {
		alert("uploading files here is currently broken :(");
		return;
		const target = e.target as HTMLInputElement | null;
		if (!target?.files) return;
		const files = Array.from(target.files);
		for (const file of files) {
			handleUpload(file);
		}
	}

	function handleFormSubmit() {
		if (!title().trim()) return;
		const content = serializeToMarkdown(
			formEditorState()?.doc ?? formEditor.schema.create(null),
		);
		channels2.create(props.channel.room_id ?? "", {
			name: title(),
			parent_id: props.channel.id,
			type: "ThreadForum2",
			starter_message: {
				content,
				// attachments: attachments().map((id) => ({
				// 	type: "Local",
				// 	local_id: id,
				// })),
				mentions: {},
			},
		});
		props.onSuccess();
	}

	return (
		<div class="forum2-create-form">
			<input
				type="text"
				placeholder="title..."
				class="title-input"
				value={title()}
				onInput={(e) => setTitle(e.target.value)}
			/>
			<formEditor.View
				onSubmit={handleFormSubmit}
				onChange={setFormEditorState}
				onUpload={handleUpload}
				placeholder="message content..."
				channelId={props.channel.id}
				submitOnEnter={false}
			/>
			<div class="attachments" style="margin: 8px 0;">
				<ul>
					<For each={attachments()}>
						{(local_id) => {
							const att = attachmentMap().get(local_id);
							return (
								<Show when={att}>
									<RenderUploadItem thread_id={props.channel.id} att={att!} />
								</Show>
							);
						}}
					</For>
				</ul>
			</div>
			<div style="display: flex; align-items: center; gap: 8px; margin-top: 8px;">
				<label class="upload button">
					upload file
					<input
						multiple
						type="file"
						onInput={uploadFile}
						style="display: none"
					/>
				</label>
				<div style="flex: 1"></div>
				<button type="button" class="button secondary" onClick={props.onCancel}>
					Cancel
				</button>
				<button type="button" class="button primary" onClick={handleFormSubmit}>
					Post
				</button>
			</div>
		</div>
	);
};
