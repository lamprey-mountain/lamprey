// import { Tooltip } from "./Atoms.tsx";
import { getTimestampFromUUID } from "sdk";
import { For, Show } from "solid-js";
import { AttachmentT, MessageT, MessageType, ThreadT, UserT } from "./types.ts";
import { marked } from "marked";
// @ts-types="npm:@types/sanitize-html@^2.13.0"
import sanitizeHtml from "npm:sanitize-html";
import { useCtx } from "./context.ts";

// const Tooltip = (props: ParentProps<{ tip: any, attrs: any }>) => props.children;

export type TimelineItemT =
	& { id: string; class?: string }
	& (
		| { type: "info"; header: boolean }
		| { type: "editor" }
		| { type: "spacer" }
		| { type: "spacer-mini" }
		| { type: "spacer-mini2" }
		| { type: "unread-marker" }
		| { type: "time-split" }
		| { type: "anchor" }
		| {
			type: "message";
			message: MessageT;
			separate: boolean;
			is_local: boolean;
		}
	);

const sanitizeHtmlOptions: sanitizeHtml.IOptions = {
	transformTags: {
		del: "s",
	},
};

type UserPopupProps = {
	user: UserT;
};

// const UserTooltip = (props: UserPopupProps) => {
// 	// TODO: click to view full profile
// 	return (
// 		<div class="user">
// 			<h3>{props.user.name}</h3>
// 			<Show when={props.user.description} fallback={<p><em>no description</em></p>}>
// 				<p>{props.user.description}</p>
// 			</Show>
// 			<code>{props.user.id}</code>
// 		</div>
// 	);
// };

type MessageProps = {
	message: MessageT;
	is_local: boolean;
};

const md = marked.use({
	breaks: true,
	gfm: true,
});

export function getAttachment(a: AttachmentT) {
	const b = a.mime.split("/")[0];
	const byteFmt = Intl.NumberFormat("en", {
		notation: "compact",
		style: "unit",
		unit: "byte",
		unitDisplay: "narrow",
	});

	const [ty] = a.mime.split(";");
	// const [ty, paramsRaw] = a.mime.split(";");
	// const params = new Map(paramsRaw?.split(" ").map(i => i.trim().split("=") as [string, string]));
	// console.log({ ty, params });

	if (b === "image") {
		// <div class="spacer" style={{ height: `${a.height}px`, width: `${a.width}px` }}></div>
		return (
			<li>
				<div
					class="media"
					style={{ "aspect-ratio": `${a.width} / ${a.height}` }}
				>
					<img
						src={a.url}
						alt={a.alt ?? undefined}
						style={{ height: `${a.height}px`, width: `${a.width}px` }}
					/>
				</div>
				<a download={a.filename} href={a.url}>download {a.filename}</a>
				<div class="dim">{ty} - {byteFmt.format(a.size)}</div>
			</li>
		);
	} else if (b === "video") {
		return (
			<li>
				<div
					class="media"
					style={{ "aspect-ratio": `${a.width} / ${a.height}` }}
				>
					<div class="spacer"></div>
					<video height={a.height!} width={a.width!} src={a.url} controls />
				</div>
				<a download={a.filename} href={a.url}>download {a.filename}</a>
				<div class="dim">{ty} - {byteFmt.format(a.size)}</div>
			</li>
		);
	} else if (b === "audio") {
		return (
			<li>
				<audio src={a.url} controls />
				<a download={a.filename} href={a.url}>download {a.filename}</a>
				<div class="dim">{ty} - {byteFmt.format(a.size)}</div>
			</li>
		);
	} else {
		return (
			<li>
				<a download={a.filename} href={a.url}>download {a.filename}</a>
				<div class="dim">{ty} - {byteFmt.format(a.size)}</div>
			</li>
		);
	}
}

function Reply(props: { reply: MessageT }) {
	const name = props.reply.override_name ?? props.reply.author.name;
	const content = props.reply.content ??
		`${props.reply.attachments.length} attachment(s)`;
	return (
		<>
			<div class="reply arrow">{"\u21B1"}</div>
			<div class="reply reply-content">
				<span class="author">{name}:</span>
				{content}
			</div>
			<div class="reply"></div>
		</>
	);
}

export const Message = (props: MessageProps) => {
	const ctx = useCtx();
	let bodyEl: HTMLDivElement;

	// createEffect(async () => {
	// 	props.message; // make it react
	// 	// FIXME: flash of unhighlighted code on update
	// 	const hljs = await import("highlight.js");
	// 	for (const code of bodyEl.querySelectorAll("code[class*=language-]")) {
	// 		hljs.default.highlightElement(code);
	// 	}
	// });

	function getComponent() {
		const date =
			/^[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}$/.test(
					props.message.id,
				)
				? getTimestampFromUUID(props.message.id)
				: new Date();
		const authorName = props.message.override_name ?? props.message.author.name;
		if (props.message.type === MessageType.ThreadUpdate) {
			const updates = [];
			const listFormatter = new Intl.ListFormat();
			const patch = props.message.metadata as any;
			if (patch.name) updates.push(`set name to ${patch.name}`);
			if (patch.description) {
				updates.push(
					patch.description ? `set description to ${patch.description}` : "",
				);
			}
			if (patch.is_locked) {
				updates.push(patch.is_locked ? "locked thread" : "unlocked thread");
			}
			if (patch.is_closed) {
				updates.push(patch.is_closed ? "closed thread" : "unarchived thread");
			}
			return (
				<>
					<span></span>
					<div class="content">
						<span class="body" ref={bodyEl!}>
							<span class="author">{authorName}</span> updated the thread:{" "}
							{listFormatter.format(updates) || "did nothing"}
						</span>
					</div>
					<span class="timestamp">
						{date.toDateString()}
					</span>
				</>
			);
		} else {
			// console.log(md.parse(props.message.content!));
			// IDEA: make usernames sticky? so when scrolling, you can see who sent a certain message
			// {tooltip(
			// 	{ placement: "right-start", animGroup: "message-user", interactive: true },
			// 	<UserTooltip user={props.message.author} />,
			// 	<div class="author-wrap">
			// 		<div
			// 			class="author"
			// 			classList={{ "override-name": !!props.message.override_name }}>
			// 		{authorName}
			// 		</div>
			// 	</div>
			// )}
			return (
				<>
					<Show
						when={props.message.reply_id &&
							ctx.data.messages[props.message.reply_id!]}
					>
						<Reply reply={ctx.data.messages[props.message.reply_id!]} />
					</Show>
					<div class="author-wrap">
						<div
							class="author"
							classList={{ "override-name": !!props.message.override_name }}
						>
							{authorName}
						</div>
					</div>
					<div class="content">
						<Show when={props.message.content}>
							<div
								class="body markdown"
								classList={{ local: props.is_local }}
								ref={bodyEl!}
							>
								<span
									innerHTML={sanitizeHtml(
										md.parse(props.message.content!) as string,
										sanitizeHtmlOptions,
									).trim()}
								>
								</span>
								<Show when={props.message.id !== props.message.version_id}>
									<span class="edited">(edited)</span>
								</Show>
							</div>
						</Show>
						<ul class="attachments">
							<For each={props.message.attachments}>
								{(att) => getAttachment(att)}
							</For>
						</ul>
					</div>
					<span class="timestamp">{date.toDateString()}</span>
				</>
			);
		}
	}

	return <>{getComponent()}</>;
};

function getTimelineItem(thread: ThreadT, item: TimelineItemT) {
	switch (item.type) {
		case "message": {
			// unread: item.message.unread,
			// "bg-[#67dc8222]": item.message.mention,
			// "shadow-arst": item.message.mention || item.message.unread,
			// "shadow-[#67dc82]": item.message.mention,
			// "shadow-[#3fa9c9]": item.message.unread,
			// "text-fg4": item.message.is_local,
			const ctx = useCtx();
			return (
				<li
					class="message"
					classList={{
						"selected":
							item.message.id === ctx.data.thread_state[thread.id]?.reply_id,
					}}
					data-message-id={item.message.id}
				>
					<Message message={item.message} is_local={item.is_local} />
				</li>
			);
		}
		case "info": {
			// <header class="sticky top-[0] px-[144px] bg-bg3 mb-[8px] border-b-[1px] border-b-sep mt-[-8px]">
			// <header class="shadow-foo shadow-[#0009] bg-bg1 p-2 text-cente">
			// 	<p>more info here</p>
			// </header>
			return (
				<li class="header">
					<header>
						<h1>{thread.name}</h1>
						<p>
							{thread.description ?? "(no description)"} /
							<Show when={thread.is_closed}>(archived)</Show>
						</p>
					</header>
				</li>
			);
		}
		case "spacer": {
			return (
				<li class="spacer">
					<div style="flex:1;height:800px;grid-column:span 3"></div>
				</li>
			);
		}
		case "spacer-mini2": {
			return (
				<li class="spacer">
					<div style="flex:1;height:8rem;grid-column:span 3"></div>
				</li>
			);
		}
		case "spacer-mini": {
			return (
				<li class="spacer">
					<div style="height:2rem;grid-column:span 3"></div>
				</li>
			);
		}
		case "anchor": {
			return <li class="anchor"></li>;
		}
		case "unread-marker": {
			return (
				<li class="unread-marker">
					<div class="content">new messages</div>
				</li>
			);
		}
			// <Match when={props.item.type === "unread-marker" && false}>
			// </Match>
			// <Match when={props.item.type === "unread-marker"}>
			// 	<li classList={{ unreadMarker2: true }}>
			// 		<hr />
			// 		<span>unread messages</span>
			// 		<hr />
			// 	</li>
			// </Match>
	}
}

export const TimelineItem = (
	props: { thread: ThreadT; item: TimelineItemT },
) => {
	return <>{getTimelineItem(props.thread, props.item)}</>;

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
};
