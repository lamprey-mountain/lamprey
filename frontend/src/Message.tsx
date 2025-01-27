import { getTimestampFromUUID } from "sdk";
import { MediaT, MessageT, MessageType } from "./types.ts";
import { useCtx } from "./context.ts";
import { createMemo, For, Show } from "solid-js";
import { marked } from "marked";
// @ts-types="npm:@types/sanitize-html@^2.13.0"
import sanitizeHtml from "npm:sanitize-html";

type MessageProps = {
	message: MessageT;
	is_local: boolean;
};

type MessageTextProps = {
	message: MessageT;
	is_local: boolean;
};

const sanitizeHtmlOptions: sanitizeHtml.IOptions = {
	transformTags: {
		del: "s",
	},
};

const md = marked.use({
	breaks: true,
	gfm: true,
});

function MessageText(props: MessageTextProps) {
	const html = createMemo(() =>
		sanitizeHtml(
			md.parse(props.message.content!) as string,
			sanitizeHtmlOptions,
		).trim()
	);
	return (
		<div class="body markdown" classList={{ local: props.is_local }}>
			<span innerHTML={html()}></span>
			<Show when={props.message.id !== props.message.version_id}>
				<span class="edited">(edited)</span>
			</Show>
		</div>
	)
}

export function MessageView(props: MessageProps) {
	const ctx = useCtx();

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
						<span class="body">
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
			return (
				<>
					<Show
						when={props.message.reply_id &&
							ctx.data.messages[props.message.reply_id!]}
					>
						<ReplyView reply={ctx.data.messages[props.message.reply_id!]} />
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
							<MessageText is_local={props.is_local} message={props.message} />
						</Show>
						<ul class="attachments">
							<For each={props.message.attachments}>
								{(att) => renderAttachment(att)}
							</For>
						</ul>
					</div>
					<span class="timestamp">{date.toDateString()}</span>
				</>
			);
		}
	}

	return <>{getComponent()}</>;
}

function ReplyView(props: { reply: MessageT }) {
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

export function renderAttachment(a: MediaT) {
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
