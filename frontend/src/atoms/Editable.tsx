import { createSignal, Show } from "solid-js";
import { Dynamic } from "solid-js/web";

// TODO: use this for .chat-header name edit

export type EditableProps = {
	wrapper?: string;
	value: string;
	onInput?: (s: string) => void;
	onSave?: (s: string) => void;
	blur: "cancel" | "save";
	class?: string;
	autoselect?: boolean;
};

export const Editable = (props: EditableProps) => {
	const [editing, setEditing] = createSignal(false);
	const [value, setValue] = createSignal<null | string>(null);

	const cancel = () => {
		setEditing(false);
		setValue(null);
	};

	const save = () => {
		const v = value();
		if (v) props.onSave?.(v);
		cancel();
	};

	const edit = () => {
		setValue(props.value);
		setEditing(true);
	};

	return (
		<Show
			when={editing()}
			fallback={
				<Dynamic
					component={props.wrapper}
					onClick={edit}
					class={props.class}
					classList={{ editable: true }}
				>
					{props.value}
				</Dynamic>
			}
		>
			<input
				ref={(el) =>
					queueMicrotask(() => {
						el.focus();
						if (props.autoselect) el.select();
					})
				}
				type="text"
				class={props.class}
				classList={{ editable: true, editing: true }}
				value={props.value}
				onInput={(e) => {
					setValue(e.target.value);
					props.onInput?.(e.target.value);
				}}
				onBlur={() => {
					if (props.blur === "cancel") {
						cancel();
					} else {
						save();
					}
				}}
				onKeyDown={(e) => {
					if (e.key === "Enter") {
						save();
					} else if (e.key === "Escape") {
						cancel();
					}
				}}
			/>
		</Show>
	);
};
