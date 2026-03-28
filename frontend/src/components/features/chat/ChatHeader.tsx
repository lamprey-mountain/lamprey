import { createSignal, Match, Show, Switch } from "solid-js";
import type { Channel } from "sdk";
import { useCtx } from "../../../context.ts";
import { useChannels2, useMessages2 } from "@/api";
import { useChannel } from "../../../channelctx.tsx";
import { useCurrentUser } from "../../../contexts/currentUser.tsx";
import { useModals } from "../../../contexts/modal.tsx";
import { SearchInput } from "./SearchInput.tsx";
import { md } from "../../../markdown_utils.tsx";
import { ChannelIcon } from "../../../User.tsx";
import { usePermissions } from "../../../hooks/usePermissions.ts";
import icPin from "../../../assets/pin.png";
import icMembers from "../../../assets/members.png";
import icCall from "../../../assets/call.png";
import icThreads from "../../../assets/threads.png";

type ChatHeaderProps = {
	channel: Channel;
	showMembersButton?: boolean;
};

export const ChatHeader = (
	props: ChatHeaderProps,
) => {
	const ctx = useCtx();
	const channels2 = useChannels2();
	const messagesService = useMessages2();
	const [channelState, setChannelState] = useChannel()!;
	const [, modalctl] = useModals();
	const [hovered, setHovered] = createSignal(false);
	const currentUser = useCurrentUser();
	const [editingName, setEditingName] = createSignal<string | undefined>();
	let inputRef!: HTMLInputElement;

	const selected = () => channelState.selectedMessages;
	const inSelectMode = () => channelState.selectMode;

	const { has: hasPermission } = usePermissions(
		() => currentUser()?.id as string | undefined,
		() => props.channel.room_id as string | undefined,
		() => props.channel.id,
	);
	const canManageChannel = () => hasPermission("ChannelManage");

	const canDelete = () => hasPermission("MessageDelete");
	const canRemove = () => hasPermission("MessageRemove");

	const exitSelectMode = () => {
		setChannelState("selectMode", false);
		setChannelState("selectedMessages", []);
	};

	const deleteSelected = () => {
		modalctl.confirm(
			`Are you sure you want to delete ${selected().length} messages?`,
			(confirmed) => {
				if (!confirmed) return;
				messagesService.deleteBulk(props.channel.id, selected());
				exitSelectMode();
			},
		);
	};

	const removeSelected = () => {
		modalctl.confirm(
			`Are you sure you want to remove ${selected().length} messages?`,
			(confirmed) => {
				if (!confirmed) return;
				messagesService.removeBulk(props.channel.id, selected());
				exitSelectMode();
			},
		);
	};

	const toggleMembers = () => {
		const c = ctx.preferences();
		ctx.setPreferences({
			...c,
			frontend: {
				...c.frontend,
				showMembers: !(c.frontend.showMembers ?? true),
			},
		});
	};

	const isShowingPinned = () => channelState.pinned_view;

	const togglePinned = () => {
		setChannelState("pinned_view", (v) => !v);
	};

	const startEditingName = () => {
		if (!canManageChannel()) return;
		setEditingName(name());
		setTimeout(() => inputRef.focus());
	};

	const saveName = () => {
		const newName = editingName()?.trim();
		if (newName && newName !== name()) {
			channels2.update(props.channel.id, { name: newName });
		}
		setEditingName(undefined);
	};

	const cancelEditingName = () => {
		setEditingName(undefined);
	};

	const handleKeyDown = (e: KeyboardEvent) => {
		if (e.key === "Enter") {
			e.preventDefault();
			saveName();
		} else if (e.key === "Escape") {
			cancelEditingName();
		}
	};

	const name = () => {
		if (props.channel.type === "Dm") {
			const user_id = currentUser()?.id;
			return props.channel.recipients?.find((i) => i.id !== user_id)?.name ??
				"dm";
		}

		return props.channel.name;
	};

	const hasPins = () =>
		props.channel.type === "Text" ||
		props.channel.type === "ThreadPublic" ||
		props.channel.type === "ThreadPrivate" ||
		props.channel.type === "ThreadForum2" ||
		props.channel.type === "Announcement" ||
		props.channel.type === "Dm" ||
		props.channel.type === "Gdm" ||
		props.channel.type === "Voice" ||
		props.channel.type === "Broadcast";

	return (
		<Show
			when={inSelectMode()}
			fallback={
				<header
					class="chat-header"
					style="display:flex"
					onMouseEnter={() => setHovered(true)}
					onMouseLeave={() => setHovered(false)}
				>
					<ChannelIcon channel={props.channel} animate={hovered()} />
					<Show
						when={editingName() !== undefined}
						fallback={
							<b
								class="channel-name-display"
								onClick={startEditingName}
								title={canManageChannel()
									? "Click to edit channel name"
									: undefined}
								style={canManageChannel() ? "cursor:pointer" : undefined}
							>
								{name()}
							</b>
						}
					>
						<input
							ref={inputRef}
							class="channel-name-input"
							type="text"
							value={editingName()}
							onInput={(e) => setEditingName(e.currentTarget.value)}
							onBlur={saveName}
							onKeyDown={handleKeyDown}
						/>
					</Show>
					<Show when={props.channel.description}>
						<span class="dim" style="white-space:pre;font-size:1em">
							{"  -  "}
						</span>
						<span
							class="markdown channel-topic-clickable"
							innerHTML={md(props.channel.description ?? "") as string}
							onClick={() => {
								if (props.channel.description) {
									modalctl.open({
										type: "channel_topic",
										channel_id: props.channel.id,
									});
								}
							}}
							title="click to view topic"
						>
						</span>
					</Show>
					<Switch>
						<Match when={props.channel.deleted_at}>{" (removed)"}</Match>
						<Match when={props.channel.archived_at}>{" (archived)"}</Match>
					</Switch>
					<div style="flex:1"></div>
					<SearchInput channel={props.channel} />
					<Show
						when={props.channel.type === "Dm" || props.channel.type === "Gdm"}
					>
						<button
							onClick={() => {
								// TODO: calling
							}}
							title="Start call"
						>
							<img class="icon" src={icCall} />
						</button>
					</Show>
					<Show
						when={props.channel.type === "Text" ||
							props.channel.type === "Announcement" ||
							props.channel.type === "Gdm"}
					>
						<button
							onClick={(e) => {
								if (!ctx.threadsView()) {
									const ref = e.currentTarget;
									setTimeout(() => {
										ctx.setThreadsView({
											channel_id: props.channel.id,
											ref,
										});
									});
								}
							}}
							title="Threads"
						>
							<img class="icon" src={icThreads} />
						</button>
					</Show>
					<button
						onClick={togglePinned}
						classList={{ active: isShowingPinned() }}
						title="Show pinned messages"
						style={!hasPins() ? "display:none" : undefined}
					>
						<img class="icon" src={icPin} />
					</button>
					<Show when={props.showMembersButton ?? true}>
						<button
							onClick={toggleMembers}
							title="Show members"
						>
							<img class="icon" src={icMembers} />
						</button>
					</Show>
				</header>
			}
		>
			<header class="chat-header select-mode-header" style="display:flex">
				<ChannelIcon channel={props.channel} />
				<span>{selected().length} selected</span>
				<div style="flex:1"></div>
				<Show when={canDelete()}>
					<button onClick={deleteSelected}>Delete</button>
				</Show>
				<Show when={canRemove()}>
					<button onClick={removeSelected}>Remove</button>
				</Show>
				<button onClick={exitSelectMode}>Cancel</button>
			</header>
		</Show>
	);
};
