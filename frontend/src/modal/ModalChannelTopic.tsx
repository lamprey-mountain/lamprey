import { Modal } from "./mod";
import { md } from "../markdown_utils.tsx";
import { useApi } from "../api";

interface ModalChannelTopicProps {
	channel_id: string;
}

export const ModalChannelTopic = (props: ModalChannelTopicProps) => {
	const api = useApi();
	const channel = api.channels.fetch(() => props.channel_id);

	return (
		<Modal>
			<div class="modal-channel-topic">
				<h3>#{channel()?.name}</h3>
				<div
					class="topic-content markdown"
					innerHTML={md(channel()?.description ?? "") as string}
				/>
			</div>
		</Modal>
	);
};
