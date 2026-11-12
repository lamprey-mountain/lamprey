import type { Room } from "sdk";
import type { VoidProps } from "solid-js";
import { WebhooksView } from "../webhooks/Webhooks";

export function Webhooks(props: VoidProps<{ room: Room }>) {
	return <WebhooksView scope={{ type: "room", room_id: props.room.id }} />;
}
