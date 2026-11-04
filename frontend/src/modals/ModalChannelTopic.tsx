import { Show } from "solid-js";
import { useChannels } from "@/api";
import icClose from "@/assets/x.png"; // TODO: random icons
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown";
import { useModals } from "@/contexts/modal";
import {
	channelDescription,
	channelDisplayName,
	isAutoDescription,
} from "@/utils/channel";
import { Modal } from "./mod";

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
					<h3 class="channel-name">
						#{channel() ? channelDisplayName(channel()!) : ""}
					</h3>
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
				<Show when={channel() ? channelDescription(channel()!) : null}>
					{(desc) => (
						<Markdown
							class="channel-topic"
							content={desc()}
							classList={{
								"description-automatic": channel()
									? isAutoDescription(channel()!)
									: false,
							}}
						/>
					)}
				</Show>
			</div>
		</Modal>
	);
};
