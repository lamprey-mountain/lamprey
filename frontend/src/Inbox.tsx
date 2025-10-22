import { createSignal, For, Show } from "solid-js";
import { useApi } from "./api.tsx";
import type { Channel, Message, Notification, Room } from "sdk";
import { A } from "@solidjs/router";
import { Time } from "./Time.tsx";
import { MessageView } from "./Message.tsx";
import type { NotificationPagination } from "./api/inbox.ts";

export const Inbox = () => {
	const api = useApi();
	const [params, setParams] = createSignal({
		include_read: false,
		room_id: [],
		thread_id: [],
	});
	const [inboxItems, { refetch }] = api.inbox.list(params);
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
		refetch();
	};

	const handleMarkSelectedUnread = async () => {
		if (selected().length === 0) return;
		await api.inbox.markUnread(getMessageIdsFromNotifIds(selected()));
		setSelected([]);
		refetch();
	};

	const toggleSelection = (notifId: string, isSelected: boolean) => {
		setSelected((s) =>
			isSelected ? [...s, notifId] : s.filter((id) => id !== notifId)
		);
	};

	const toggleSelectAll = (e: Event) => {
		const checked = (e.currentTarget as HTMLInputElement).checked;
		if (checked) {
			setSelected(inboxItems()?.items.map((i) => i.id) ?? []);
		} else {
			setSelected([]);
		}
	};

	return (
		<div class="inbox">
			<header>
				<h2>inbox</h2>
				<div class="spacer" />
				<div class="filters">
					<label>
						<input
							type="checkbox"
							checked={params().include_read}
							onChange={(e) =>
								setParams({
									...params(),
									include_read: e.currentTarget.checked,
								})}
						/>
						include read
					</label>
				</div>
			</header>
			<div style="margin:8px;margin-bottom:0;margin-left: 16px;height:1rem;display:flex;align-items:center">
				<label>
					<input
						type="checkbox"
						onChange={toggleSelectAll}
						style="margin-right:8px"
					/>
					select all
				</label>
				<Show when={selected().length > 0}>
					<div style="margin-left: 8px">
						<span>{selected().length} selected</span>
						<button onClick={handleMarkSelectedRead}>Mark as read</button>
						<button onClick={handleMarkSelectedUnread}>Mark as unread</button>
					</div>
				</Show>
			</div>
			<div class="inner">
				<For each={inboxItems()?.items} fallback={<div>loading...</div>}>
					{(it) => (
						<NotificationItem
							notification={it}
							allData={inboxItems()}
							selected={selected().includes(it.id)}
							onSelect={toggleSelection}
							refetch={refetch}
							include_read={params().include_read}
						/>
					)}
				</For>
			</div>
		</div>
	);
};

const NotificationItem = (
	props: {
		notification: Notification;
		allData: NotificationPagination | undefined;
		selected: boolean;
		onSelect: (id: string, selected: boolean) => void;
		refetch: () => void;
		include_read: boolean;
	},
) => {
	const api = useApi();
	const thread = () =>
		props.allData?.threads.find((t) => t.id === props.notification.thread_id);
	const message = () =>
		props.allData?.messages.find((m) => m.id === props.notification.message_id);
	const room = () => {
		const t = thread();
		if (!t?.room_id) return;
		return props.allData?.rooms.find((r) => r.id === t.room_id);
	};

	const handleMarkRead = async () => {
		await api.inbox.markRead([props.notification.message_id]);
		props.refetch();
	};

	const handleMarkUnread = async () => {
		await api.inbox.markUnread([props.notification.message_id]);
		props.refetch();
	};

	const reasonText = () => {
		switch (props.notification.reason) {
			case "Mention":
				return "Mention";
			case "MentionBulk":
				return "Room Mention";
			case "Reminder":
				return "Reminder";
			case "Reply":
				return "Reply";
		}
	};

	return (
		<article class="notification" data-type={props.notification.reason}>
			<header>
				<input
					type="checkbox"
					class="notification-checkbox"
					checked={props.selected}
					onChange={(e) =>
						props.onSelect(props.notification.id, e.currentTarget.checked)}
				/>
				<Show when={room()}>
					<A href={`/room/${room()!.id}`}>{room()!.name}</A>
					&nbsp;&gt;&nbsp;
				</Show>
				<A href={`/thread/${thread()?.id}`}>{thread()?.name ?? "..."}</A>
				&nbsp;&bull;&nbsp;
				<Time date={new Date(props.notification.added_at)} />
				<div class="spacer"></div>
				<div class="label">{reasonText()}</div>
				<Show
					when={!props.notification.read_at}
					fallback={
						<button class="mark-read" onClick={handleMarkUnread}>
							Mark as unread
						</button>
					}
				>
					<button class="mark-read" onClick={handleMarkRead}>
						Mark as read
					</button>
				</Show>
			</header>
			<div class="notification-content">
				<A
					class="body-link"
					href={`/thread/${thread()?.id}/message/${message()?.id}`}
				>
					<Show when={message()}>
						<MessageView message={message() as Message} separate={true} />
					</Show>
				</A>
			</div>
		</article>
	);
};
