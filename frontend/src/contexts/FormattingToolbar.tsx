import { TextSelection } from "prosemirror-state";
import type { EditorView } from "prosemirror-view";
import iconBold from "@/assets/format-bold.png";
import iconCode from "@/assets/format-code.png";
import iconItalic from "@/assets/format-italic.png";
import iconStrikethrough from "@/assets/format-strikethrough.png";
import { setIsApplyingFormat } from "@/components/features/editor/Editor";
import { useFormattingToolbar } from "./formatting-toolbar.tsx";

type FormattingToolbarProps = {
	onClose: () => void;
};

const isFormatActive = (view: EditorView, wrapper: string) => {
	const { state } = view;
	const { from, to } = state.selection;
	if (from >= to) return false;

	const beforeStart = Math.max(0, from - wrapper.length);
	const afterEnd = Math.min(state.doc.content.size, to + wrapper.length);

	return (
		state.doc.textBetween(beforeStart, from) === wrapper &&
		state.doc.textBetween(to, afterEnd) === wrapper
	);
};

const toggleFormat = (view: EditorView, wrapper: string) => {
	try {
		const { state, dispatch } = view;
		const { from, to } = state.selection;

		// Ensure valid selection range
		if (from >= to || from < 0 || to > state.doc.content.size) return false;

		const selectedText = state.doc.textBetween(from, to);
		if (!selectedText) return false;

		const beforeStart = Math.max(0, from - wrapper.length);
		const afterEnd = Math.min(state.doc.content.size, to + wrapper.length);

		const textBefore = state.doc.textBetween(beforeStart, from);
		const textAfter = state.doc.textBetween(to, afterEnd);

		const hasFormat = textBefore === wrapper && textAfter === wrapper;

		const tr = state.tr;

		if (hasFormat) {
			// Remove formatting - delete the wrapper markers
			const newFrom = from - wrapper.length;
			const newTo = to - wrapper.length;
			// Delete from end to start to preserve positions
			tr.delete(to, to + wrapper.length);
			tr.delete(from - wrapper.length, from);
			tr.setSelection(TextSelection.create(tr.doc, newFrom, newTo));
		} else {
			// Add formatting
			tr.insertText(wrapper, from, to);
			tr.insertText(selectedText, from + wrapper.length);
			tr.insertText(wrapper, from + wrapper.length + selectedText.length);
			tr.setSelection(
				TextSelection.create(
					tr.doc,
					from + wrapper.length,
					from + wrapper.length + selectedText.length,
				),
			);
		}

		dispatch(tr);
		view.focus();

		return hasFormat;
	} catch (e) {
		console.error("Failed to toggle format:", e);
		return false;
	}
};

export const FormattingToolbar = (_props: FormattingToolbarProps) => {
	const { toolbar } = useFormattingToolbar();

	const isActive = (wrapper: string) => {
		const view = toolbar().view;
		return view ? isFormatActive(view, wrapper) : false;
	};

	const applyFormat = (wrapper: string) => {
		const view = toolbar().view;
		if (!view) return;
		setIsApplyingFormat(true);

		toggleFormat(view, wrapper);

		setTimeout(() => {
			setIsApplyingFormat(false);
		}, 100);
	};

	return (
		<div class="formatting-toolbar">
			<button
				type="button"
				class="button"
				classList={{ active: isActive("**") }}
				onMouseDown={(e) => e.preventDefault()}
				onClick={() => applyFormat("**")}
				title="Bold"
			>
				<img src={iconBold} />
			</button>
			<button
				type="button"
				class="button"
				classList={{ active: isActive("*") }}
				onMouseDown={(e) => e.preventDefault()}
				onClick={() => applyFormat("*")}
				title="Italic"
			>
				<img src={iconItalic} />
			</button>
			<button
				type="button"
				class="button"
				classList={{ active: isActive("`") }}
				onMouseDown={(e) => e.preventDefault()}
				onClick={() => applyFormat("`")}
				title="Code"
			>
				<img src={iconCode} />
			</button>
			<button
				type="button"
				class="button"
				classList={{ active: isActive("~~") }}
				onMouseDown={(e) => e.preventDefault()}
				onClick={() => applyFormat("~~")}
				title="Strikethrough"
			>
				<img src={iconStrikethrough} />
			</button>
		</div>
	);
};
