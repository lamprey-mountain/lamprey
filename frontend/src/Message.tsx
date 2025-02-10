import { getTimestampFromUUID } from "sdk";
import { MediaT, MessageT, MessageType } from "./types.ts";
import { For, Show } from "solid-js";
import { marked } from "marked";
// @ts-types="npm:@types/sanitize-html@^2.13.0"
import sanitizeHtml from "npm:sanitize-html";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { AudioView, ImageView, VideoView, VideoViewOld } from "./media/mod.tsx";
import { flags } from "./flags.ts";

type MessageProps = {
	message: MessageT;
};

type MessageTextProps = {
	message: MessageT;
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

const contentToHtml = new WeakMap();

function MessageText(props: MessageTextProps) {
	function getHtml(): string {
		const cached = contentToHtml.get(props.message);
		if (cached) return cached;
		// console.count("render_html");
		const html = sanitizeHtml(
			md.parse(props.message.content!) as string,
			sanitizeHtmlOptions,
		).trim();
		contentToHtml.set(props.message, html);
		return html;
	}

	return (
		<div class="body markdown" classList={{ local: props.message.is_local }}>
			<span innerHTML={getHtml()}></span>
			<Show when={props.message.id !== props.message.version_id}>
				<span class="edited">(edited)</span>
			</Show>
		</div>
	);
}

export function MessageView(props: MessageProps) {
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
					<Show when={props.message.reply_id}>
						<ReplyView
							thread_id={props.message.thread_id}
							reply_id={props.message.reply_id!}
						/>
					</Show>
					<div class="author-wrap">
						<div
							class="author has-menu"
							classList={{ "override-name": !!props.message.override_name }}
							data-user-id={props.message.author.id}
							data-thread-id={props.message.thread_id}
						>
							{authorName}
						</div>
					</div>
					<div class="content">
						<Show when={props.message.content}>
							<MessageText message={props.message} />
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

type ReplyProps = {
	thread_id: string;
	reply_id: string;
};

function ReplyView(props: ReplyProps) {
	const ctx = useCtx();
	const api = useApi();
	const reply = api.messages.fetch(() => props.thread_id, () => props.reply_id);

	const name = () => {
		const r = reply();
		if (!r) return;
		return r.override_name ?? r.author.name;
	};

	const content = () => {
		const r = reply();
		if (!r) return;
		return r.content ?? `${r.attachments.length} attachment(s)`;
	};

	const scrollToReply = () => {
		// if (!props.reply) return;
		ctx.thread_anchor.set(props.thread_id, {
			type: "context",
			limit: 50, // TODO: calc dynamically
			message_id: props.reply_id,
		});
		ctx.thread_highlight.set(props.thread_id, props.reply_id);
	};

	return (
		<>
			<div class="reply arrow">{"\u21B1"}</div>
			<div class="reply reply-content" onClick={scrollToReply}>
				<Show when={!reply.loading} fallback="loading..">
					<span class="author">{name()}:</span>
					{content()}
				</Show>
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
		return (
			<li>
				<ImageView media={a} />
				<a download={a.filename} href={a.url}>download {a.filename}</a>
				<div class="dim">{ty} - {byteFmt.format(a.size)}</div>
			</li>
		);
	} else if (b === "video") {
		return (
			<li classList={{ raw: flags.has("new_media") }}>
				<Show when={!flags.has("new_media")} fallback={<VideoView media={a} />}>
					<VideoViewOld media={a} />
					<a download={a.filename} href={a.url}>download {a.filename}</a>
					<div class="dim">{ty} - {byteFmt.format(a.size)}</div>
				</Show>
			</li>
		);
	} else if (b === "audio") {
		return (
			<li classList={{ raw: flags.has("new_media") }}>
				<Show when={!flags.has("new_media")} fallback={<AudioView media={a} />}>
					<audio controls src={a.url} />
					<a download={a.filename} href={a.url}>download {a.filename}</a>
					<div class="dim">{ty} - {byteFmt.format(a.size)}</div>
				</Show>
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
