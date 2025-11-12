import { createSignal, onMount } from "solid-js";
import { Modal } from "./mod";
import { useCtx } from "../context";
import { Tag, TagCreate, TagPatch } from "sdk";
import { useApi } from "../api";

interface ModalTagEditorProps {
	tag?: Tag;
	forumChannelId: string;
	onSave?: (tag: Tag) => void;
	onClose?: () => void;
}

export const ModalTagEditor = (props: ModalTagEditorProps) => {
	const ctx = useCtx();
	const api = useApi();
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
			ctx.dispatch({ do: "modal.close" });
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
			<form onSubmit={handleSubmit}>
				<div class="form-group">
					<label for="tagName">Name</label>
					<input
						id="tagName"
						type="text"
						value={name()}
						onInput={(e) => setName(e.currentTarget.value)}
						required
						maxLength={64}
					/>
				</div>

				<div class="form-group">
					<label for="tagDescription">Description</label>
					<textarea
						id="tagDescription"
						value={description()}
						onInput={(e) => setDescription(e.currentTarget.value)}
						maxLength={8192}
						rows={3}
					/>
				</div>

				<div class="form-group">
					<label for="tagColor">Color</label>
					<input
						id="tagColor"
						type="color"
						value={color()}
						onInput={(e) => setColor(e.currentTarget.value)}
					/>
				</div>

				<div class="form-group checkbox-group">
					<label for="tagRestricted">
						<input
							id="tagRestricted"
							type="checkbox"
							checked={restricted()}
							onInput={(e) => setRestricted(e.currentTarget.checked)}
						/>
						<span>
							Restricted (only users with ThreadEdit or ThreadManage can apply
							this tag)
						</span>
					</label>
				</div>

				{error() && (
					<div class="error">
						{error()}
					</div>
				)}

				<div class="bottom">
					<button
						type="button"
						onClick={() => ctx.dispatch({ do: "modal.close" })}
					>
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
