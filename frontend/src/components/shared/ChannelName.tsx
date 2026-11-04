import type { Channel } from "sdk";
import type { JSX } from "solid-js";
import { createMemo } from "solid-js";
import { useCurrentUser } from "@/contexts/currentUser";
import { channelDisplayName } from "@/utils/channel";

export function ChannelName(
	props: {
		channel: Channel;
		class?: string;
	} & JSX.HTMLAttributes<HTMLSpanElement>,
) {
	const currentUser = useCurrentUser();

	const name = createMemo(() => {
		const ch = props.channel;
		if (ch.type === "Dm") {
			const userId = currentUser()?.id;
			return ch.recipients?.find((i) => i.id !== userId)?.name ?? "dm";
		}
		return channelDisplayName(ch);
	});

	const isAutoName = () =>
		props.channel.name === "thread" && !!props.channel.auto_name;

	return (
		<span
			{...props}
			class={`channel-name ${props.class ?? ""}`}
			classList={{ "name-automatic": isAutoName() }}
		>
			{name()}
		</span>
	);
}
