import { Show } from "solid-js";
import { useApi } from "@/api";
import { ChannelSettings } from "@/components/features/channel_settings/index";
import { Modal } from "./mod";

export const ModalChannelSettings = (props: {
	channel_id: string;
	page?: string;
}) => {
	const api = useApi();
	const channel = api.channels.use(() => props.channel_id);
	return (
		<Modal class="modal-settings">
			<Show when={channel()}>
				{(c) => <ChannelSettings channel={c()} page={props.page ?? ""} />}
			</Show>
		</Modal>
	);
};
