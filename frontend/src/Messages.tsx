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

const WRAPPER_CSS = "group grid grid-cols-[128px_auto_max-content] px-[8px] hover:bg-bg1/30";
const BODY_CSS = "mx-[8px] overflow-hidden whitespace-pre-wrap";

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
		<div class={WRAPPER_CSS}>
			<span class="hover:underline cursor-pointer truncate text-right text-fg4">
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
						class={BODY_CSS}
						ref={bodyEl!}
						innerHTML={props.message.body}
					>
					</span>
				)
				: <span class={BODY_CSS} ref={bodyEl!}>{props.message.body}</span>}
			<span class="invisible group-hover:visible text-fg4">
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
					class=""
					classList={{
						unread: props.msg.unread,
						"bg-[#67dc8222]": props.msg.mention,
						"shadow-arst": props.msg.mention || props.msg.unread,
						"shadow-[#67dc82]": props.msg.mention,
						"shadow-[#3fa9c9]": props.msg.unread,
						"text-fg4": props.msg.is_local,
					}}
				>
					<Message message={props.msg} />
				</li>
			</Match>
			<Match when={props.msg.type === "unread-marker" && false}>
				<li class="text-[#3fa9c9] shadow-arst shadow-[#3fa9c9] shadow-[#3fa9c922]">
					<div class="grid grid-cols-[128px_auto_max-content] px-[8px]">
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
					<div class="grid grid-cols-[128px_auto_max-content] px-[8px]">
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
			<ul class="flex flex-col justify-end py-[8px]" classList={{ "notime": props.notime }}>
				{props.messages.map((i) => <TimelineItem msg={i} />)}
			</ul>
		</>
	);
};
