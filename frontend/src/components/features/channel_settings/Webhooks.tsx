import type { Channel } from "sdk";
import type { VoidProps } from "solid-js";
import { WebhooksView } from "../webhooks/Webhooks";

export function Webhooks(props: VoidProps<{ channel: Channel }>) {
	return (
		<WebhooksView scope={{ type: "channel", channel_id: props.channel.id }} />
	);
}
