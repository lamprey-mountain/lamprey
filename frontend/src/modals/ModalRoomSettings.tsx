import { Show } from "solid-js";
import { useApi } from "@/api";
import { RoomSettings } from "@/components/features/room_settings/RoomSettings";
import { Modal } from "./mod";

export const ModalRoomSettings = (props: {
	room_id: string;
	page?: string;
}) => {
	const api = useApi();
	const room = api.rooms.use(() => props.room_id);
	return (
		<Modal class="modal-settings">
			<Show when={room()}>
				{(r) => <RoomSettings room={r()} page={props.page ?? ""} />}
			</Show>
		</Modal>
	);
};
