import { A } from "@solidjs/router";
import type { Message, Notification } from "sdk";
import { createMemo, createSignal, For, Match, Show, Switch } from "solid-js";
import { useApi } from "@/api";
import type { NotificationPagination } from "@/api/services/InboxService.ts";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Icon } from "@/atoms/Icon";
import { Checkbox } from "@/atoms/icons";
import { Time } from "@/atoms/Time";
import { MessageView } from "@/components/features/chat/Message.tsx";
import { getDate } from "@/utils/general";
import { icCheck, icQuestion } from "@/utils/icons";
import { MessageToolbarProvider } from "../features/chat/message-toolbar-context";
import { ChannelIcon, RoomIcon } from "./User";

// TODO: skeletons for inbox items
// TODO: render other notification items besides messages
// TODO: refactor styles
// TODO: better caching, update inbox as sync messages are received, don't refetch when marking messages as read/unread

export const Inbox = () => {
	const api = useApi();
	const [params, setParams] = createSignal({
		include_read: false,
		room_id: [],
		thread_id: [],
	});
	const inboxResult = api.inbox.useList(params);
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
		await api.inbox.markRead(getMessageIdsFromNotifIds(selected()));
		setSelected([]);
		inboxResult.refetch();
	};

	const handleMarkSelectedUnread = async () => {
		if (selected().length === 0) return;
		await api.inbox.markUnread(getMessageIdsFromNotifIds(selected()));
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
			<MessageToolbarProvider>
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
						// TODO: checked=true when all items are selected
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
					<div style="flex:1"></div>
					<Show when={selected().length > 0}>
						<div style="margin-left: 8px;display:flex;align-items:center">
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
			</MessageToolbarProvider>
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
	const api = useApi();

	const ty = () => props.notification.type;

	const channel = () => {
		const channelId = props.notification.channel_id;
		if (!channelId) return undefined;
		return api.channels.get(channelId);
	};

	const message = createMemo(() =>
		props.allData?.messages.find(
			(m: Message) => m.id === props.notification.message_id,
		),
	);

	const room = () => {
		const t = channel();
		if (!t?.room_id) return;
		return api.rooms.get(t.room_id);
	};

	const handleMarkRead = async () => {
		await api.inbox.markRead([props.notification.id]);
		props.refetch();
	};

	const handleMarkUnread = async () => {
		await api.inbox.markUnread([props.notification.id]);
		props.refetch();
	};

	return (
		<article
			class="notification"
			data-type={ty()}
			classList={{ selected: props.selected }}
		>
			<header>
				<Switch>
					<Match when={room()}>{(room) => <RoomIcon room={room()} />}</Match>
					<Match when={channel()}>
						{(chan) => <ChannelIcon channel={chan()} />}
					</Match>
				</Switch>
				<Show when={room()}>
					<A href={`/room/${room()?.id}`}>{room()?.name}</A>
					&nbsp;&gt;&nbsp;
				</Show>
				<A href={`/channel/${channel()?.id}`}>{channel()?.name ?? "..."}</A>
				&nbsp;&bull;&nbsp;
				<Time date={getDate(props.notification.added_at)} />
				<div class="spacer"></div>
				<menu>
					<Show
						when={!props.notification.read_at}
						fallback={
							<button
								type="button"
								class="icon-button mark-read"
								onClick={handleMarkUnread}
								data-tooltip="Mark as unread"
							>
								{/* TODO: icon for this*/}
								<Icon src={icQuestion} />
							</button>
						}
					>
						<button
							type="button"
							class="icon-button mark-read"
							onClick={handleMarkRead}
							data-tooltip="Mark as read"
						>
							<Icon src={icCheck} />
						</button>
					</Show>

					<label
						class="select"
						data-tooltip={
							props.selected ? "Deselect notification" : "Select notification"
						}
					>
						<Checkbox
							checked={props.selected}
							seed={`inbox-notif-${props.notification.id}`}
						/>
						<input
							type="checkbox"
							checked={props.selected}
							onInput={(e) =>
								props.onSelect(props.notification.id, e.currentTarget.checked)
							}
							style="display:none"
						/>
					</label>
				</menu>
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
