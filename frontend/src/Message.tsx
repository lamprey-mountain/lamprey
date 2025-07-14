import { getTimestampFromUUID, type Message, type Thread } from "sdk";
import { type MessageT, MessageType } from "./types.ts";
import {
	createEffect,
	createSignal,
	For,
	Match,
	onMount,
	Show,
	Switch,
} from "solid-js";
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
} from "./media/mod.tsx";
import { flags } from "./flags.ts";
import { byteFmt, getUrl, type MediaProps } from "./media/util.tsx";
import { Time } from "./Time.tsx";
import { createTooltip, tooltip } from "./Tooltip.tsx";
import { Avatar, UserView } from "./User.tsx";
import { EmbedView } from "./UrlEmbed.tsx";
import { transformBlock } from "./text.tsx";

type MessageProps = {
	message: MessageT;
	separate?: boolean;
};

type MessageTextTaggedProps = {
	message: MessageT & { type: "DefaultTagged" };
};

type MessageTextMarkdownProps = {
	message: MessageT & { type: "DefaultMarkdown" };
};

const sanitizeHtmlOptions: sanitizeHtml.IOptions = {
	// transformTags: {
	// 	del: "s",
	// },
	allowedTags: sanitizeHtml.defaults.allowedTags.concat(["ins", "del"]),
};

const md = marked.use({
	breaks: true,
	gfm: true,
});

const contentToHtml = new WeakMap();

function MessageTextMarkdown(props: MessageTextMarkdownProps) {
	function getHtml(): string {
		const cached = contentToHtml.get(props.message);
		if (cached) return cached;
		// console.count("render_html");
		const html = sanitizeHtml(
			md.parse(props.message.content ?? "") as string,
			sanitizeHtmlOptions,
		).trim();
		contentToHtml.set(props.message, html);
		return html;
	}

	let highlightEl;
	function highlight() {
		getHtml();
		import("highlight.js").then(({ default: hljs }) => {
			// HACK: retain line numbers
			// FIXME: use language if provided instead of guessing
			for (const el of [...highlightEl.querySelectorAll("pre")]) {
				el.dataset.highlighted = "";
				hljs.highlightElement(el);
			}
		});
	}

	createEffect(highlight);

	const ctx = useCtx();
	const viewHistory = () => {
		ctx.dispatch({
			do: "modal.open",
			modal: {
				type: "message_edits",
				message_id: props.message.id,
				thread_id: props.message.thread_id,
			},
		});
	};

	return (
		<div
			class="body markdown"
			classList={{ local: props.message.is_local }}
			ref={highlightEl}
		>
			<span innerHTML={getHtml()}></span>
			<Show when={props.message.id !== props.message.version_id}>
				<span class="edited" onClick={viewHistory}>(edited)</span>
			</Show>
		</div>
	);
}

function MessageTextTagged(props: MessageTextTaggedProps) {
	return (
		<div class="body markdown" classList={{ local: props.message.is_local }}>
			{transformBlock(props.message.content ?? "")}
			<Show when={props.message.id !== props.message.version_id}>
				<span class="edited">{" "}(edited)</span>
			</Show>
		</div>
	);
}

export function MessageView(props: MessageProps) {
	const api = useApi();
	const thread = api.threads.fetch(() => props.message.thread_id);

	function reactionAdd(key: string) {
		api.client.http.PUT(
			"/api/v1/thread/{thread_id}/message/{message_id}/reaction/{key}",
			{
				params: {
					path: {
						key,
						message_id: props.message.id,
						thread_id: props.message.thread_id,
					},
				},
			},
		);
	}

	function reactionDel(key: string) {
		api.client.http.DELETE(
			"/api/v1/thread/{thread_id}/message/{message_id}/reaction/{key}",
			{
				params: {
					path: {
						key,
						message_id: props.message.id,
						thread_id: props.message.thread_id,
					},
				},
			},
		);
	}

	function getComponent() {
		const date =
			/^[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}$/.test(
					props.message.id,
				)
				? getTimestampFromUUID(props.message.id)
				: new Date();
		if (props.message.type === MessageType.ThreadUpdate && false) {
			const updates = [];
			const listFormatter = new Intl.ListFormat();
			const patch = props.message.patch as any;
			if (patch) {
				if (patch.name) updates.push(`set name to ${patch.name}`);
				if (patch.description) {
					updates.push(
						patch.description ? `set description to ${patch.description}` : "",
					);
				}
				if (patch.state) {
					updates.push(`set state to ${patch.state}`);
				}
			} else {
				console.warn("missing patch", props.message);
			}
			return (
				<>
					<span></span>
					<div class="content">
						<span class="body">
							<Author message={props.message} thread={thread()} />{" "}
							updated the thread:{" "}
							{listFormatter.format(updates) || "did nothing"}
						</span>
					</div>
					<div class="time">
						<Time date={date} animGroup="message-ts" />
					</div>
				</>
			);
		} else if (
			props.message.type === "DefaultMarkdown" ||
			props.message.type === "DefaultTagged"
		) {
			const [arrow_width, set_arrow_width] = createSignal(0);
			const user = api.users.fetch(() => props.message.author_id);
			const set_w = (e: HTMLElement) => {
				onMount(() => {
					set_arrow_width(
						e.querySelector(".user")!.getBoundingClientRect().width,
					);
				});
			};
			const ctx = useCtx();
			const withAvatar = ctx.settings.get("message_pfps") === "yes";

			// TODO: this code is getting messy and needs a refactor soon...
			return (
				<article
					class="message menu-message"
					data-message-id={props.message.id}
					classList={{
						withavatar: withAvatar,
						separate: props.separate,
						notseparate: !props.separate,
					}}
				>
					<Show when={props.message.reply_id}>
						<ReplyView
							thread_id={props.message.thread_id}
							reply_id={props.message.reply_id!}
							arrow_width={arrow_width()}
						/>
					</Show>
					<Show when={withAvatar}>
						<Show when={props.separate}>
							<Avatar user={user()} />
							<div
								class="author menu-user"
								classList={{ "override-name": !!props.message.override_name }}
							>
								<Author message={props.message} thread={thread()} />
								<Time date={date} animGroup="message-ts" />
							</div>
						</Show>
						<Show when={!props.separate}>
							<div class="avatar"></div>
						</Show>
						<div class="content">
							<Show when={props.message.type === "DefaultMarkdown"}>
								<MessageTextMarkdown message={props.message} />
							</Show>
							<Show when={props.message.type === "DefaultTagged"}>
								<MessageTextTagged message={props.message} />
							</Show>
							<Show when={props.message.attachments?.length}>
								<ul class="attachments">
									<For each={props.message.attachments}>
										{(att) => <AttachmentView media={att} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.embeds?.length}>
								<ul class="embeds">
									<For each={props.message.embeds}>
										{(embed) => <EmbedView embed={embed} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.reactions?.length}>
								<ul class="reactions">
									<For each={props.message.reactions}>
										{(r) => (
											<li
												classList={{ me: r.self }}
												onClick={() =>
													r.self ? reactionAdd(r.key) : reactionDel(r.key)}
											>
												<span class="emoji">{r.key.toString()}</span>
												<span class="count">{r.count}</span>
											</li>
										)}
									</For>
								</ul>
							</Show>
						</div>
					</Show>
					<Show when={!withAvatar}>
						<div class="author-wrap">
							<div
								class="author sticky menu-user"
								classList={{ "override-name": !!props.message.override_name }}
								ref={set_w}
							>
								<Author message={props.message} thread={thread()} />
							</div>
						</div>
						<div class="content">
							<Show when={props.message.type === "DefaultMarkdown"}>
								<MessageTextMarkdown message={props.message} />
							</Show>
							<Show when={props.message.type === "DefaultTagged"}>
								<MessageTextTagged message={props.message} />
							</Show>
							<Show when={props.message.attachments?.length}>
								<ul class="attachments">
									<For each={props.message.attachments}>
										{(att) => <AttachmentView media={att} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.embeds?.length}>
								<ul class="embeds">
									<For each={props.message.embeds}>
										{(embed) => <EmbedView embed={embed} />}
									</For>
								</ul>
							</Show>
							<Show when={props.message.reactions?.length}>
								<ul class="reactions">
									<For each={props.message.reactions}>
										{(r) => (
											<li
												classList={{ me: r.self }}
												onClick={() =>
													r.self ? reactionAdd(r.key) : reactionDel(r.key)}
											>
												<span class="emoji">{r.key.toString()}</span>
												<span class="count">{r.count}</span>
											</li>
										)}
									</For>
								</ul>
							</Show>
						</div>
						<Time date={date} animGroup="message-ts" />
					</Show>
				</article>
			);
		}
	}

	return <>{getComponent()}</>;
}

type ReplyProps = {
	thread_id: string;
	reply_id: string;
	arrow_width?: number;
};

function ReplyView(props: ReplyProps) {
	const ctx = useCtx();
	const api = useApi();
	const reply = api.messages.fetch(() => props.thread_id, () => props.reply_id);
	const thread = api.threads.fetch(() => props.thread_id);

	const content = () => {
		const r = reply();
		if (!r) return;
		return ('content' in r && r.content) ?? (('attachments' in r && r.attachments) ? `${r.attachments.length} attachment(s)` : undefined);
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
			<div class="reply">
				<div class="arrow">
					<svg
						viewBox="0 0 100 100"
						preserveAspectRatio="none"
						style={{ width: props.arrow_width ? `${props.arrow_width}px` : "" }}
					>
						<path
							vector-effect="non-scaling-stroke"
							shape-rendering="crispEdges"
							// M = move to x y
							// L = line to x y
							d="M 50 100 L 50 50 L 100 50"
						/>
					</svg>
				</div>
				<div class="content" onClick={scrollToReply}>
					<Show when={!reply.loading} fallback="loading...">
						<Show
							when={reply() && thread()}
							fallback={<span class="author"></span>}
						>
							<Author message={reply()!} thread={thread()!} />
						</Show>
						{content()}
					</Show>
				</div>
			</div>
		</>
	);
}

export function AttachmentView(props: MediaProps) {
	const b = () => props.media.source.mime.split("/")[0];
	const ty = () => props.media.source.mime.split(";")[0];
	if (b() === "image") {
		return (
			<li class="raw">
				<ImageView
					media={props.media}
					thumb_height={props.size}
					thumb_width={props.size}
				/>
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
			<li class="raw">
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

function Author(props: { message: Message; thread?: Thread }) {
	const api = useApi();
	const room_member = props.thread
		? api.room_members.fetch(
			() => props.thread!.room_id,
			() => props.message.author_id,
		)
		: () => null;
	const thread_member = api.thread_members.fetch(
		() => props.message.thread_id,
		() => props.message.author_id,
	);
	const user = api.users.fetch(() => props.message.author_id);

	function name() {
		let name = ('override_name' in props.message) ? props.message.override_name : undefined;
		const tm = thread_member();
		if (tm?.membership === "Join") name ??= tm.override_name;

		const rm = room_member?.();
		if (rm?.membership === "Join") name ??= rm.override_name;

		const us = user();
		name ??= us?.name;

		return name;
	}

	const { content } = createTooltip({
		// animGroup: "users",
		placement: "right-start",
		interactive: true,
		tip: () => (
			<UserView
				user={user()}
				room_member={room_member()}
				thread_member={thread_member()}
			/>
		),
	});

	return (
		<span
			class="user"
			classList={{ "override-name": !!props.message.override_name }}
			data-user-id={props.message.author_id}
			use:content
		>
			{name()}
		</span>
	);
}
