// import { Tooltip } from "./Atoms.tsx";
const Tooltip = (props) => props.children;
import * as sdk from "sdk";
import {
	createEffect,
	createSignal,
	lazy,
	Match,
	onMount,
	Switch,
} from "solid-js";
import { TimelineItemT } from "./list.tsx";

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

type MessageProps = {
	message: sdk.Message;
};

export const Message = (props: MessageProps) => {
	let bodyEl: HTMLSpanElement;

	createEffect(async () => {
		props.message.data; // make it react
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
					tip={() => <User name={props.message.data.author_id} />}
					attrs={{ class: "" }}
				>
					{props.message.data.author_id}
				</Tooltip>
			</span>
			<span class={BODY_CSS} ref={bodyEl!}>{props.message.data.content}</span>
			<span class="invisible group-hover:visible text-fg4">
				{(/^[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}$/.test(props.message.id) ? sdk.getTimestampFromUUID(props.message.id) : new Date).toDateString()}
			</span>
		</div>
	);
};

export const TimelineItem = (props: { item: TimelineItemT }) => {
	// console.log(props.item)
	return (
		<Switch>
			<Match when={props.item.type === "message"}>
				<li
					class=""
					classList={{
						unread: props.item.message.unread,
						"bg-[#67dc8222]": props.item.message.mention,
						"shadow-arst": props.item.message.mention || props.item.message.unread,
						"shadow-[#67dc82]": props.item.message.mention,
						"shadow-[#3fa9c9]": props.item.message.unread,
						"text-fg4": props.item.message.is_local,
					}}
				>
					<Message message={props.item.message} />
				</li>
			</Match>
			<Match when={props.item.type === "spacer"}>
				<li class="flex-1"><div class="h-24"></div></li>
			</Match>
			<Match when={props.item.type === "spacer-mini"}>
				<li><div class="h-6"></div></li>
			</Match>
		</Switch>
	)

	// <Match when={props.item.type === "unread-marker" && false}>
	// 	<li class="text-[#3fa9c9] shadow-arst shadow-[#3fa9c9] shadow-[#3fa9c922]">
	// 		<div class="grid grid-cols-[128px_auto_max-content] px-[8px]">
	// 			<span class="sender">-----</span>
	// 			<span class="body">new messages</span>
	// 		</div>
	// 	</li>
	// </Match>
	// <Match when={props.item.type === "unread-marker"}>
	// 	<li classList={{ unreadMarker2: true }}>
	// 		<hr />
	// 		<span>unread messages</span>
	// 		<hr />
	// 	</li>
	// </Match>
	// <Match when={props.item.type === "time-split" && false}>
	// 	<li
	// 		classList={{
	// 			message: true,
	// 			timeSplit: true,
	// 		}}
	// 	>
	// 		<div class="grid grid-cols-[128px_auto_max-content] px-[8px]">
	// 			<span class="sender">-----</span>
	// 			<span class="body">
	// 				time changed to{" "}
	// 				<time>{new Date(props.msg.origin_ts).toDateString()}</time>
	// 			</span>
	// 		</div>
	// 	</li>
	// </Match>
	// <Match when={props.item.type === "time-split"}>
	// 	<li
	// 		classList={{
	// 			timeSplit2: true,
	// 		}}
	// 	>
	// 		<hr />
	// 		<time>{new Date(props.msg.origin_ts).toDateString()}</time>
	// 		<hr />
	// 	</li>
	// </Match>
}

// export const Messages = (props: MessagesProps) => {
// 	return (
// 		<>
// 			<ul class="flex flex-col justify-end py-[8px]" classList={{ "notime": props.notime }}>
// 				{props.messages.map((i) => <TimelineItem msg={i} />)}
// 			</ul>
// 		</>
// 	);
// };

// {props.message.type === "message_html"
// 	? (
// 		<span
// 			class={BODY_CSS}
// 			ref={bodyEl!}
// 			innerHTML={props.message.body}
// 		>
// 		</span>
// 	)
// 	: <span class={BODY_CSS} ref={bodyEl!}>{props.message.body}</span>}
