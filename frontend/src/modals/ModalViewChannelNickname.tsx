import { createSignal, onMount } from "solid-js";
import { useModals } from "@/contexts/modal";
import { Modal } from "./mod";

export const ModalViewChannelNickname = (props: {
	initial: string;
	placeholder: string;
	cont: (nickname: string) => void;
}) => {
	const [, modalCtl] = useModals();
	const [value, setValue] = createSignal(props.initial);
	let input: HTMLInputElement | undefined;

	onMount(() => input?.focus());

	return (
		<Modal
			onKeyDown={(e) => {
				if (e.key === "Escape") modalCtl.close();
			}}
		>
			<form
				onSubmit={(e) => {
					e.preventDefault();
					props.cont(value().trim());
					modalCtl.close();
				}}
			>
				<h3 class="dim">nickname</h3>
				<input
					ref={input}
					type="text"
					value={value()}
					placeholder={props.placeholder}
					onInput={(e) => setValue(e.currentTarget.value)}
				/>
				<p style="margin-top:12px;color:oklch(var(--color-fg5))">
					This name will only be visible to you.
				</p>
				<div class="bottom">
					<input type="submit" class="button primary" value="done!" />
					<button type="button" class="button" onClick={modalCtl.close}>
						nevermind...
					</button>
				</div>
			</form>
		</Modal>
	);
};
