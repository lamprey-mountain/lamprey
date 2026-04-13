import type { User } from "sdk";
import { createMemo, Show } from "solid-js";
import {
	useChannels,
	useRoles,
	useRoomMembers,
	useThreadMembers,
	useUsers,
} from "@/api";
import { ChannelIcon } from "@/avatar/ChannelIcon";
import { Avatar } from "@/avatar/UserAvatar";
import type { ThreadT } from "@/types";
import { SEARCH_FILTERS, type SearchContext } from "./filters.config";

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

export const FilterChip = (props: {
	type: string;
	id: string;
	name?: string;
	negated?: boolean;
	animate?: boolean;
	roomId: string | null;
	channelId?: string;
}) => {
	const users = useUsers();
	const channels = useChannels();
	const roomMembers = useRoomMembers();
	const threadMembers = useThreadMembers();
	const roles = useRoles();

	const userResource = users.use(() =>
		props.type === "author"
			? props.id
			: props.type === "mentions" && props.id.startsWith("user-")
				? props.id.replace("user-", "")
				: undefined,
	);
	const channelResource = channels.use(() =>
		props.type === "channel" ? props.id : undefined,
	);

	const searchContext = createMemo<SearchContext>(() => ({
		users,
		channels,
		roomMembers,
		threadMembers,
		roles,
		roomThreads: () => {
			const rId = props.roomId;
			if (!rId) return [];
			return [...channels.cache.values()].filter(
				(c) => c.room_id === rId,
			) as ThreadT[];
		},
		roomId: props.roomId,
	}));

	const resolved = createMemo(() => {
		const def = SEARCH_FILTERS[props.type];
		const id = props.id;

		if (def?.resolveDisplayData) {
			const ctxResolved = def.resolveDisplayData(id, searchContext());
			const user = userResource();
			if (props.type === "author" && user) {
				return {
					...ctxResolved,
					user,
					name: user.name,
				};
			}
			const channel = channelResource();
			if (props.type === "channel" && channel) {
				return {
					...ctxResolved,
					channel,
					name: channel.name,
				};
			}
			if (props.type === "mentions" && props.id.startsWith("user-") && user) {
				return {
					...ctxResolved,
					user,
					name: user.name,
				};
			}
			return ctxResolved;
		}

		const user = userResource();
		if (props.type === "author" && user) {
			return { name: user.name, user };
		}
		const channel = channelResource();
		if (props.type === "channel" && channel) {
			return { name: channel.name, channel: channel as ThreadT };
		}
		if (props.type === "mentions" && props.id.startsWith("user-") && user) {
			return { name: user.name, user };
		}

		return { name: props.name ?? id };
	});

	return (
		<FilterChipUI
			type={props.type}
			label={resolved().name ?? props.name ?? props.id}
			user={resolved().user}
			channel={resolved().channel}
			negated={props.negated}
			animate={props.animate}
		/>
	);
};
