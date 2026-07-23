import { autoUpdate } from "@floating-ui/dom";
import type { ReactiveSet } from "@solid-primitives/set";
import { type Channel, getTimestampFromUUID } from "sdk";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	onCleanup,
	Show,
} from "solid-js";
import { useApi, useFlumes, useUsers } from "@/api";
import { Components } from "@/atoms/Components";
import { Time } from "@/atoms/Time";
import { Avatar } from "@/avatar/UserAvatar";
import { EmbedView } from "@/components/shared/UrlEmbed";
import { useCurrentUser } from "@/contexts/currentUser";
import { useChannel } from "@/contexts/mod";
import {
	AttachmentView,
	MessageTextMarkdown,
	UserDisplayName,
} from "../chat/Message";
import { useMessageToolbar } from "../chat/message-toolbar-context";
import { Reactions } from "../chat/Reactions";
import { CommentEditor, type CommentNode } from "./Forum2";

export const Comment = (props: {
	collapsed: ReactiveSet<string>;
	channel: Channel;
	node: CommentNode;
	depth: number;
}) => {
	const message = () => props.node.message;
	const children = () => props.node.children;
	const api = useApi();
	const users = useUsers();
	const flumes = useFlumes();
	const [ch, chUpdate] = useChannel();
	const toolbar = useMessageToolbar();

	const flume = () =>
		message().flume?.state === "Live" && flumes.get(message().id);

	const collapsed = () => props.collapsed.has(message().id);
	const isEditing = () => ch.editingMessage?.message_id === message().id;
	const isSelected = () => ch.selectedMessages?.includes(message().id) ?? false;
	const isReplyTarget = () => ch.reply_id === message().id;
	const inSelectMode = () => ch.selectMode ?? false;

	const currentUser = useCurrentUser();
	const isOwnMessage = () => {
		return currentUser()?.id === message().author_id;
	};

	const _canEditMessage = () => {
		const msg = message();
		return (
			msg.latest_version.type === "DefaultMarkdown" &&
			!msg.is_local &&
			isOwnMessage()
		);
	};

	const handleClick = (e: MouseEvent) => {
		if (!inSelectMode() || !chUpdate) return;
		e.preventDefault();
		e.stopPropagation();

		const message_id = message().id;
		const selected = ch.selectedMessages;

		if (e.shiftKey && selected.length > 0) {
			// TODO: range selection for comments
			if (selected.includes(message_id)) {
				chUpdate(
					"selectedMessages",
					selected.filter((id) => id !== message_id),
				);
			} else {
				chUpdate("selectedMessages", [...selected, message_id]);
			}
		} else {
			if (selected.includes(message_id)) {
				chUpdate(
					"selectedMessages",
					selected.filter((id) => id !== message_id),
				);
			} else {
				chUpdate("selectedMessages", [...selected, message_id]);
			}
		}
	};

	const [summary] = createResource(
		() => {
			const v = message().latest_version;
			if (v.type === "DefaultMarkdown" && v.content) {
				return {
					content: v.content,
					channel_id: message().channel_id,
					mentions: v.mentions,
				};
			}
			return null;
		},
		async (data) => {
			if (!data) return "(no content)";
			return await api.stripMarkdownAndResolveMentions(
				data.content,
				data.channel_id,
				data.mentions,
			);
		},
	);

	const countAllChildren = (node: CommentNode): number => {
		return (
			node.children.length +
			node.children.reduce((sum, child) => sum + countAllChildren(child), 0)
		);
	};

	let lineRef: HTMLDivElement | undefined;
	let childrenListRef: HTMLUListElement | undefined;
	let loadMoreRef: HTMLDivElement | undefined;

	const [lineHeight, setLineHeight] = createSignal(0);

	// TODO: double check, clean up this code
	createEffect(() => {
		const line = lineRef;
		const list = childrenListRef;
		const more = loadMoreRef;

		const target = more || list?.lastElementChild;
		if (!line || !target) return;

		const cleanup = autoUpdate(line, target as HTMLElement, () => {
			const lineRect = line.getBoundingClientRect();
			const targetRect = target.getBoundingClientRect();
			let h = targetRect.top - lineRect.top;
			if (more) {
				h += targetRect.height / 2; // to the middle of the load more button
			} else {
				h += 24; // to the middle of the avatar (approx 24px down from the top of the comment wrap)
			}
			setLineHeight(Math.max(0, h));
		});
		onCleanup(cleanup);
	});

	// FIXME: use timeline.commands instead
	// createEffect(() => {
	// 	const hl = ch.highlight;
	// 	if (hl === message().id) {
	// 		// expand parent comments
	// 		// props.expand(); // TODO: we need a way to expand parents if they are collapsed
	// 		// for now we just scroll to it
	// 		const el = contentEl?.closest(".comment");
	// 		if (el) {
	// 			el.scrollIntoView({ block: "center" });
	// 			highlight(el);
	// 			chUpdate("highlight", undefined);
	// 		}
	// 	}
	// });
	const author = users.use(() => message().author_id);

	// TODO: show button to load more replies
	const canLoadMore = () => false;

	return (
		<div
			class="comment-wrap menu-message"
			data-message-id={message().id}
			classList={{
				collapsed: collapsed(),
				selected: isSelected(),
				"reply-target": isReplyTarget(),
				selectable: inSelectMode(), // TODO: lift to top level
				toplevel: props.depth === 0,
				"from-op": false, // TODO
			}}
			style={{
				"--depth": props.depth,
				"--is-darker": props.depth % 2 === 1 ? 1 : 0,
			}}
			onClick={handleClick}
			onMouseEnter={(e) => {
				toolbar.setTarget({ message: message(), element: e.currentTarget });
			}}
			onMouseLeave={(e) => {
				const toolbarEl = toolbar.containerRef();
				if (
					toolbarEl &&
					e.relatedTarget instanceof Node &&
					toolbarEl.contains(e.relatedTarget)
				) {
					return;
				}
				toolbar.setTarget(null);
			}}
		>
			<Show when={props.depth > 0}>
				<div class="line top"></div>
			</Show>
			<Show when={!collapsed() && (children().length > 0 || canLoadMore())}>
				<div
					class="line side"
					classList={{ "connects-to-reply": !canLoadMore() }}
					ref={lineRef}
					style={{ height: `${lineHeight() - (!canLoadMore() ? 8 : 0)}px` }}
				></div>
			</Show>
			<div class="comment-wrap2">
				<article class="comment message separate">
					<aside class="aside">
						{/* TODO: port logic
				<Avatar user={props.user} animate={props.hovered} />
				<Time date={props.date} animGroup="message-ts" format="time" />
			  */}
						{/* TODO: animate avatar on hover */}
						<Show when={author()} fallback={<div class="avatar"></div>}>
							{(a) => <Avatar animate={false} user={a()} />}
						</Show>
					</aside>
					<div class="content">
						<header class="header">
							{/* FIXME: re-add comment collapse button
          <button
            type="button"
            class="collapse"
            onClick={() => collapsed()
              ? props.collapsed.delete(message().id)
              : props.collapsed.add(message().id)}
          >
            {collapsed() ? "+" : "-"}
          </button>
          */}
							<Show when={collapsed()}>
								<span class="childCount dim">
									[{countAllChildren(props.node)}]
								</span>
							</Show>
							<Show when={props.channel}>
								<UserDisplayName
									user_id={props.node.message.author_id}
									room_id={props.channel.room_id ?? undefined}
									thread_id={props.node.message.channel_id}
									onClick
								/>
							</Show>
							<Time
								date={getTimestampFromUUID(message().id)}
								class="onlytime"
								format="time"
							/>
							<Time
								date={getTimestampFromUUID(message().id)}
								class="full"
								format="full"
							/>
							<Show when={collapsed()}>
								<div class="summary">{summary() ?? "..."}</div>
							</Show>
						</header>
						<Show when={!collapsed()}>
							<Show
								when={!isEditing()}
								fallback={
									<CommentEditor message={message()} channel={props.channel} />
								}
							>
								{(() => {
									const msg = message();
									return <MessageTextMarkdown message={msg} />;
								})()}
							</Show>
						</Show>
					</div>
					<div class="accessories">
						<Show when={!collapsed() && !isEditing()}>
							<div style="padding: 0 8px">
								{(() => {
									const version = message().latest_version;
									if (version.type !== "DefaultMarkdown") return null;
									return (
										<>
											<Show when={version.attachments?.length}>
												<ul class="attachments">
													<For each={version.attachments}>
														{(att) => <AttachmentView att={att} />}
													</For>
												</ul>
											</Show>
											<Show when={version.embeds?.length}>
												<ul class="embeds">
													<For each={version.embeds}>
														{(embed) => <EmbedView embed={embed} />}
													</For>
												</ul>
											</Show>
										</>
									);
								})()}
							</div>

							<Show when={message().reactions?.length}>
								<Reactions message={message()} />
							</Show>

							<Show when={flume()}>
								{(f) => (
									<Components
										components={f().components}
										channelId={message().channel_id}
									/>
								)}
							</Show>

							<Show
								when={
									message().latest_version.type === "DefaultMarkdown" &&
									(message().latest_version as any).components?.length &&
									!flume()
								}
							>
								<Components
									components={
										(message().latest_version as any).components ?? []
									}
									channelId={message().channel_id}
								/>
							</Show>
						</Show>
					</div>
				</article>
			</div>
			<Show when={!collapsed() && children().length > 0}>
				<ul class="children" ref={childrenListRef}>
					<For each={children()}>
						{(child) => (
							<li>
								<Comment
									collapsed={props.collapsed}
									channel={props.channel}
									node={child}
									depth={props.depth + 1}
								/>
							</li>
						)}
					</For>
				</ul>
			</Show>
			<Show when={!collapsed() && canLoadMore()}>
				<div class="more dim" ref={loadMoreRef}>
					{123} more replies...
				</div>
			</Show>
		</div>
	);
};
