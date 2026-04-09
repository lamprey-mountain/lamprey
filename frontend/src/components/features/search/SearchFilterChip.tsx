import type { User } from "sdk";
import { Show } from "solid-js";
import { ChannelIcon } from "@/avatar/ChannelIcon";
import { Avatar } from "@/avatar/UserAvatar";
import type { ThreadT } from "@/types";

export const FilterChipUI = (props: {
	type: string;
	label: string;
	user?: User;
	channel?: ThreadT;
	negated?: boolean;
	animate?: boolean;
}) => (
	<span
		class={`filter-${props.type} filter-atom`}
		classList={{ "filter-negated": props.negated }}
	>
		<span class="filter-prefix">
			{props.negated ? "-" : ""}
			{props.type}:
		</span>
		<Show when={props.user}>
			{(u) => (
				<Avatar
					user={u()}
					animate={props.animate}
					style="width: 14px; height: 14px; margin-left: 4px; margin-right: 2px; flex: none;"
				/>
			)}
		</Show>
		<Show when={props.channel}>
			{(c) => (
				<ChannelIcon
					channel={c()}
					animate={props.animate}
					style="width: 14px; height: 14px; margin-left: 4px; margin-right: 2px; flex: none;"
				/>
			)}
		</Show>
		<span class="filter-value">{props.label}</span>
	</span>
);
