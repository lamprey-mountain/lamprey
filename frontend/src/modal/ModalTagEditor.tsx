import { createSignal, onMount } from "solid-js";
import { Modal } from "./mod";
import { useCtx } from "../context";
import { Tag, TagCreate, TagPatch } from "sdk";
import { useApi } from "../api";
import { useModals } from "../contexts/modal";
import { Checkbox } from "../icons";

interface ModalTagEditorProps {
	tag?: Tag;
	forumChannelId: string;
	onSave?: (tag: Tag) => void;
	onClose?: () => void;
}

export const ModalTagEditor = (props: ModalTagEditorProps) => {
	const ctx = useCtx();
	const api = useApi();
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
				const result = await api.channels.updateTag(
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
				const result = await api.channels.createTag(props.forumChannelId, {
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
					<input
						id="tagColor"
						type="color"
						value={color()}
						onInput={(e) => setColor(e.currentTarget.value)}
					/>
				</div>

				<div class="option">
					<label class="option">
						<input
							type="checkbox"
							checked={restricted()}
							onInput={(e) => setRestricted(e.currentTarget.checked)}
							style="display: none;"
						/>
						<Checkbox checked={restricted()} />
						<div>
							<div>Restricted</div>
							<div class="dim">
								Only users with ThreadEdit or ThreadManage can apply this tag
							</div>
						</div>
					</label>
				</div>

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
