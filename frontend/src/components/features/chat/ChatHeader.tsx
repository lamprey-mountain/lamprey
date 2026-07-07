import type { Channel } from "sdk";
import { createSignal, Match, Show, Switch } from "solid-js";
import { useChannels, useMessages } from "@/api";
import { useCtx } from "@/app/context";
import icCall from "@/assets/call.png";
import icMembers from "@/assets/members.png";
import icPin from "@/assets/pin.png";
import icThreads from "@/assets/threads.png";
import icCancel from "@/assets/x.png";
import icDelete from "@/assets/delete.png";
import icRemove from "@/assets/emoji-symbols.png"; // TEMP: get a better icon
import { Icon } from "@/atoms/Icon";
import { SearchInput } from "@/components/features/search/SearchInput";
import { ChannelIcon } from "@/components/shared/User";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useMenu } from "@/contexts/menu.tsx";
import { useModals } from "@/contexts/modal.tsx";
import { usePermissions } from "@/hooks/usePermissions.ts";
import { md } from "@/lib/markdown";

type ChatHeaderProps = {
	channel: Channel;
	showMembersButton?: boolean;
};

export const ChatHeader = (props: ChatHeaderProps) => {
	const ctx = useCtx();
	const channels2 = useChannels();
	const messagesService = useMessages();
	const [channelState, setChannelState] = useChannel()!;
	const [, modalctl] = useModals();
	const { setMenu } = useMenu();
	const [hovered, setHovered] = createSignal(false);
	const currentUser = useCurrentUser();
	const [editingName, setEditingName] = createSignal<string | undefined>();
	let inputRef!: HTMLInputElement;

	const selected = () => channelState.selectedMessages;
	const isSelecting = () => channelState.selectMode;

	const { has: hasPermission } = usePermissions(
		() => currentUser()?.id as string | undefined,
		() => props.channel.room_id as string | undefined,
		() => props.channel.id,
	);

	const isThread = () =>
		props.channel.type === "ThreadPublic" ||
		props.channel.type === "ThreadPrivate" ||
		props.channel.type === "ThreadForum2";

	const canEditChannelName = () => {
		if (!isThread()) return hasPermission("ChannelManage");
		// TODO: can edit if current user created the thread and thread isn't locked
		// TODO: can edit if current user has ThreadEdit and thread isn't locked
		// TODO: can edit if current user has ThreadManage
		return hasPermission("ThreadManage");
	};

	const canDeleteMessages = () => hasPermission("MessageDelete");
	const canRemoveMessages = () => hasPermission("MessageRemove");

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
		if (!canEditChannelName()) return;
		setEditingName(name());
		setTimeout(() => inputRef.focus());
	};

	// TODO: show new channel name with class=".local" while waiting for the patch request to go through
	// const [isSaving, setIsSaving] = createSignal(false);

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
			e.stopPropagation();
			saveName();
		} else if (e.key === "Escape") {
			cancelEditingName();
		}
	};

	const name = () => {
		if (props.channel.type === "Dm") {
			const user_id = currentUser()?.id;
			return (
				props.channel.recipients?.find((i) => i.id !== user_id)?.name ?? "dm"
			);
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
		<header
			class="chat-header"
			classList={{
				selecting: isSelecting(),
				deleted: !!props.channel.deleted_at,
			}}
			onMouseEnter={() => setHovered(true)}
			onMouseLeave={() => setHovered(false)}
		>
			<div class="channel-icon">
				<ChannelIcon channel={props.channel} animate={hovered()} />
			</div>

			<div
				class="name"
				classList={{
					editable: !isSelecting() && canEditChannelName(),
					editing: editingName() !== undefined,
				}}
				tabindex="0"
				onClick={startEditingName}
				onKeyDown={(e) => {
					if (e.key === "Enter") {
						e.preventDefault();
						startEditingName();
					}
				}}
				title={canEditChannelName() ? "Click to edit channel name" : undefined}
			>
				<Switch>
					<Match when={isSelecting()}>
						<h3 class="name-text">{selected().length} message(s) selected</h3>
					</Match>
					<Match when={editingName() !== undefined}>
						<input
							ref={inputRef}
							placeholder="awesome-channel"
							class="name-input"
							type="text"
							value={editingName()}
							onInput={(e) => setEditingName(e.currentTarget.value)}
							onBlur={saveName}
							onKeyDown={handleKeyDown}
						/>
					</Match>
					<Match when={true}>
						<h3 class="name-text">{name()}</h3>
					</Match>
				</Switch>
			</div>

			<Show when={props.channel.description}>
				{/*<span class="dim" style="white-space:pre;font-size:1em">*/}
				<div>{"  -  "}</div>
				<div
					class="topic"
					innerHTML={md(props.channel.description ?? "") as string}
					onClick={() => {
						// TODO: extract into function
						if (props.channel.description) {
							modalctl.open({
								type: "channel_topic",
								channel_id: props.channel.id,
							});
						}
					}}
					onContextMenu={(e) => {
						// TODO: extract into function
						e.preventDefault();
						queueMicrotask(() => {
							setMenu({
								x: e.clientX,
								y: e.clientY,
								type: "topic",
								channel_id: props.channel.id,
							});
						});
					}}
					title="click to view topic"
				></div>
			</Show>
			<div class="spacer"></div>
			{/* TODO: tooltips */}
			<Show when={isSelecting()}>
				<menu class="menu">
					{/* TODO: forwarding selected messages */}
					<Show when={canDeleteMessages()}>
						<button
							type="button"
							class="danger"
							onClick={deleteSelected}
							title="delete"
						>
							<Icon src={icDelete} color={null} />
						</button>
					</Show>
					<Show when={canRemoveMessages()}>
						<button
							type="button"
							class="danger"
							onClick={removeSelected}
							title="remove"
						>
							<Icon src={icRemove} color={null} />
						</button>
					</Show>
					<button type="button" onClick={exitSelectMode} title="cancel">
						<Icon src={icCancel} color={null} />
					</button>
				</menu>
			</Show>
			<Show when={!isSelecting()}>
				<SearchInput channel={props.channel} />
				<menu class="menu">
					<Show
						when={props.channel.type === "Dm" || props.channel.type === "Gdm"}
					>
						<button
							type="button"
							onClick={() => {
								// TODO: calling
							}}
							title="Start call"
						>
							<Icon src={icCall} color={null} />
						</button>
					</Show>
					<Show
						when={
							props.channel.type === "Text" ||
							props.channel.type === "Announcement" ||
							props.channel.type === "Gdm"
						}
					>
						<button
							type="button"
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
							<Icon src={icThreads} color={null} />
						</button>
					</Show>
					<button
						type="button"
						onClick={togglePinned}
						classList={{ active: isShowingPinned() }}
						title="Show pinned messages"
						style={{ display: hasPins() ? undefined : "none" }}
					>
						<Icon src={icPin} color={null} />
					</button>
					<Show when={props.showMembersButton ?? true}>
						<button type="button" onClick={toggleMembers} title="Show members">
							<Icon src={icMembers} color={null} />
						</button>
					</Show>
				</menu>
			</Show>
		</header>
	);
};
