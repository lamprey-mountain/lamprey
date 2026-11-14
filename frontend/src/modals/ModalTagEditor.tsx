import type { EditorState } from "prosemirror-state";
import type { Tag, TagCreate } from "sdk";
import { createSignal } from "solid-js";
import { useChannels } from "@/api";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { ColorPickerButton } from "@/atoms/ColorPicker";
import { Checkbox } from "@/atoms/icons";
import { createEditor } from "@/components/features/editor/Editor";
import { serializeToMarkdown } from "@/components/features/editor/serializer";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useModals } from "@/contexts/modal";
import { Modal } from "./mod";

const MAX_DESCRIPTION_LENGTH = 8192;

interface ModalTagEditorProps {
	tag?: Tag;
	forumChannelId: string;
	onSave?: (tag: Tag) => void;
	onClose?: () => void;
}

export const ModalTagEditor = (props: ModalTagEditorProps) => {
	const channels2 = useChannels();
	const [, modalCtl] = useModals();
	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();
	const forumChannel = channels2.use(() => props.forumChannelId);

	const [name, setName] = createSignal(props.tag?.name || "");
	const [description, setDescription] = createSignal(
		props.tag?.description || "",
	);
	const [color, setColor] = createSignal(props.tag?.color);
	const [restricted, setRestricted] = createSignal(
		props.tag?.restricted || false,
	);
	const [loading, setLoading] = createSignal(false);
	const [error, setError] = createSignal<string | null>(null);

	const editor = createEditor({
		channelId: () => props.forumChannelId,
		roomId: () => forumChannel()?.room_id ?? "",
		toolbar,
		autocomplete,
		initialContent: () => description(),
	});

	// the editor dispatches metadata transactions on mount, and serializing the parsed
	// content back to markdown doesn't reproduce the stored description exactly. keep the
	// description untouched until the document actually changes.
	let descriptionBaseline: string | undefined;

	const handleDescriptionChange = (state: EditorState) => {
		const markdown = serializeToMarkdown(state.doc);
		descriptionBaseline ??= markdown;
		if (markdown === descriptionBaseline) return;
		descriptionBaseline = markdown;
		setDescription(markdown);
	};

	const handleSubmit = async (e: SubmitEvent) => {
		e.preventDefault();

		if (description().length > MAX_DESCRIPTION_LENGTH) {
			setError(
				`Description must be at most ${MAX_DESCRIPTION_LENGTH} characters.`,
			);
			return;
		}

		setLoading(true);
		setError(null);

		try {
			if (props.tag) {
				// update existing tag
				const result = await channels2.updateTag(
					props.forumChannelId,
					props.tag.id,
					{
						name: name() || undefined,
						description: description() || undefined,
						color: color(),
						restricted: restricted(),
					},
				);
				props.onSave?.(result);
			} else {
				// create new tag
				const result = await channels2.createTag(props.forumChannelId, {
					name: name(),
					description: description() || undefined,
					color: color(),
					restricted: restricted(),
				} as TagCreate);
				props.onSave?.(result);
			}
			modalCtl.close();
		} catch (err) {
			console.error("Error saving tag:", err);
			setError(err instanceof Error ? err.message : "Failed to save tag");
		} finally {
			setLoading(false);
		}
	};

	return (
		<Modal>
			<h3>{props.tag ? "Edit Tag" : "Create Tag"}</h3>
			<form class="tag-edit-form" onSubmit={handleSubmit}>
				<div class="option-block">
					<label for="tagName" class="small">
						Name
					</label>
					<input
						id="tagName"
						type="text"
						value={name()}
						onInput={(e) => setName(e.currentTarget.value)}
						required
						maxLength={64}
					/>
				</div>

				<div class="option-block">
					<label class="small">Description</label>
					<editor.View
						autofocus={false}
						placeholder="what is this tag for?"
						submitOnEnter={false}
						onChange={handleDescriptionChange}
					/>
				</div>

				<div class="option-block">
					<label for="tagColor">Color</label>
					<ColorPickerButton
						value={color() ?? "#000000"}
						onInput={(newColor) => setColor(newColor)}
					/>
				</div>

				<CheckboxOption
					id={`modal-tag-editor-${props.forumChannelId}-restricted`}
					checked={restricted()}
					onChange={setRestricted}
					seed={`modal-tag-editor-${props.forumChannelId}-restricted`}
				>
					<Checkbox
						checked={restricted()}
						seed={`modal-tag-editor-${props.forumChannelId}-restricted`}
					/>
					<div>
						<div>Restricted</div>
						<div class="dim">
							Only users with ThreadEdit or ThreadManage can apply this tag
						</div>
					</div>
				</CheckboxOption>

				{error() && <div class="error">{error()}</div>}

				<div class="bottom">
					<button type="button" class="button" onClick={modalCtl.close}>
						Cancel
					</button>
					<button type="submit" class="button primary" disabled={loading()}>
						{loading() ? "Saving..." : props.tag ? "Update" : "Create"}
					</button>
				</div>
			</form>
		</Modal>
	);
};
