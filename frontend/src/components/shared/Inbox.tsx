import { A } from "@solidjs/router";
import type { Message, Notification } from "sdk";
import { createSignal, For, Show } from "solid-js";
import { useChannels, useInbox, useRooms } from "@/api";
import type { NotificationPagination } from "@/api/services/InboxService.ts";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";
import { Time } from "@/atoms/Time";
import { MessageView } from "@/components/features/chat/Message.tsx";

export const Inbox = () => {
	const inbox2 = useInbox();
	const [params, setParams] = createSignal({
		include_read: false,
		room_id: [],
		thread_id: [],
	});
	const inboxResult = inbox2.useList(params);
	const inboxItems = inboxResult.resource;
	const [selected, setSelected] = createSignal<string[]>([]);

	const getMessageIdsFromNotifIds = (notifIds: string[]) => {
		const items = inboxItems()?.items ?? [];
		return notifIds
			.map((id) => {
				const notif = items.find((it) => it.id === id);
				return notif ? notif.message_id : null;
			})
			.filter((id): id is string => !!id);
	};

	const handleMarkSelectedRead = async () => {
		if (selected().length === 0) return;
		await inbox2.markRead(getMessageIdsFromNotifIds(selected()));
		setSelected([]);
		inboxResult.refetch();
	};

	const handleMarkSelectedUnread = async () => {
		if (selected().length === 0) return;
		await inbox2.markUnread(getMessageIdsFromNotifIds(selected()));
		setSelected([]);
		inboxResult.refetch();
	};

	const toggleSelection = (notifId: string, isSelected: boolean) => {
		setSelected((s) =>
			isSelected ? [...s, notifId] : s.filter((id) => id !== notifId),
		);
	};

	return (
		<div class="inbox">
			<header>
				<h2>inbox</h2>
				<div class="spacer" />
				<div class="filters">
					<CheckboxOption
						id="inbox-include-read"
						checked={params().include_read}
						onChange={(checked) =>
							setParams({
								...params(),
								include_read: checked,
							})
						}
						seed="inbox-include-read"
					>
						<Checkbox
							checked={params().include_read}
							seed="inbox-include-read"
						/>
						<span>include read</span>
					</CheckboxOption>
				</div>
			</header>
			<div style="margin:8px;margin-bottom:0;margin-left: 16px;height:1rem;display:flex;align-items:center">
				<CheckboxOption
					id="inbox-select-all"
					checked={false}
					onChange={(checked) => {
						if (checked) {
							setSelected(inboxItems()?.items.map((i) => i.id) ?? []);
						} else {
							setSelected([]);
						}
					}}
					seed="inbox-select-all"
				>
					<Checkbox checked={false} seed="inbox-select-all" />
					<span>select all</span>
				</CheckboxOption>
				<Show when={selected().length > 0}>
					<div style="margin-left: 8px">
						<span>{selected().length} selected</span>
						<button
							type="button"
							class="button"
							onClick={handleMarkSelectedRead}
						>
							Mark as read
						</button>
						<button
							type="button"
							class="button"
							onClick={handleMarkSelectedUnread}
						>
							Mark as unread
						</button>
					</div>
				</Show>
			</div>
			<div class="inner">
				<Show when={!inboxItems.loading} fallback={<div>loading...</div>}>
					<For each={inboxItems()?.items} fallback={<div>no entries</div>}>
						{(it) => (
							<NotificationItem
								notification={it}
								allData={inboxItems()}
								selected={selected().includes(it.id)}
								onSelect={toggleSelection}
								refetch={inboxResult.refetch}
								include_read={params().include_read}
							/>
						)}
					</For>
				</Show>
			</div>
		</div>
	);
};

const NotificationItem = (props: {
	notification: Notification;
	allData: NotificationPagination | undefined;
	selected: boolean;
	onSelect: (id: string, selected: boolean) => void;
	refetch: () => void;
	include_read: boolean;
}) => {
	const inbox = useInbox();
	const channels = useChannels();
	const rooms = useRooms();

	const ty = () => props.notification.type;

	const channel = () => {
		const channelId = props.notification.channel_id;
		if (!channelId) return undefined;
		return channels.get(channelId);
	};

	const message = () =>
		props.allData?.messages.find(
			(m: Message) => m.id === props.notification.message_id,
		);

	const room = () => {
		const t = channel();
		if (!t?.room_id) return;
		return rooms.get(t.room_id);
	};

	const handleMarkRead = async () => {
		await inbox.markRead([props.notification.message_id]);
		props.refetch();
	};

	const handleMarkUnread = async () => {
		await inbox.markUnread([props.notification.message_id]);
		props.refetch();
	};

	return (
		<article class="notification" data-type={ty()}>
			<header>
				<CheckboxOption
					id={`inbox-notif-${props.notification.id}`}
					checked={props.selected}
					onChange={(checked) => props.onSelect(props.notification.id, checked)}
					seed={`inbox-notif-${props.notification.id}`}
					class="notification-checkbox"
				>
					<Checkbox
						checked={props.selected}
						seed={`inbox-notif-${props.notification.id}`}
					/>
				</CheckboxOption>
				<Show when={room()}>
					<A href={`/room/${room()?.id}`}>{room()?.name}</A>
					&nbsp;&gt;&nbsp;
				</Show>
				<A href={`/channel/${channel()?.id}`}>{channel()?.name ?? "..."}</A>
				&nbsp;&bull;&nbsp;
				<Time date={new Date(props.notification.added_at)} />
				<div class="spacer"></div>
				<div class="label">{ty()}</div>
				<Show
					when={!props.notification.read_at}
					fallback={
						<button type="button" class="mark-read" onClick={handleMarkUnread}>
							Mark as unread
						</button>
					}
				>
					<button type="button" class="mark-read" onClick={handleMarkRead}>
						Mark as read
					</button>
				</Show>
			</header>
			<div class="notification-content">
				<A
					class="body-link"
					href={`/channel/${channel()?.id}/message/${message()?.id}`}
				>
					<Show when={message()}>
						{(msg) => <MessageView message={msg()} separate={true} />}
					</Show>
				</A>
			</div>
		</article>
	);
};
