import { useNavigate } from "@solidjs/router";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { usePermissions } from "../hooks/usePermissions.ts";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import { timeAgo } from "../Time.tsx";
import { Channel, Tag } from "sdk";
import { useModals } from "../contexts/modal";
import { Checkbox } from "../icons.tsx";

// the context menu for channels
export function ChannelMenu(props: { channel_id: string }) {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const [, modalCtl] = useModals();

	const self_id = () => api.users.cache.get("@self")!.id;
	const channel = api.channels.fetch(() => props.channel_id);
	const parentChan = api.channels.fetch(() => channel()?.parent_id);

	const { has: hasPermission } = usePermissions(
		self_id,
		() => channel()?.room_id ?? undefined,
		() => props.channel_id,
	);

	// FIXME: documents are threads if they are in wikis
	const isThread = () =>
		channel()?.type === "ThreadPublic" || channel()?.type === "ThreadPrivate" ||
		channel()?.type === "ThreadForum2";

	const self_channel_member = api.thread_members.fetch(
		() => props.channel_id,
		self_id,
	);
	const copyId = () => navigator.clipboard.writeText(props.channel_id);
	const markRead = async () => {
		const channel = api.channels.cache.get(props.channel_id)!;

		if (channel.type === "Category") {
			ctx.dispatch({
				do: "category.mark_read",
				category_id: props.channel_id,
			});
		} else {
			const version_id = channel.last_version_id;
			if (!version_id) return;
			ctx.dispatch({
				do: "thread.mark_read",
				thread_id: props.channel_id,
				also_local: true,
				version_id,
			});
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
		api.channels.archive(props.channel_id);
	};

	const unarchiveChannel = () => {
		api.channels.unarchive(props.channel_id);
	};

	const toggleLock = () => {
		if (channel()?.locked) {
			api.channels.unlock(props.channel_id);
		} else {
			api.channels.lock(props.channel_id);
		}
	};

	const toggleTag = (tagId: string) => {
		const c = channel();
		if (!c) return;
		const currentTags = c.tags || [];
		const newTags = currentTags.includes(tagId)
			? currentTags.filter((t) => t !== tagId)
			: [...currentTags, tagId];

		api.channels.update(props.channel_id, { tags: newTags });
	};

	const joinOrLeaveChannel = () => {
		if (self_channel_member()?.membership === "Leave") {
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

	const [tagsResource] = createResource(() => {
		const c = channel();
		const p = parentChan();
		if (!c || !p || !c.room_id) return null;
		if (!p.type.includes("Forum")) return null;
		return p.id;
	}, async (forumId) => {
		if (!forumId) return [];
		try {
			const result = await api.tags.list(forumId, false);
			return result.items;
		} catch (e) {
			console.error("Failed to fetch tags:", e);
			return [];
		}
	});

	const [tagSearchQuery, setTagSearchQuery] = createSignal("");
	const [tagsSearchResource] = createResource(
		() => ({
			forumId: tagsResource.state === "ready" ? parentChan()?.id : null,
			query: tagSearchQuery(),
		}),
		async ({ forumId, query }) => {
			if (!forumId) return [];
			if (!query.trim()) {
				return tagsResource.latest ?? [];
			}
			try {
				// FIXME: throttle
				const result = await api.tags.search(forumId, query, false);
				return result.items;
			} catch (e) {
				console.error("Failed to search tags:", e);
				return [];
			}
		},
	);

	const displayedTags = () => {
		if (tagSearchQuery().trim()) {
			return tagsSearchResource.latest ?? [];
		}
		return tagsResource.latest ?? [];
	};

	const canApplyRestrictedTags = () =>
		hasPermission("ThreadEdit") || hasPermission("ThreadManage");

	let tagSearchInputRef: HTMLInputElement | undefined;

	return (
		<Menu>
			<Item onClick={markRead}>mark as read</Item>
			<Item onClick={copyLink}>copy link</Item>
			<Show when={channel()}>
				{(c) => <ChannelNotificationMenu channel={c()} />}
			</Show>
			<Show when={channel() && isThread()}>
				<Item onClick={joinOrLeaveChannel}>
					{self_channel_member()?.membership === "Leave" ? "join" : "leave"}
				</Item>
			</Show>
			<Separator />
			<Submenu content={"edit"} onClick={settings("")}>
				<Item onClick={settings("")}>info</Item>
				<Item onClick={settings("/permissions")}>permissions</Item>
				<Item onClick={settings("/invites")}>invites</Item>
				<Item onClick={settings("/webhooks")}>webhooks</Item>
			</Submenu>
			<Show
				when={channel() && isThread() &&
					(parentChan()?.type === "Forum" || parentChan()?.type === "Forum2")}
			>
				<Submenu content={"tags"} onOpen={() => tagSearchInputRef?.focus()}>
					<div class="tags">
						<input
							ref={tagSearchInputRef}
							class="tags-search"
							type="search"
							placeholder="search tags..."
							value={tagSearchQuery()}
							onInput={(e) => setTagSearchQuery(e.currentTarget.value)}
							onClick={(e) => e.stopPropagation()}
						/>
						<Show when={tagsResource.loading}>
							<div>loading tags...</div>
						</Show>
						<For each={displayedTags()}>
							{(tag) => {
								const isRestricted = tag.restricted ?? false;
								const isDisabled = isRestricted && !canApplyRestrictedTags();
								const isChecked = channel()?.tags?.includes(tag.id) ?? false;
								return (
									<Item
										disabled={isDisabled}
										onClick={(e) => {
											e.stopPropagation();
											if (!isDisabled) {
												toggleTag(tag.id);
											}
										}}
									>
										<div style="display: flex; align-items: start; gap: 8px">
											<Checkbox checked={isChecked} />
											<div style="margin: 2px 0">
												<div classList={{ has: isChecked }}>
													{tag.name}
													{isRestricted && (
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
							when={tagsResource.state === "ready" &&
								displayedTags().length === 0}
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
			<Show when={hasPermission("ThreadLock")}>
				<Item onClick={toggleLock}>
					{channel()?.locked ? "unlock" : "lock"}
				</Item>
			</Show>
			<Show
				when={isThread()
					? hasPermission("ThreadManage")
					: hasPermission("ChannelManage")}
			>
				<Item onClick={removeChannel} color="danger">remove</Item>
			</Show>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}

function ChannelNotificationMenu(props: { channel: Channel }) {
	const api = useApi();
	const channelConfig = () => props.channel.user_config;

	const setNotifs = (notifs: Partial<NotifsChannel>) => {
		const current = channelConfig() ?? { notifs: {}, frontend: {} };
		const newConfig = {
			...current,
			notifs: { ...current.notifs, ...notifs },
		};
		for (const key in newConfig.notifs) {
			if (
				newConfig.notifs[key as keyof typeof newConfig.notifs] === undefined
			) {
				delete newConfig.notifs[key as keyof typeof newConfig.notifs];
			}
		}
		api.channels.cache.set(props.channel.id, {
			...props.channel,
			user_config: newConfig,
		});
		api.client.http.PUT("/api/v1/config/thread/{thread_id}", {
			params: { path: { thread_id: props.channel.id } },
			body: newConfig,
		});
	};

	const setMute = (duration_ms: number | null) => {
		const expires_at = duration_ms === null
			? null
			: new Date(Date.now() + duration_ms).toISOString();
		setNotifs({ mute: { expires_at } });
	};

	const unmute = () => setNotifs({ mute: undefined });

	const isMuted = () => {
		const c = channelConfig();
		if (!c?.notifs.mute) return false;
		if (c.notifs.mute.expires_at === null) return true;
		return Date.parse(c.notifs.mute.expires_at) > Date.now();
	};

	const fifteen_mins = 15 * 60 * 1000;
	const three_hours = 3 * 60 * 60 * 1000;
	const eight_hours = 8 * 60 * 60 * 1000;
	const one_day = 24 * 60 * 60 * 1000;
	const one_week = 7 * one_day;

	return (
		<>
			<Submenu content={"notifications"}>
				<Item
					onClick={() => setNotifs({ messages: undefined, threads: undefined })}
				>
					<div>default</div>
					<div class="subtext">
						Uses the room's default notification setting.
					</div>
				</Item>
				<Item onClick={() => setNotifs({ messages: "Everything" })}>
					<div>everything</div>
					<div class="subtext">
						You will be notified of all new messages in this channel.
					</div>
				</Item>
				<Item onClick={() => setNotifs({ messages: "Watching" })}>
					<div>watching</div>
					<div class="subtext">
						Messages in this channel will show up in your inbox.
					</div>
				</Item>
				<Item onClick={() => setNotifs({ messages: "Mentions" })}>
					<div>mentions</div>
					<div class="subtext">You will only be notified on @mention</div>
				</Item>
				<Item onClick={() => setNotifs({ messages: "Nothing" })}>
					<div>nothing</div>
					<div class="subtext">You won't be notified for anything.</div>
				</Item>
				<Separator />
				<Item onClick={() => setNotifs({ threads: "Notify" })}>
					<div>new threads</div>
					<div class="subtext">
						You will be notified when a new thread is created.
					</div>
				</Item>
				<Item onClick={() => setNotifs({ threads: "Inbox" })}>
					<div>threads to inbox</div>
					<div class="subtext">New threads will show up in your inbox.</div>
				</Item>
				<Item onClick={() => setNotifs({ threads: "Nothing" })}>
					<div>ignore threads</div>
					<div class="subtext">You won't be notified for new threads.</div>
				</Item>
				<Separator />
				<Item>bookmark</Item>
				<Submenu content={"remind me"}>
					<Item>in 15 minutes</Item>
					<Item>in 3 hours</Item>
					<Item>in 8 hours</Item>
					<Item>in 1 day</Item>
					<Item>in 1 week</Item>
				</Submenu>
			</Submenu>
			<Show
				when={isMuted()}
				fallback={
					<Submenu content={"mute"} onClick={() => setMute(null)}>
						<Item onClick={() => setMute(fifteen_mins)}>for 15 minutes</Item>
						<Item onClick={() => setMute(three_hours)}>for 3 hours</Item>
						<Item onClick={() => setMute(eight_hours)}>for 8 hours</Item>
						<Item onClick={() => setMute(one_day)}>for 1 day</Item>
						<Item onClick={() => setMute(one_week)}>for 1 week</Item>
						<Item onClick={() => setMute(null)}>forever</Item>
					</Submenu>
				}
			>
				<Item onClick={unmute}>
					<div>unmute</div>
					<Show when={channelConfig()?.notifs.mute?.expires_at}>
						<div class="subtext">
							unmutes {timeAgo(
								new Date(Date.parse(channelConfig()!.notifs.mute!.expires_at!)),
							)}
						</div>
					</Show>
				</Item>
			</Show>
		</>
	);
}
