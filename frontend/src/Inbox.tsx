import { For, Match, Switch } from "solid-js";

// start with just mention and reminder for now
const inboxItems = [
	{ type: "mention" },
	{ type: "mention" },
	{ type: "reply" },
	{ type: "new_thread" },
	{ type: "unread" },
	{ type: "mention" },
	{ type: "reminder" },
	{ type: "mention" },
	{ type: "mention" },
	{ type: "friend_request_received" },
	{ type: "dm_request" },
	{ type: "mention" },
	{ type: "mention" },
	{ type: "mention" },
	{ type: "friend_request_accepted" },
];

export const Inbox = () => {
	return (
		<div class="inbox" style="">
			<h2>inbox</h2>
			<p>filter by room, show reminders</p>
			<div class="inner">
				<For each={inboxItems}>
					{(it) => {
						return (
							<article class="notification" data-type={it.type}>
								<header>
									<Switch>
										<Match when={it.type === "friend_request_received"}>
											new friend?
										</Match>
										<Match when={it.type === "friend_request_accepted"}>
											new friend!
										</Match>
										<Match when={true}>room name &gt; thread name</Match>
									</Switch>{" "}
									&bull; some time ago
									<div class="spacer"></div>
									<div class="label">{it.type}</div>
								</header>
								<Switch>
									<Match when={it.type === "mention"}>
										<NotificationMention />
									</Match>
									<Match when={it.type === "reply"}>
										<NotificationReply />
									</Match>
									<Match when={it.type === "new_thread"}>
										<NotificationNewThread />
									</Match>
									<Match when={it.type === "unread"}>
										<NotificationUnreadThread />
									</Match>
									<Match when={it.type === "reminder"}>
										<NotificationReminder />
									</Match>
									<Match when={it.type === "friend_request_received"}>
										<NotificationFriendRequestRecieved />
									</Match>
									<Match when={it.type === "dm_request"}>
										<NotificationDmRequest />
									</Match>
									<Match when={it.type === "friend_request_accepted"}>
										<NotificationFriendRequestAccepted />
									</Match>
								</Switch>
							</article>
						);
					}}
				</For>
			</div>
		</div>
	);
};

const NotificationReminder = () => {
	return (
		<div style="padding:8px">
			message, mention, etc
			<br />
			if theres just a few messages, show them all here
			<br />
			<button>jump</button>
			<button>close</button>
		</div>
	);
};

const NotificationUnreadThread = () => {
	return (
		<div style="padding:8px">
			message, mention, etc
			<br />
			if theres just a few messages, show them all here
			<br />
			<button>jump</button>
			<button>close</button>
		</div>
	);
};

const NotificationNewThread = () => {
	return (
		<div style="padding:8px">
			message, mention, etc
			<br />
			if theres just a few messages, show them all here
			<br />
			<button>jump</button>
			<button>close</button>
		</div>
	);
};

const NotificationReply = () => {
	return (
		<div style="padding:8px">
			message, mention, etc
			<br />
			if theres just a few messages, show them all here
			<br />
			<button>jump</button>
			<button>close</button>
		</div>
	);
};

const NotificationMention = () => {
	return (
		<div style="padding:8px">
			message, mention, etc
			<br />
			if theres just a few messages, show them all here
			<br />
			<button>jump</button>
			<button>close</button>
		</div>
	);
};

const NotificationFriendRequestAccepted = () => {
	return (
		<div style="padding:8px">
			show user profile
			<br />
			<button>send dm</button>
			<button>hide notification</button>
		</div>
	);
};

const NotificationFriendRequestRecieved = () => {
	return (
		<div style="padding:8px">
			show user profile
			<br />
			<button>accept</button>
			<button>reject</button>
			<button>hide notification</button>
		</div>
	);
};

const NotificationDmRequest = () => {
	return (
		<div style="padding:8px">
			show user profile
			<br />
			<button>accept</button>
			<button>reject</button>
			<button>hide notification</button>
		</div>
	);
};

/*
notification types

- dm_request (merge with new_thread?)
- friend_request_received
- friend_request_accepted
- mention
- new_thread
- reminder
- reply
- unread

notification behavior

- merge (new_thread, mention, reply, unread) -> unread if messages are near each other
- show all of a thread's unread messages in inbox if unread count < ~30
- try to show full message group if possible
- automatically mark notifications as read as you scroll through them (in inbox or thread)
- option to delete ("close") individual notifications
- option to include read notifications
- option to include future reminders
- option to filter by room
- accepting friend/dm request also deletes the notification

where should the timestamp go? for messages its redundant

messages
type Notification = { id, thread_id, messages: Array<message>, flags }
flags contain reply, mention, new_thread

reminder
type Notification = { id, thread_id, message_id, reason? }

friend request
type Notification = { id, user_id }

dm request
type Notification = { id, user_id, thread_id }
*/
