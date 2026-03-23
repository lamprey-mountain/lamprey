import { Modal } from "./mod";
import { useModals } from "../contexts/modal.tsx";
import { TextSelection } from "prosemirror-state";
import { onMount } from "solid-js";

interface ModalLinkProps {
	editor: any;
}

export const ModalLink = (props: ModalLinkProps) => {
	const [, modalCtl] = useModals();
	let textInput: HTMLInputElement | undefined;
	let urlInput: HTMLInputElement | undefined;

	onMount(() => {
		textInput?.focus();
	});

	const applyLink = () => {
		const text = textInput?.value || "";
		const url = urlInput?.value || "";

		if (!url) {
			modalCtl.close();
			return;
		}

		const view = props.editor.view;
		if (!view) {
			modalCtl.close();
			return;
		}

		const { from, to } = view.state.selection;
		const selectedText = view.state.doc.textBetween(from, to);

		const linkText = text || selectedText || url;

		const tr = view.state.tr;
		tr.insertText(`[${linkText}](${url})`, from, to);
		tr.setSelection(
			TextSelection.create(tr.doc, from + linkText.length + 3),
		);

		view.dispatch(tr);
		view.focus();
		modalCtl.close();
	};

	return (
		<Modal
			onKeyDown={(e) => {
				if (e.key === "Enter") {
					e.preventDefault();
					applyLink();
				} else if (e.key === "Escape") {
					modalCtl.close();
				}
			}}
		>
			<form
				onSubmit={(e) => {
					e.preventDefault();
					applyLink();
				}}
			>
				<div style="margin-bottom: 8px;">
					<label style="display: block; margin-bottom: 4px;">
						Text <span class="dim">(optional if selection exists)</span>
					</label>
					<input
						ref={textInput}
						type="text"
						name="text"
						placeholder="Link text"
						style="padding: 0 2px"
					/>
				</div>
				<div style="margin-bottom: 8px;">
					<label style="display: block; margin-bottom: 4px;">URL</label>
					<input
						ref={urlInput}
						type="url"
						name="url"
						placeholder="https://example.com"
						style="padding: 0 2px"
						required
					/>
				</div>
				<div class="bottom">
					<input class="primary" type="submit" value="Insert" />{" "}
					<button
						type="button"
						onClick={() => {
							modalCtl.close();
						}}
					>
						Cancel
					</button>
				</div>
			</form>
		</Modal>
	);
};
