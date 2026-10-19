import { throttle } from "@solid-primitives/scheduled";
import type { Channel, NotifsChannel, PreferencesChannel } from "sdk";
import {
	createResource,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import { useApi, useChannels, useRooms, useTags, useThreadMembers } from "@/api";
import { useCtx } from "@/app/context";
import { formatDuration } from "@/atoms/Duration";
import { Checkbox } from "@/atoms/icons";
import { Search } from "@/atoms/Search.tsx";
import { timeAgo } from "@/atoms/Time.tsx";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useModals } from "@/contexts/modal";
import { useReadTracking } from "@/contexts/read-tracking.tsx";
import { useNavigate } from "@/contexts/router.tsx";
import { usePermissions } from "@/hooks/usePermissions.ts";
import { getDate } from "@/utils/general.tsx";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";

// the context menu for channels
export function ChannelMenu(props: { channel_id: string }) {
	const ctx = useCtx();
	const channels2 = useChannels();
	const threadMembers2 = useThreadMembers();
	const tags2 = useTags();
	const { markCategoryRead } = useReadTracking();
	const readTracking = useReadTracking();
	const nav = useNavigate();
	const [, modalCtl] = useModals();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id ?? "";
	const channel = channels2.use(() => props.channel_id);
	const parentChan = channels2.use(
		() => (channel() as any)?.parent_id as string,
	);

	const { has: hasPermission } = usePermissions(
		self_id,
		() => channel()?.room_id ?? undefined,
		() => props.channel_id,
	);

	// FIXME: documents are threads if they are in wikis
	const isThread = () =>
		channel()?.type === "ThreadPublic" ||
		channel()?.type === "ThreadPrivate" ||
		channel()?.type === "ThreadForum2";

	const self_channel_member = threadMembers2.useMember(
		() => props.channel_id,
		self_id,
	);
	const copyId = () => navigator.clipboard.writeText(props.channel_id);
	const markRead = async () => {
		const c = channels2.cache.get(props.channel_id);
		if (!c) return;

		if (c.type === "Category") {
			markCategoryRead(props.channel_id);
		} else {
			const message_id = c.last_message_id;
			if (!message_id) return;
			readTracking.ack(props.channel_id, message_id, true, false);
		}
	};

	const removeChannel = () => {
		modalCtl.confirm(
			"are you sure you want to remove this channel?",
			(confirmed) => {
				if (!confirmed) return;
				ctx.client.http.PUT("/api/v1/channel/{channel_id}/remove", {
					params: {
						path: { channel_id: props.channel_id },
					},
				});
			},
		);
	};

	const copyLink = () => {
		const url = `${ctx.client.opts.apiUrl}/channel/${props.channel_id}`;
		navigator.clipboard.writeText(url);
	};

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(channel())));

	const settings = (to: string) => () =>
		nav(`/channel/${props.channel_id}/settings${to}`);

	const archiveChannel = () => {
		channels2.archive(props.channel_id);
	};

	const unarchiveChannel = () => {
		channels2.unarchive(props.channel_id);
	};

	const toggleLock = () => {
		if (channel()?.locked) {
			channels2.unlock(props.channel_id);
		} else {
			channels2.lock(props.channel_id);
		}
	};

	const toggleTag = (tagId: string) => {
		const c = channel();
		if (!c) return;
		const currentTags = c.tags || [];
		const newTags = currentTags.includes(tagId)
			? currentTags.filter((t) => t !== tagId)
			: [...currentTags, tagId];

		channels2.update(props.channel_id, { tags: newTags });
	};

	const joinOrLeaveChannel = () => {
		if (!self_channel_member()) {
			ctx.client.http.PUT("/api/v1/thread/{thread_id}/member/{user_id}", {
				params: {
					path: { thread_id: props.channel_id, user_id: "@self" },
				},
				body: {},
			});
		} else {
			ctx.client.http.DELETE("/api/v1/thread/{thread_id}/member/{user_id}", {
				params: {
					path: { thread_id: props.channel_id, user_id: "@self" },
				},
			});
		}
	};

	const [tagsResource] = createResource(
		() => {
			const c = channel();
			const p = parentChan();
			if (!c || !p || !c.room_id) return null;
			if (!p.type.includes("Forum")) return null;
			return p.id;
		},
		async (forumId) => {
			if (!forumId) return [];
			try {
				const result = await tags2.list(forumId, false);
				return result.items;
			} catch (e) {
				console.error("Failed to fetch tags:", e);
				return [];
			}
		},
	);

	// TODO: maybe extract search throttling into a hook?
	const [searchQuery, setSearchQuery] = createSignal("");
	const [throttledQuery, setThrottledQuery] = createSignal("");

	const updateThrottled = throttle((val: string) => {
		setThrottledQuery(val);
	}, 500);

	const onSearchInput = (val: string) => {
		setSearchQuery(val);
		updateThrottled(val);
	};

	const [tagsSearchResource] = createResource(
		() => ({
			forumId: tagsResource.state === "ready" ? parentChan()?.id : null,
			query: throttledQuery(),
		}),
		async ({ forumId, query }) => {
			if (!forumId) return [];
			if (!query.trim()) {
				return tagsResource.latest ?? [];
			}
			try {
				const result = await tags2.search(forumId, query, false);
				return result.items;
			} catch (e) {
				console.error("Failed to search tags:", e);
				return [];
			}
		},
	);

	const displayedTags = () => {
		if (searchQuery().trim()) {
			return tagsSearchResource.latest ?? [];
		}
		return tagsResource.latest ?? [];
	};

	const canApplyRestrictedTags = () =>
		hasPermission("ThreadEdit") || hasPermission("ThreadManage");

	let tagSearchEl!: HTMLInputElement;

	return (
		<Menu>
			<Item onClick={markRead}>mark as read</Item>
			<Item onClick={copyLink}>copy link</Item>
			<Show when={channel()}>
				{(c) => <ChannelNotificationMenu channel={c()} />}
			</Show>
			<Show when={hasPermission("InviteCreate") && channel()?.type !== "Dm"}>
				<Item
					onClick={() =>
						modalCtl.open({
							type: "invite_create",
							channel_id: props.channel_id,
							room_id: channel()?.room_id ?? undefined,
						})
					}
				>
					create invite
				</Item>
			</Show>
			<Show when={channel() && isThread()}>
				<Item onClick={joinOrLeaveChannel}>
					{self_channel_member() ? "leave" : "join"}
				</Item>
			</Show>
			<Separator />
			<Show when={channel()?.type !== "Dm"}>
				<Submenu content={"edit"} onClick={settings("")}>
					{/* TODO: what permissions should be required to see the info and permissions items? */}
					{/* TODO: hide edit menu if no children are visible*/}
					<Item onClick={settings("")}>info</Item>
					<Show when={hasPermission("RoleManage")}>
						<Item onClick={settings("/permissions")}>permissions</Item>
					</Show>
					<Show when={hasPermission("InviteManage")}>
						<Item onClick={settings("/invites")}>invites</Item>
					</Show>
					<Show
						when={
							hasPermission("RoleManage") &&
							(channel()?.type === "Forum" || channel()?.type === "Forum2")
						}
					>
						<Item onClick={settings("/tags")}>tags</Item>
					</Show>
					<Show
						when={
							hasPermission("IntegrationsManage") &&
							(channel()?.type === "Text" || channel()?.type === "Announcement")
						}
					>
						<Item onClick={settings("/webhooks")}>webhooks</Item>
					</Show>
				</Submenu>
			</Show>
			<Show
				when={
					channel() &&
					isThread() &&
					(parentChan()?.type === "Forum" || parentChan()?.type === "Forum2")
					// TODO: only show when tag_count > 0
				}
			>
				<Submenu content={"tags"} onOpen={() => tagSearchEl.focus()}>
					<div class="channel-menu-tags" onClick={(e) => e.stopPropagation()}>
						<Search
							ref={tagSearchEl}
							value={searchQuery}
							onInput={onSearchInput}
							placeholder="search tags..."
						/>
						<Show when={tagsResource.loading}>
							<div>loading tags...</div>
						</Show>
						<For each={displayedTags()}>
							{(tag) => {
								const isRestricted = () => tag.restricted ?? false;
								const isDisabled = () =>
									isRestricted() && !canApplyRestrictedTags();
								const isChecked = () =>
									channel()?.tags?.includes(tag.id) ?? false;
								// TODO: optimistic ui
								return (
									<Item
										disabled={isDisabled()}
										onClick={() => {
											if (!isDisabled()) {
												toggleTag(tag.id);
											}
										}}
									>
										<div style="display: flex; align-items: start; gap: 8px">
											<Checkbox
												checked={isChecked()}
												seed={`menu-channel-${props.channel_id}-tag-${tag.id}`}
											/>
											<div style="margin: 2px 0">
												<div classList={{ has: isChecked() }}>
													{tag.name}
													{isRestricted() && (
														<span
															class="dim"
															style="margin-left: 4px; font-size: 0.8em;"
														>
															(restricted)
														</span>
													)}
												</div>
												<Show when={tag.description}>
													<div class="dim">{tag.description}</div>
												</Show>
											</div>
										</div>
									</Item>
								);
							}}
						</For>
						<Show
							when={
								tagsResource.state === "ready" && displayedTags().length === 0
							}
						>
							<Item disabled>no tags available</Item>
						</Show>
					</div>
				</Submenu>
			</Show>
			<Show when={channel() && isThread()}>
				<Switch>
					<Match when={!channel()?.archived_at}>
						<Item onClick={archiveChannel}>archive</Item>
					</Match>
					<Match when={channel()?.archived_at}>
						<Item onClick={unarchiveChannel}>unarchive</Item>
					</Match>
				</Switch>
			</Show>
			<Show when={hasPermission("ThreadManage")}>
				<Item onClick={toggleLock}>
					{channel()?.locked ? "unlock" : "lock"}
				</Item>
			</Show>
			<Show
				when={
					isThread()
						? hasPermission("ThreadManage")
						: hasPermission("ChannelManage")
				}
			>
				<Item onClick={removeChannel} color="danger">
					remove
				</Item>
			</Show>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}

export function ChannelNotificationMenu(props: { channel: Channel }) {
	const ctx = useCtx();
	const { t } = ctx;
	const api2 = useApi();
	const channels2 = useChannels();
	const rooms2 = useRooms();
	const channelConfig = () => props.channel.preferences;
	const room = rooms2.use(() => props.channel.room_id ?? "");

	// a channel without an override inherits from its room, which in turn
	// inherits from the user's global preferences.
	const defaultMessages = () =>
		room()?.preferences?.notifs.messages ?? ctx.preferences().notifs.messages;
	const defaultThreads = () =>
		room()?.preferences?.notifs.threads ?? ctx.preferences().notifs.threads;

	const setNotifs = (notifs: Partial<NotifsChannel>) => {
		const current = channelConfig();
		const newConfig: PreferencesChannel = {
			notifs: { ...(current?.notifs ?? {}), ...notifs },
			frontend: current?.frontend ?? {},
		};
		for (const key in newConfig.notifs) {
			if (
				newConfig.notifs[key as keyof typeof newConfig.notifs] === undefined
			) {
				delete newConfig.notifs[key as keyof typeof newConfig.notifs];
			}
		}
		channels2.cache.set(props.channel.id, {
			...props.channel,
			preferences: newConfig,
		});
		api2.preferences
			.setChannelPreferences(props.channel.id, newConfig)
			.catch((e) => {
				console.error("Failed to update channel preferences", e);
			});
	};

	const setMute = (duration_ms: number | null) => {
		const expires_at =
			duration_ms === null
				? null
				: new Date(Date.now() + duration_ms).toISOString();
		setNotifs({ mute: { expires_at } });
	};

	const unmute = () => setNotifs({ mute: undefined });

	const isMuted = () => {
		const c = channelConfig();
		if (!c?.notifs.mute) return false;
		if (!c.notifs.mute.expires_at) return true;
		return getDate(c.notifs.mute.expires_at) > Date.now();
	};

	const fifteen_mins = 15 * 60 * 1000;
	const three_hours = 3 * 60 * 60 * 1000;
	const eight_hours = 8 * 60 * 60 * 1000;
	const one_day = 24 * 60 * 60 * 1000;
	const one_week = 7 * one_day;

	return (
		<>
			<Submenu content={t("notifs.title")}>
				<Item
					onClick={() => setNotifs({ messages: undefined })}
					classList={{
						selected: channelConfig()?.notifs.messages === undefined,
					}}
				>
					<div>{t("notifs.default")}</div>
					<div class="subtext">
						{t("notifs.default_messages_room", {
							value: defaultMessages().toLowerCase(),
						})}
					</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Everything" })}
					classList={{
						selected: channelConfig()?.notifs.messages === "Everything",
					}}
				>
					<div>{t("notifs.everything")}</div>
					<div class="subtext">{t("notifs.everything_channel")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Watching" })}
					classList={{
						selected: channelConfig()?.notifs.messages === "Watching",
					}}
				>
					<div>{t("notifs.watching")}</div>
					<div class="subtext">{t("notifs.watching_channel")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Mentions" })}
					classList={{
						selected: channelConfig()?.notifs.messages === "Mentions",
					}}
				>
					<div>{t("notifs.mentions")}</div>
					<div class="subtext">{t("notifs.mentions_description")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Nothing" })}
					classList={{
						selected: channelConfig()?.notifs.messages === "Nothing",
					}}
				>
					<div>{t("notifs.nothing")}</div>
					<div class="subtext">{t("notifs.nothing_description")}</div>
				</Item>
				<Separator />
				<Item
					onClick={() => setNotifs({ threads: undefined })}
					classList={{
						selected: channelConfig()?.notifs.threads === undefined,
					}}
				>
					<div>{t("notifs.default")}</div>
					<div class="subtext">
						{t("notifs.default_threads_room", {
							value: defaultThreads().toLowerCase(),
						})}
					</div>
				</Item>
				<Item
					onClick={() => setNotifs({ threads: "Notify" })}
					classList={{ selected: channelConfig()?.notifs.threads === "Notify" }}
				>
					<div>{t("notifs.new_threads")}</div>
					<div class="subtext">{t("notifs.new_threads_channel")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ threads: "Inbox" })}
					classList={{ selected: channelConfig()?.notifs.threads === "Inbox" }}
				>
					<div>{t("notifs.threads_to_inbox")}</div>
					<div class="subtext">{t("notifs.threads_to_inbox_description")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ threads: "Nothing" })}
					classList={{
						selected: channelConfig()?.notifs.threads === "Nothing",
					}}
				>
					<div>{t("notifs.ignore_threads")}</div>
					<div class="subtext">{t("notifs.ignore_threads_description")}</div>
				</Item>
				<Separator />
				<Item>{t("notifs.bookmark")}</Item>
				<Submenu content={t("notifs.remind_me")}>
					<Item>{timeAgo(new Date(Date.now() + fifteen_mins))}</Item>
					<Item>{timeAgo(new Date(Date.now() + three_hours))}</Item>
					<Item>{timeAgo(new Date(Date.now() + eight_hours))}</Item>
					<Item>{timeAgo(new Date(Date.now() + one_day))}</Item>
					<Item>{timeAgo(new Date(Date.now() + one_week))}</Item>
				</Submenu>
			</Submenu>
			<Show
				when={isMuted()}
				fallback={
					<Submenu content={t("notifs.mute")} onClick={() => setMute(null)}>
						<Item onClick={() => setMute(fifteen_mins)}>
							{t("notifs.for_duration", {
								duration: formatDuration(fifteen_mins),
							})}
						</Item>
						<Item onClick={() => setMute(three_hours)}>
							{t("notifs.for_duration", {
								duration: formatDuration(three_hours),
							})}
						</Item>
						<Item onClick={() => setMute(eight_hours)}>
							{t("notifs.for_duration", {
								duration: formatDuration(eight_hours),
							})}
						</Item>
						<Item onClick={() => setMute(one_day)}>
							{t("notifs.for_duration", { duration: formatDuration(one_day) })}
						</Item>
						<Item onClick={() => setMute(one_week)}>
							{t("notifs.for_duration", { duration: formatDuration(one_week) })}
						</Item>
						<Item onClick={() => setMute(null)}>{t("notifs.forever")}</Item>
					</Submenu>
				}
			>
				<Item onClick={unmute}>
					<div>{t("notifs.unmute")}</div>
					<Show when={channelConfig()?.notifs.mute?.expires_at}>
						<div class="subtext">
							{t("notifs.unmutes", {
								time: timeAgo(
									getDate(channelConfig()?.notifs.mute?.expires_at!),
								),
							})}
						</div>
					</Show>
				</Item>
			</Show>
		</>
	);
}

export function TopicMenu(props: { channel_id: string }) {
	const channels2 = useChannels();
	const channel = channels2.use(() => props.channel_id);
	const nav = useNavigate();
	const currentUser = useCurrentUser();

	const { has: hasPermission } = usePermissions(
		() => currentUser()?.id,
		() => channel()?.room_id ?? undefined,
		() => props.channel_id,
	);

	const canEdit = () => hasPermission("ChannelEdit");

	const copyTopic = () => {
		navigator.clipboard.writeText(channel()?.description ?? "");
	};

	const editTopic = () => {
		nav(`/channel/${props.channel_id}/settings`);
	};

	return (
		<Menu>
			<Show when={canEdit()}>
				<Item onClick={editTopic}>edit</Item>
			</Show>
			<Item onClick={copyTopic}>copy text</Item>
		</Menu>
	);
}
