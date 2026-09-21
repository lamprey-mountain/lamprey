import { Show } from "solid-js";
import { UserSettings } from "@/components/features/user_settings";
import { useCurrentUser } from "@/contexts/currentUser";
import { Modal } from "./mod";

export const ModalUserSettings = (props: { page?: string }) => {
	const user = useCurrentUser();

	return (
		<Modal class="modal-settings">
			<Show when={user()}>
				{(u) => <UserSettings user={u()} page={props.page ?? ""} />}
			</Show>
		</Modal>
	);
};
