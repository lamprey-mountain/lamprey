import { createSignal, type ParentProps } from "solid-js";
import { CheckboxOptionWithLabel } from "../atoms/CheckboxOption";
import { useCtx } from "../context";
import { useModals } from "../contexts/modal";
import { RadioDot } from "../icons";
import { Modal } from "./mod";

interface ModalRoomCreateProps {
	cont: (data: { name: string; public: boolean }) => void;
}

export const ModalRoomCreate = (props: ModalRoomCreateProps) => {
	const [roomName, setRoomName] = createSignal("");
	const [isPublic, setIsPublic] = createSignal(false);
	const [, modalCtl] = useModals();

	const handleSubmit = (e: SubmitEvent) => {
		e.preventDefault();
		if (!roomName().trim()) return;

		props.cont({
			name: roomName().trim(),
			public: isPublic(),
		});
		modalCtl.close();
	};

	const handleCancel = () => {
		props.cont(null as any);
		modalCtl.close();
	};

	return (
		<Modal>
			<h3>new room</h3>
			<form class="new-room" onSubmit={handleSubmit}>
				<label style="display: block; margin-top: 12px">
					<h3 class="dim">room name</h3>
					<input
						type="text"
						value={roomName()}
						onInput={(e) => setRoomName(e.currentTarget.value)}
						placeholder="my awesome room"
						required
						autofocus
					/>
				</label>

				<CheckboxOptionWithLabel
					id="room-public"
					checked={isPublic()}
					onChange={setIsPublic}
					seed="public"
					label="public room (visible to everyone)"
				/>

				<div class="bottom">
					<button type="button" onClick={handleCancel}>
						Cancel
					</button>
					<button type="submit" class="primary">
						Create Room
					</button>
				</div>
			</form>
		</Modal>
	);
};
