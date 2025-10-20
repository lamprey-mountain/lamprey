import { useNavigate } from "@solidjs/router";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";
import { createResource, Match, Show, Switch } from "solid-js";
import { timeAgo } from "../Time.tsx";

// the context menu for threads
export function ThreadMenu(props: { thread_id: string }) {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();

	const self_id = () => api.users.cache.get("@self")!.id;
	const thread = api.threads.fetch(() => props.thread_id);
	const self_thread_member = api.thread_members.fetch(
		() => props.thread_id,
		self_id,
	);
	const copyId = () => navigator.clipboard.writeText(props.thread_id);
	const markRead = () => {
		const thread = api.threads.cache.get(props.thread_id)!;
		const version_id = thread.last_version_id;
		ctx.dispatch({
			do: "thread.mark_read",
			thread_id: props.thread_id,
			also_local: true,
			version_id,
		});
	};

	const removeThread = () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "are you sure you want to remove this tread?",
			cont(confirmed) {
				if (!confirmed) return;
				ctx.client.http.PUT("/api/v1/channel/{channel_id}/remove", {
					params: {
						path: { channel_id: props.thread_id },
					},
				});
			},
		});
	};

	const copyLink = () => {
		const url = `${ctx.client.opts.apiUrl}/thread/${props.thread_id}`;
		navigator.clipboard.writeText(url);
	};

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(thread())));

	const settings = (to: string) => () =>
		nav(`/thread/${props.thread_id}/settings${to}`);

	const archiveThread = () => {
		api.threads.archive(props.thread_id);
	};

	const unarchiveThread = () => {
		api.threads.unarchive(props.thread_id);
	};

	const toggleLock = () => {
		if (thread()?.locked) {
			api.threads.unlock(props.thread_id);
		} else {
			api.threads.lock(props.thread_id);
		}
	};

	const joinOrLeaveThread = () => {
		if (self_thread_member()?.membership === "Leave") {
			ctx.client.http.PUT("/api/v1/thread/{thread_id}/member/{user_id}", {
				params: {
					path: { thread_id: props.thread_id, user_id: "@self" },
				},
				body: {},
			});
		} else {
			ctx.client.http.DELETE("/api/v1/thread/{thread_id}/member/{user_id}", {
				params: {
					path: { thread_id: props.thread_id, user_id: "@self" },
				},
			});
		}
	};

	return (
		<Menu>
			<Item onClick={markRead}>mark as read</Item>
			<Item onClick={copyLink}>copy link</Item>
			<Show when={thread()}>
				{(t) => <ThreadNotificationMenu thread={t()} />}
			</Show>
			<Item onClick={joinOrLeaveThread}>
				{self_thread_member()?.membership === "Leave" ? "join" : "leave"}
			</Item>
			<Separator />
			<Submenu content={"edit"} onClick={settings("")}>
				<Item onClick={settings("")}>info</Item>
				<Item>permissions</Item>
				<Submenu content={"tags"}>
					<Item>foo</Item>
					<Item>bar</Item>
					<Item>baz</Item>
				</Submenu>
			</Submenu>
			<Switch>
				<Match when={!thread()?.archived_at}>
					<Item onClick={archiveThread}>archive</Item>
				</Match>
				<Match when={thread()?.archived_at}>
					<Item onClick={unarchiveThread}>unarchive</Item>
				</Match>
			</Switch>
			<Item onClick={toggleLock}>{thread()?.locked ? "unlock" : "lock"}</Item>
			<Item onClick={removeThread}>remove</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}

function ThreadNotificationMenu(props: { thread: import("sdk").Thread }) {
	const api = useApi();
	const threadConfig = () => props.thread.user_config;

	const setNotifs = (notifs: Partial<import("sdk").NotifsThread>) => {
		const current = threadConfig() ?? { notifs: {}, frontend: {} };
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
		api.threads.cache.set(props.thread.id, {
			...props.thread,
			user_config: newConfig,
		});
		api.client.http.PUT("/api/v1/config/thread/{thread_id}", {
			params: { path: { thread_id: props.thread.id } },
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
		const c = threadConfig();
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
					onClick={() =>
						setNotifs({ messages: undefined, mentions: undefined })}
				>
					<div>default</div>
					<div class="subtext">
						Uses the room's default notification setting.
					</div>
				</Item>
				<Item onClick={() => setNotifs({ messages: "Notify" })}>
					<div>everything</div>
					<div class="subtext">
						You will be notified of all new messages in this thread.
					</div>
				</Item>
				<Item onClick={() => setNotifs({ messages: "Watching" })}>
					<div>watching</div>
					<div class="subtext">
						Messages in this thread will show up in your inbox.
					</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Ignore", mentions: "Notify" })}
				>
					<div>mentions</div>
					<div class="subtext">You will only be notified on @mention</div>
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
					<Show when={threadConfig()?.notifs.mute?.expires_at}>
						<div class="subtext">
							unmutes {timeAgo(
								new Date(Date.parse(threadConfig()!.notifs.mute!.expires_at!)),
							)}
						</div>
					</Show>
				</Item>
			</Show>
		</>
	);
}
