// import { Tooltip } from "./Atoms.tsx";
const Tooltip = (props) => props.children;
import {
	createEffect,
	createSignal,
	lazy,
	Match,
	onMount,
	Switch,
} from "solid-js";

type UserProps = {
	name: string;
};

const User = (props: UserProps) => {
	return (
		<div>
			<h3>{props.name}</h3>
			<p>some info here</p>
			<p>more stuff</p>
			<p>click to view full profile</p>
		</div>
	);
};

type Message = any;

type MessageProps = {
	message: Message;
};

type MessagesProps = {
	messages: Array<Message>;
	notime?: boolean;
};

export const Message = (props: MessageProps) => {
	let bodyEl: HTMLSpanElement;

	createEffect(async () => {
		props.message.body; // make it react
		// FIXME: flash of unhighlighted code on update
		const hljs = await import("highlight.js");
		for (const code of bodyEl.querySelectorAll("code[class*=language-]")) {
			hljs.default.highlightElement(code);
		}
	});

	return (
		<div class="messageWrap">
			<span class="sender">
				<Tooltip
					tip={() => <User name={props.message.sender} />}
					attrs={{ class: "" }}
				>
					{props.message.sender}
				</Tooltip>
			</span>
			{props.message.type === "message_html"
				? (
					<span
						class="body"
						ref={bodyEl!}
						innerHTML={props.message.body}
					>
					</span>
				)
				: <span class="body" ref={bodyEl!}>{props.message.body}</span>}
			<span class="time">
				{new Date(props.message.origin_ts).toDateString()}
			</span>
		</div>
	);
};

export const TimelineItem = (props: { msg: Message }) => {
	return (
		<Switch>
			<Match when={props.msg.type === "message" || props.msg.type === "message_html"}>
				<li
					classList={{
						message: true,
						unread: props.msg.unread,
						mention: props.msg.mention,
						is_local: props.msg.is_local,
					}}
				>
					<Message message={props.msg} />
				</li>
			</Match>
			<Match when={props.msg.type === "unread-marker" && false}>
				<li
					classList={{
						message: true,
						unreadMarker: true,
					}}
				>
					<div class="messageWrap">
						<span class="sender">-----</span>
						<span class="body">new messages</span>
					</div>
				</li>
			</Match>
			<Match when={props.msg.type === "unread-marker"}>
				<li classList={{ unreadMarker2: true }}>
					<hr />
					<span>unread messages</span>
					<hr />
				</li>
			</Match>
			<Match when={props.msg.type === "time-split" && false}>
				<li
					classList={{
						message: true,
						timeSplit: true,
					}}
				>
					<div class="messageWrap">
						<span class="sender">-----</span>
						<span class="body">
							time changed to{" "}
							<time>{new Date(props.msg.origin_ts).toDateString()}</time>
						</span>
					</div>
				</li>
			</Match>
			<Match when={props.msg.type === "time-split"}>
				<li
					classList={{
						timeSplit2: true,
					}}
				>
					<hr />
					<time>{new Date(props.msg.origin_ts).toDateString()}</time>
					<hr />
				</li>
			</Match>
		</Switch>
	)
}

export const Messages = (props: MessagesProps) => {
	return (
		<>
			<ul class="messages" classList={{ "notime": props.notime }}>
				{props.messages.map((i) => <TimelineItem msg={i} />)}
			</ul>
		</>
	);
};
