import { Modal } from "./mod";
import { md } from "../markdown_utils.tsx";
import { useApi, useChannels2 } from "@/api";

interface ModalChannelTopicProps {
	channel_id: string;
}

export const ModalChannelTopic = (props: ModalChannelTopicProps) => {
	const channels2 = useChannels2();
	const channel = channels2.use(() => props.channel_id);

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
