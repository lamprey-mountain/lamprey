import { type ParentProps, Show } from "solid-js";

export interface SavebarProps {
	show: boolean;
	onCancel: () => void;
	onSave: () => void | Promise<void>;
	warningText?: string;
	cancelText?: string;
	saveText?: string;
}

export function Savebar(props: ParentProps<SavebarProps>) {
	return (
		<Show when={props.show}>
			<div class="savebar">
				<div class="inner">
					<div class="warning">
						{props.warningText ?? "you have unsaved changes"}
					</div>
					<button class="reset" onClick={props.onCancel}>
						{props.cancelText ?? "cancel"}
					</button>
					<button class="save" onClick={props.onSave}>
						{props.saveText ?? "save"}
					</button>
				</div>
			</div>
		</Show>
	);
}
