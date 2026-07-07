import { useChannels } from "@/api";
import { md } from "@/lib/markdown";
import { Modal } from "./mod";
import { Markdown } from "@/atoms/Markdown";
import { Icon } from "@/atoms/Icon";
import icClose from "@/assets/x.png"; // TODO: random icons
import { useModals } from "@/contexts/modal";

interface ModalChannelTopicProps {
	channel_id: string;
}

export const ModalChannelTopic = (props: ModalChannelTopicProps) => {
	const channels2 = useChannels();
	const channel = channels2.use(() => props.channel_id);
	const [_, modalctl] = useModals();

	return (
		<Modal class="unpadded">
			<div class="modal-channel-topic">
				<header class="header">
					<h3 class="channel-name">#{channel()?.name}</h3>
					<div class="spacer"></div>
					<button
						type="button"
						class="icon-button"
						tabindex={0}
						onClick={modalctl.close}
						title="close modal"
					>
						<Icon src={icClose} color={null} />
					</button>
				</header>
				<Markdown
					content={channel()?.description ?? ""}
					class="channel-topic"
				/>
			</div>
		</Modal>
	);
};
