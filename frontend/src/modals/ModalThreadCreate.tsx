import { useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";
import { useApi } from "@/api/mod.ts";
import { CheckboxOptionWithLabel } from "@/atoms/CheckboxOption.tsx";
import { useModals } from "@/contexts/modal.tsx";
import { Modal } from "./mod.tsx";

export const ModalThreadCreate = (props: {
	room_id: string;
	channel_id: string;
}) => {
	const [, modalCtl] = useModals();
	const nav = useNavigate();
	const api = useApi();

	const [name, setName] = createSignal("");
	const [isPrivate, setPrivate] = createSignal(false);
	const [loading, setLoading] = createSignal(false);

	const handleSubmit = async (e: SubmitEvent) => {
		e.preventDefault();
		setLoading(true);
		const chan = await api.channels.create(props.room_id, {
			name: name(),
			parent_id: props.channel_id,
			type: isPrivate() ? "ThreadPrivate" : "ThreadPublic",
		});
		modalCtl.close();
		nav(`/channel/${chan.id}`);
	};

	return (
		<Modal>
			<h3>create thread</h3>
			<form onSubmit={handleSubmit} style="margin-top:8px;">
				<label>
					<h3 class="dim">name</h3>
					<input
						type="text"
						name="name"
						required
						ref={(el) => queueMicrotask(() => el.focus())}
						onInput={(e) => setName(e.target.value)}
						style="width:100%"
					/>
				</label>
				<CheckboxOptionWithLabel
					id="thread-private"
					checked={isPrivate()}
					onChange={setPrivate}
					seed="thread-private"
					label="private thread"
					description="only thread members and moderators can view"
				/>
				<div class="bottom">
					<button
						type="button"
						class="button link"
						onClick={() => modalCtl.close()}
					>
						nevermind...
					</button>
					<input
						type="submit"
						class="button primary"
						value={loading() ? "creating..." : "create"}
					></input>
				</div>
			</form>
		</Modal>
	);
};
