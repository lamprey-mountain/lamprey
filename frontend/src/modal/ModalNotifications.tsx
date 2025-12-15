import { Modal } from "./mod";

interface ModalNotificationsProps {
	room_id: string;
}

export const ModalNotifications = (props: ModalNotificationsProps) => {
	return (
		<Modal>
			<div class="modal-notifications">
			</div>
		</Modal>
	);
};
