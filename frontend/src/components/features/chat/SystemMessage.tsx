import { useNavigate } from "@solidjs/router";
import type {
	AutomodAction,
	Message as MessageT,
	MessageVersion as MessageVersionT,
} from "sdk";
import { createMemo, type JSX, Match, Show, Switch } from "solid-js";
import { useChannels } from "@/api";
import { useCtx } from "@/app/context";
import { Duration } from "@/atoms/Duration.tsx";
import { Time } from "@/atoms/Time";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import {
	icCall,
	icChannelMove,
	icEdit,
	icMemberAdd,
	icMemberJoin,
	icMemberRemove,
	icPin,
	icReply,
	icSword,
	icThread,
} from "@/utils/icons.ts";
import { useVoice } from "../voice/context.tsx";
import { UserDisplayName } from "./Message";
import { useMessageToolbar } from "./message-toolbar-context";

export type SystemMessageBaseProps = {
	message: MessageT;
	date: Date;
	separate: boolean;
	toolbarVisible: boolean;
	handleClick: (e: MouseEvent) => void;
	onMouseDown: (e: MouseEvent) => void;
	handleAltClick: (e: MouseEvent) => void;
	setHovered: (v: boolean) => void;
	messageArticleRef: (el: HTMLElement | undefined) => void;
	room_id?: string;
};

export type SystemMessageProps = SystemMessageBaseProps & {
	icon: string;
	content: JSX.Element;
	class?: string;
};

export const SystemMessage = (props: SystemMessageProps) => {
	const toolbar = useMessageToolbar();

	return (
		<article
			ref={props.messageArticleRef}
			class={`message menu-message oneline ${props.class ?? ""}`}
			data-message-id={props.message.id}
			classList={{
				separate: props.separate,
			}}
			onClick={props.handleClick}
			onMouseDown={(e) => {
				props.onMouseDown(e);
				props.handleAltClick(e);
			}}
			onMouseEnter={(e) => {
				props.setHovered(true);
				toolbar.setTarget({ message: props.message, element: e.currentTarget });
			}}
			onMouseLeave={(e) => {
				props.setHovered(false);
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
			<aside class="aside">
				<img class="icon" src={props.icon} />
			</aside>
			<div class="content">
				{props.content}
				<Time
					date={props.date}
					animGroup="message-ts"
					class="onlytime"
					format="time"
				/>
				<Time
					date={props.date}
					animGroup="message-ts"
					class="full"
					format="full"
				/>
			</div>
		</article>
	);
};

export function SystemMessageMemberAdd(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	const version = () =>
		props.message.latest_version as MessageVersionT & {
			target_user_id: string;
		};

	return (
		<SystemMessage
			{...props}
			icon={icMemberAdd}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.member_add",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						<span class="author">
							<UserDisplayName
								user_id={version().target_user_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

export function SystemMessageMemberRemove(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	const version = () =>
		props.message.latest_version as MessageVersionT & {
			target_user_id: string;
		};

	return (
		<SystemMessage
			{...props}
			icon={icMemberRemove}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.member_remove",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						<span class="author">
							<UserDisplayName
								user_id={version().target_user_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

export function SystemMessageMemberJoin(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	return (
		<SystemMessage
			{...props}
			icon={icMemberJoin}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.member_join",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

export function SystemMessagePinned(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	const navigate = useNavigate();
	const version = () =>
		props.message.latest_version as MessageVersionT & {
			pinned_message_id: string;
		};

	return (
		<SystemMessage
			{...props}
			icon={icPin}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.message_pinned",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						(text: string) => (
							<button
								type="button"
								style="color: oklch(var(--color-fg1))"
								class="link"
								onClick={(e) => {
									e.stopPropagation();
									navigate(
										`/channel/${props.message.channel_id}/message/${
											version().pinned_message_id
										}`,
									);
								}}
							>
								{text}
							</button>
						),
					)}
				</div>
			}
		/>
	);
}

export function SystemMessageChannelRename(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	const version = () =>
		props.message.latest_version as MessageVersionT & { name_new: string };

	return (
		<SystemMessage
			{...props}
			icon={icEdit}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.channel_rename",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						<b>{version().name_new}</b>,
					)}
				</div>
			}
		/>
	);
}

export function SystemMessageCall(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	const [voice, voiceActions] = useVoice();
	const getMe = useCurrentUser();
	const version = () =>
		props.message.latest_version as MessageVersionT & {
			ended_at?: string | null;
			participants: string[];
		};

	const participated = createMemo(() =>
		version().participants.includes(getMe()?.id ?? ""),
	);
	const duration = createMemo(() => {
		const { ended_at } = version();
		if (!ended_at) return null;
		return Date.parse(ended_at) - Date.parse(props.message.created_at);
	});

	const joined = () => voice.joinedChannelId === props.message.channel_id;

	const joinCall = () => {
		voiceActions.selectChannel(props.message.channel_id);
	};

	return (
		<SystemMessage
			{...props}
			icon={icCall}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					<Switch>
						<Match when={duration() !== null && !participated()}>
							{t(
								"message_content.call_missed",
								<span class="author">
									<UserDisplayName
										user_id={props.message.author_id}
										room_id={props.room_id}
										onClick
									/>
								</span>,
								<Duration ms={duration()!} dim={false} />,
							)}
						</Match>
						<Match when={duration() !== null}>
							{t(
								"message_content.call_ended",
								<span class="author">
									<UserDisplayName
										user_id={props.message.author_id}
										room_id={props.room_id}
										onClick
									/>
								</span>,
								<Duration ms={duration()!} dim={false} />,
							)}
						</Match>
						<Match when={true}>
							{t(
								"message_content.call_started",
								<span class="author">
									<UserDisplayName
										user_id={props.message.author_id}
										room_id={props.room_id}
										onClick
									/>
								</span>,
							)}
							<Show when={!joined()}>
								{" - "}
								<button
									type="button"
									class="button link"
									style="display:inline-block"
									onClick={joinCall}
								>
									{t("message_content.call_join")}
								</button>
							</Show>
						</Match>
					</Switch>
				</div>
			}
		/>
	);
}

export function SystemMessageChannelPingback(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	return (
		<SystemMessage
			{...props}
			icon={icReply}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.channel_pingback",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

export function SystemMessageChannelIcon(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	return (
		<SystemMessage
			{...props}
			icon={icEdit}
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.channel_icon",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
				</div>
			}
		/>
	);
}

export function SystemMessageThreadCreated(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	const navigate = useNavigate();
	const ctx = useCtx();
	const channels = useChannels();

	const threadId = () =>
		(props.message.latest_version as MessageVersionT & { thread_id: string })
			.thread_id;
	const thread = channels.use(threadId);

	const link = () => (
		<button
			type="button"
			class="link"
			onClick={(e) => {
				e.stopPropagation();
				if (threadId()) {
					navigate(`/channel/${threadId()}`);
				}
			}}
		>
			<Show
				when={thread()?.name}
				fallback={<em class="dim">unknown thread</em>}
			>
				{(name) => name()}
			</Show>
		</button>
	);

	const viewAll = (text: string) => (
		<button
			type="button"
			class="link"
			onClick={(e) => {
				if (!ctx.threadsView()) {
					e.stopPropagation();
					const ref = ctx.headerThreadsButtonRef() ?? e.currentTarget;
					queueMicrotask(() => {
						ctx.setThreadsView({
							channel_id: props.message.channel_id,
							ref,
						});
					});
				}
			}}
		>
			{text}
		</button>
	);

	return (
		<SystemMessage
			{...props}
			icon={icThread}
			class="message-dim-content"
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.thread_created",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						link,
						viewAll,
					)}
				</div>
			}
		/>
	);
}

export function SystemMessageChannelMoved(props: SystemMessageBaseProps) {
	const { t } = useCtx();
	const navigate = useNavigate();
	const channels = useChannels();

	const m = () =>
		props.message.latest_version as MessageVersionT & { type: "ChannelMoved" };
	const oldChan = channels.use(() => m().parent_id_old ?? undefined);

	return (
		<SystemMessage
			{...props}
			icon={icChannelMove}
			class="message-dim-content"
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.channel_moved",
						<span class="author">
							<UserDisplayName
								user_id={props.message.author_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
						<button
							type="button"
							class="link"
							onClick={(e) => {
								e.stopPropagation();
								const oldId = m().parent_id_old;
								if (oldId) {
									navigate(`/channel/${oldId}`);
								}
							}}
						>
							<Show when={oldChan()} fallback={<em>unknown channel</em>}>
								{(c) => c().name}
							</Show>
						</button>,
					)}
				</div>
			}
		/>
	);
}

// TODO: better component for automod executions
export function SystemMessageAutomodExecution(props: SystemMessageBaseProps) {
	const { t } = useCtx();

	const m = () =>
		props.message.latest_version as MessageVersionT & {
			type: "AutomodExecution";
		};

	// TODO: fix timestamp position
	// TODO: if automod acted on a message, render pseudo message and highlight phrases that triggered the action
	// TODO: if its not a message, still highlight matches? (how would the ui look in this case?)

	// m().matches.fragments[0].text

	const renderAction = (action: AutomodAction) => {
		switch (action.type) {
			case "Block":
				return <>Blocked</>;
			// TODO: better styling for Timeout duration
			// case "Timeout": return <>Timed out <Duration ms={action.duration} /></>
			case "Timeout":
				return <>Timed out</>;
			case "Remove":
				return <>Message removed</>;
			case "SendAlert":
				return null; // redundant
		}
	};

	return (
		<SystemMessage
			{...props}
			icon={icSword}
			class="message-dim-content"
			content={
				<div
					class="body markdown"
					classList={{ local: props.message.is_local }}
				>
					{/* @ts-ignore */}
					{t(
						"message_content.automod_execution",
						<span class="author">
							<UserDisplayName
								user_id={m().user_id}
								room_id={props.room_id}
								onClick
							/>
						</span>,
					)}
					<div class="automod-execution-details">
						<div class="rules">
							<strong>Rules: </strong>
							{m()
								.rules.map((i) => i.name)
								.join(", ")}
						</div>
						<div class="actions">
							<strong>Actions: </strong>
							{m()
								.actions.map(renderAction)
								.filter((i) => i)
								.flatMap((action, i, arr) =>
									i < arr.length - 1 ? [action, ", "] : [action],
								)}
						</div>
					</div>
				</div>
			}
		/>
	);
}
