// import { Tooltip } from "./Atoms.tsx";
import { getTimestampFromUUID } from "sdk";
import {
	createEffect,
	createSignal,
	lazy,
	Match,
	onMount,
	ParentProps,
	Switch,
} from "solid-js";
import { TimelineItemT } from "./list.tsx";
import { MessageT } from "./types.ts";

const Tooltip = (props: ParentProps<{ tip: any, attrs: any }>) => props.children;

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
	message: MessageT;
};

export const Message = (props: MessageProps) => {
	let bodyEl: HTMLSpanElement;

	// createEffect(async () => {
	// 	props.message; // make it react
	// 	// FIXME: flash of unhighlighted code on update
	// 	const hljs = await import("highlight.js");
	// 	for (const code of bodyEl.querySelectorAll("code[class*=language-]")) {
	// 		hljs.default.highlightElement(code);
	// 	}
	// });

	return (
		<div class={WRAPPER_CSS}>
			<span class="hover:underline cursor-pointer truncate text-right text-fg4">
				<Tooltip
					tip={() => <User name={props.message.author.name} />}
					attrs={{ class: "" }}
				>
					{props.message.author.name}
				</Tooltip>
			</span>
			<span class={BODY_CSS} ref={bodyEl!}>{props.message.content}</span>
			<span class="invisible group-hover:visible text-fg4">
				{(/^[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}$/.test(props.message.id) ? getTimestampFromUUID(props.message.id) : new Date).toDateString()}
			</span>
		</div>
	);
};

function getTimelineItem(item: TimelineItemT) {
	switch(item.type) {
		case "message": {
			// unread: item.message.unread,
			// "bg-[#67dc8222]": item.message.mention,
			// "shadow-arst": item.message.mention || item.message.unread,
			// "shadow-[#67dc82]": item.message.mention,
			// "shadow-[#3fa9c9]": item.message.unread,
			// "text-fg4": item.message.is_local,
			return (
				<li classList={{ }}>
					<Message message={item.message} />
				</li>
			);
		}
		case "info": {
					// <header class="sticky top-[0] px-[144px] bg-bg3 mb-[8px] border-b-[1px] border-b-sep mt-[-8px]">
					// <header class="shadow-foo shadow-[#0009] bg-bg1 p-2 text-cente">
					// 	<p>more info here</p>
					// </header>
			return (
				<li class="contents">
					<header class="px-[144px] bg-bg3 mb-4">
						<h1 class="text-xl">header</h1>
						<p>more info here</p>
					</header>
				</li>
			)
		}
		case "spacer": {
			return <li class="flex-1"><div class="h-[800px]"></div></li>
		}
		case "spacer-mini": {
			return <li><div class="h-8"></div></li>
		}
	}
}

export const TimelineItem = (props: { item: TimelineItemT }) => {
	return (<>{getTimelineItem(props.item)}</>);

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
