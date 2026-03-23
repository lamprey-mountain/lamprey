import { Modal } from "./mod";
import { useModals } from "../contexts/modal";
import type { Api } from "../api";
import { createSignal } from "solid-js";

interface ModalKickProps {
	api: Api;
	room_id: string;
	user_id: string;
}

const kickReasons = [
	"dude, chill",
	"they hurt my feelings",
	"they broke bad",
	"they were the impostor among us",
	"goofy ahh telegram scam",
];

export const ModalKick = (props: ModalKickProps) => {
	const [, modalCtl] = useModals();
	const user = props.api.users.fetch(() => props.user_id);
	const room_member = props.api.room_members.fetch(
		() => props.room_id,
		() => props.user_id,
	);
	const [reason, setReason] = createSignal("");

	const handleKick = async () => {
		try {
			await props.api.client.http.DELETE(
				"/api/v1/room/{room_id}/member/{user_id}",
				{
					params: {
						path: {
							room_id: props.room_id,
							user_id: props.user_id,
						},
					},
					headers: {
						"X-Reason": reason() || undefined,
					},
				},
			);
			modalCtl.close();
		} catch (err) {
			console.error("Failed to kick user:", err);
			modalCtl.alert("Failed to kick user");
		}
	};

	const displayName = () =>
		room_member()?.override_name ?? user()?.name ?? props.user_id;

	const placeholderReason =
		kickReasons[Math.floor(Math.random() * kickReasons.length)];

	return (
		<Modal>
			<div class="modal-kick">
				<h3>
					kick <strong>{displayName()}</strong>
				</h3>
				<p>
					Are you sure you want to kick{" "}
					<strong>{displayName()}</strong>? They will not be able to rejoin the
					room.
				</p>

				<div style="margin-top: 16px;">
					<label for="kick-reason">reason (optional)</label>
					<input
						id="kick-reason"
						type="text"
						value={reason()}
						onInput={(e) => setReason(e.currentTarget.value)}
						placeholder={placeholderReason}
						style="width: 100%; margin-top: 4px;"
					/>
				</div>

				<div class="bottom">
					<button class="danger" onClick={handleKick}>
						kick
					</button>
					<button onClick={() => modalCtl.close()}>
						cancel
					</button>
				</div>
			</div>
		</Modal>
	);
};
