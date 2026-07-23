import { useNavigate } from "@solidjs/router";
import type { EditorState } from "prosemirror-state";
import type { Channel } from "sdk";
import { createSignal, For, Show } from "solid-js";
import { uuidv7 } from "uuidv7";
import { useChannels } from "@/api";
import { createEditor } from "@/components/features/editor/Editor";
import { serializeToMarkdown } from "@/components/features/editor/serializer.ts";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useChannel } from "@/contexts/mod";
import { useUploads } from "@/contexts/uploads";
import { RenderUploadItem } from "../features/chat/Input";

export const Forum2CreateForm = (props: {
	channel: Channel;
	onCancel: () => void;
	onSuccess: () => void;
}) => {
	const channels2 = useChannels();
	const navigate = useNavigate();
	const uploads = useUploads();
	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const [title, setTitle] = createSignal("");
	const [formEditorState, setFormEditorState] = createSignal<EditorState>();
	const [ch] = useChannel();

	const formEditor = createEditor({
		channelId: () => props.channel.id,
		roomId: () => props.channel.room_id ?? "",
		toolbar,
		autocomplete,
		// TODO: save drafts
		initialContent: () => "",
	});

	function handleUpload(file: File) {
		const local_id = uuidv7();
		uploads.init(local_id, props.channel.id, file);
	}

	function uploadFile(e: InputEvent) {
		const target = e.target as HTMLInputElement | null;
		if (!target?.files) return;
		const files = Array.from(target.files);
		for (const file of files) {
			handleUpload(file);
		}
	}

	async function handleFormSubmit() {
		if (!title().trim()) return;
		const doc = formEditorState()?.doc;
		const content = doc ? serializeToMarkdown(doc) : null;

		// TODO: warn if any attachments aren't uploaded yet

		const newChannel = await channels2.create(props.channel.room_id ?? "", {
			name: title(),
			parent_id: props.channel.id,
			type: "ThreadForum2",
			starter_message: {
				content,
				attachments: ch.attachments
					.filter((att) => att.status === "uploaded")
					.map((att) => ({
						type: "Media",
						media_id: att.media.id,
						spoiler: att.spoiler,
					})),
			},
		});
		props.onSuccess();
		navigate(`/channel/${newChannel.id}`);
	}

	function handleEditorSubmit(_s: string) {
		handleFormSubmit();
		return true;
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
				onSubmit={handleEditorSubmit}
				onChange={setFormEditorState}
				onUpload={handleUpload}
				placeholder="message content..."
				channelId={props.channel.id}
				submitOnEnter={false}
			/>
			<Show when={ch.attachments.length > 0}>
				<div class="attachments" style="margin: 8px 0;">
					<header>
						{ch.attachments.length}{" "}
						{ch.attachments.length === 1 ? "attachment" : "attachments"}
					</header>
					<ul>
						<For each={ch.attachments}>
							{(att) => (
								<RenderUploadItem thread_id={props.channel.id} att={att} />
							)}
						</For>
					</ul>
				</div>
			</Show>
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
