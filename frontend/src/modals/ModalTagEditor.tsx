import { createSignal, onMount } from "solid-js";
import { Modal } from "./mod";
import { useCtx } from "../context";
import { type Tag, type TagCreate, TagPatch } from "sdk";
import { useApi2, useChannels2 } from "@/api";
import { useModals } from "../contexts/modal";
import { Checkbox } from "../icons";
import { Colorpicker } from "../atoms/Colorpicker";
import { CheckboxOption } from "../atoms/CheckboxOption";

interface ModalTagEditorProps {
	tag?: Tag;
	forumChannelId: string;
	onSave?: (tag: Tag) => void;
	onClose?: () => void;
}

export const ModalTagEditor = (props: ModalTagEditorProps) => {
	const ctx = useCtx();
	const api2 = useApi2();
	const channels2 = useChannels2();
	const [, modalCtl] = useModals();

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

	const handleSubmit = async (e: SubmitEvent) => {
		e.preventDefault();
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
					<label for="tagName" class="small">Name</label>
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
					<label for="tagDescription">Description</label>
					<textarea
						id="tagDescription"
						value={description()}
						onInput={(e) => setDescription(e.currentTarget.value)}
						maxLength={8192}
						rows={3}
					/>
				</div>

				<div class="option-block">
					<label for="tagColor">Color</label>
					<Colorpicker
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

				{error() && (
					<div class="error">
						{error()}
					</div>
				)}

				<div class="bottom">
					<button type="button" onClick={modalCtl.close}>
						Cancel
					</button>
					<button type="submit" class="primary" disabled={loading()}>
						{loading() ? "Saving..." : (props.tag ? "Update" : "Create")}
					</button>
				</div>
			</form>
		</Modal>
	);
};
