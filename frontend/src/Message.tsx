import { getTimestampFromUUID, Message, Thread } from "sdk";
import { MessageT, MessageType } from "./types.ts";
import { For, Match, Show, Switch } from "solid-js";
import { marked } from "marked";
import sanitizeHtml from "sanitize-html";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import {
	AudioView,
	FileView,
	ImageView,
	TextView,
	VideoView,
	VideoViewOld,
} from "./media/mod.tsx";
import { flags } from "./flags.ts";
import { MediaProps } from "./media/util.ts";

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
	const api = useApi();
	const thread = api.threads.fetch(() => props.message.thread_id);

	function getComponent() {
		const date =
			/^[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}$/.test(
				props.message.id,
			)
				? getTimestampFromUUID(props.message.id)
				: new Date();
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
			if (patch.state) {
				updates.push(`set state to ${patch.state}`);
			}
			return (
				<>
					<span></span>
					<div class="content">
						<span class="body">
							<Author message={props.message} thread={thread()} /> updated the thread:{" "}
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
							class="author sticky has-menu"
							classList={{ "override-name": !!props.message.override_name }}
							data-user-id={props.message.author.id}
							data-thread-id={props.message.thread_id}
						>
							<Author message={props.message} thread={thread()} />
						</div>
					</div>
					<div class="content">
						<Show when={props.message.content}>
							<MessageText message={props.message} />
						</Show>
						<Show when={props.message.attachments.length}>
							<ul class="attachments">
								<For each={props.message.attachments}>
									{(att) => <AttachmentView media={att} />}
								</For>
							</ul>
						</Show>
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
	const thread = api.threads.fetch(() => props.thread_id);

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
					<Show when={reply() && thread()} fallback={<span class="author"></span>}>
						<Author message={reply()!} thread={thread()!} />:
					</Show>
					{content()}
				</Show>
			</div>
			<div class="reply"></div>
		</>
	);
}

export function AttachmentView(props: MediaProps) {
	return (
		<Switch>
			<Match when={flags.has("new_media")}>
				<AttachmentView2 media={props.media} />
			</Match>
			<Match when={true}>
				<AttachmentView1 media={props.media} />
			</Match>
		</Switch>
	);
}

export function AttachmentView1(props: MediaProps) {
	const ty = () => props.media.source.mime.split(";")[0];
	const b = () => props.media.source.mime.split("/")[0];
	const byteFmt = Intl.NumberFormat("en", {
		notation: "compact",
		style: "unit",
		unit: "byte",
		unitDisplay: "narrow",
	});
	// const [ty, paramsRaw] = a.mime.split(";");
	// const params = new Map(paramsRaw?.split(" ").map(i => i.trim().split("=") as [string, string]));
	// console.log({ ty, params });

	if (b() === "image") {
		return (
			<li>
				<ImageView media={props.media} />
				<a download={props.media.filename} href={props.media.source.url}>
					download {props.media.filename}
				</a>
				<div class="dim">
					{ty()} - {byteFmt.format(props.media.source.size)}
				</div>
			</li>
		);
	} else if (b() === "video") {
		return (
			<li>
				<VideoViewOld media={props.media} />
				<a download={props.media.filename} href={props.media.source.url}>
					download {props.media.filename}
				</a>
				<div class="dim">
					{ty()} - {byteFmt.format(props.media.source.size)}
				</div>
			</li>
		);
	} else if (b() === "audio") {
		return (
			<li>
				<audio controls src={props.media.source.url} />
				<a download={props.media.filename} href={props.media.source.url}>
					download {props.media.filename}
				</a>
				<div class="dim">
					{ty()} - {byteFmt.format(props.media.source.size)}
				</div>
			</li>
		);
	} else {
		return (
			<li>
				<a download={props.media.filename} href={props.media.source.url}>
					download {props.media.filename}
				</a>
				<div class="dim">
					{ty()} - {byteFmt.format(props.media.source.size)}
				</div>
			</li>
		);
	}
}

export function AttachmentView2(props: MediaProps) {
	const b = () => props.media.source.mime.split("/")[0];
	const byteFmt = Intl.NumberFormat("en", {
		notation: "compact",
		style: "unit",
		unit: "byte",
		unitDisplay: "narrow",
	});

	const ty = () => props.media.source.mime.split(";")[0];
	if (b() === "image") {
		return (
			<li>
				<ImageView media={props.media} />
				<a download={props.media.filename} href={props.media.source.url}>
					download {props.media.filename}
				</a>
				<div class="dim">
					{ty()} - {byteFmt.format(props.media.source.size)}
				</div>
			</li>
		);
	} else if (b() === "video") {
		return (
			<li class="raw">
				<VideoView media={props.media} />
			</li>
		);
	} else if (b() === "audio") {
		return (
			<li class="raw">
				<AudioView media={props.media} />
			</li>
		);
	} else if (
		b() === "text" || /^application\/json\b/.test(props.media.source.mime)
	) {
		return (
			<li>
				<TextView media={props.media} />
			</li>
		);
	} else {
		return (
			<li>
				<FileView media={props.media} />
			</li>
		);
	}
}

function Author(props: { message: Message, thread?: Thread }) {
	const api = useApi();
	const room_member = props.thread ? api.room_members.fetch(() => props.thread!.room_id, () => props.message.author.id) : null;
	const thread_member = api.thread_members.fetch(() => props.message.thread_id, () => props.message.author.id);
	// const user = api.users.fetch(() => props.message.author.id);

	function name() {
		let name = props.message.override_name;
		const tm = thread_member();
		if (tm?.membership === "Join") name ??= tm.override_name;

		const rm = room_member?.();
		if (rm?.membership === "Join") name ??= rm.override_name;

		name ??= props.message.author.name;

		return name;
	}

	return <span class="author">{name()}</span>
}
