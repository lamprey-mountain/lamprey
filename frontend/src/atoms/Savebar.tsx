import { type ParentProps, createSignal, onCleanup } from "solid-js";

export interface SavebarProps {
	show?: boolean;
	disabled?: boolean;
	saving?: boolean;
	onCancel: () => void;
	onSave: () => void | Promise<void>;
	warningText?: string;
	cancelText?: string;
	saveText?: string;
}

export function Savebar(props: ParentProps<SavebarProps>) {
	const [width, setWidth] = createSignal(0);

	const ro = new ResizeObserver((entries) => {
		for (const entry of entries) {
			setWidth(entry.contentRect.width);
		}
	});

	onCleanup(() => ro.disconnect());

	const disabled = () => props.saving || props.disabled;

	return (
		<>
			<div class="savebar-sizer" ref={(el) => ro.observe(el)}></div>
			<div
				class="savebar"
				classList={{
					show: props.show,
					disabled: props.disabled,
					saving: props.saving,
				}}
				style={{
					width: `${width()}px`,
				}}
			>
				<div class="inner">
					<div class="warning">
						{props.warningText ?? "you have unsaved changes"}
					</div>
					<button
						type="button"
						class="button reset"
						disabled={disabled()}
						onClick={props.onCancel}
					>
						{props.cancelText ?? "cancel"}
					</button>
					<button
						type="button"
						class="button save"
						disabled={disabled()}
						onClick={props.onSave}
					>
						{props.saving ? "saving" : (props.saveText ?? "save")}
					</button>
				</div>
			</div>
		</>
	);
}
