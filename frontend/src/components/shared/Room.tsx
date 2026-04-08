import { A, useNavigate } from "@solidjs/router";
import { type Channel, getTimestampFromUUID } from "sdk";
import { createMemo, For, Show } from "solid-js";
import { useChannels } from "@/api";
import { useCtx } from "@/app/context";
import { Time } from "@/atoms/Time";
import { MemberList } from "@/components/shared/MemberList";
import { useCurrentUser } from "@/contexts/currentUser";
import { useModals } from "@/contexts/modal";
import { usePermissions } from "@/hooks/usePermissions";
import { md } from "@/lib/markdown";
import type { RoomT } from "@/types";
import { ChannelIcon } from "./User.tsx";

export const RoomMembers = (props: { room: RoomT }) => {
	return <MemberList type="room" id={props.room.id} roomId={props.room.id} />;
};

// TODO: show online/member count
// TODO: show feed with important messages/highlights
// TODO: show active channels
// TODO: show invite button
export const RoomHome = (props: { room: RoomT }) => {
	const ctx = useCtx();
	const nav = useNavigate();
	const [, modalCtl] = useModals();
	const room_id = () => props.room.id;

	const channels2 = useChannels();
	const threadsResource = createMemo(() =>
		[...channels2.cache.values()].filter((c) => c.room_id === room_id()),
	);

	const categorizedChannels = createMemo(() => {
		const items = threadsResource();
		if (!items) return [];

		const allChannels: Channel[] = [...items];

		const threads = allChannels.filter(
			(c) => c.type === "ThreadPublic" || c.type === "ThreadPrivate",
		);
		const channels = allChannels.filter(
			(c) => c.type !== "ThreadPublic" && c.type !== "ThreadPrivate",
		);

		channels.sort((a, b) => {
			if (a.position == null && b.position == null) {
				return a.id < b.id ? 1 : -1;
			}
			if (a.position == null) return 1;
			if (b.position == null) return -1;
			return a.position - b.position;
		});

		const channelMap = new Map<string, Channel & { threads: Channel[] }>();
		for (const c of channels) {
			channelMap.set(c.id, { ...c, threads: [] });
		}

		for (const thread of threads) {
			if (thread.parent_id) {
				const parent = channelMap.get(thread.parent_id);
				if (parent) {
					parent.threads.push(thread);
				}
			}
		}

		for (const c of channelMap.values()) {
			if (c.threads.length > 1) {
				c.threads.sort((a, b) => a.id.localeCompare(b.id));
			}
		}

		const categories = new Map<
			string | null,
			Array<Channel & { threads: Channel[] }>
		>();
		for (const c of channelMap.values()) {
			if (c.type === "Category") {
				const cat = categories.get(c.id) ?? [];
				categories.set(c.id, cat);
			} else {
				const parentId = c.parent_id ?? null;
				const children = categories.get(parentId) ?? [];
				children.push(c);
				categories.set(parentId, children);
			}
		}
		const list = [...categories.entries()]
			.map(([cid, cs]) => ({
				category: cid ? (channels2.cache.get(cid) ?? null) : null,
				channels: cs,
			}))
			.sort((a, b) => {
				// null category comes first
				if (!a.category) return -1;
				if (!b.category) return 1;

				// categories with positions come first
				if (a.category.position == null && b.category.position == null) {
					// newer categories first
					return a.category.id < b.category.id ? 1 : -1;
				}
				if (a.category.position == null) return 1;
				if (b.category.position == null) return -1;

				// order by position
				const p = a.category.position - b.category.position;
				if (p === 0) {
					// newer categories first
					return a.category.id < b.category.id ? 1 : -1;
				}

				return p;
			});
		return list;
	});

	function createThread(room_id: string) {
		modalCtl.open({
			type: "channel_create",
			room_id: room_id,
			cont: (data) => {
				if (!data) return;
				ctx.client.http.POST("/api/v1/room/{room_id}/channel", {
					params: {
						path: { room_id },
					},
					body: {
						name: data.name,
						type: data.type,
					},
				});
			},
		});
	}

	function leaveRoom(_room_id: string) {
		modalCtl.confirm("are you sure you want to leave?", (confirmed) => {
			if (!confirmed) return;
			const uid = user()?.id;
			if (!uid) return;
			ctx.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
				params: {
					path: {
						room_id: props.room.id,
						user_id: uid,
					},
					query: { soft: false },
				},
			});
		});
	}

	const user = useCurrentUser();
	const user_id = () => user()?.id;
	const _perms = usePermissions(user_id, room_id, () => undefined);

	return (
		<div class="room-home">
			<div style="display:flex">
				<div style="flex:1">
					<h2>{props.room.name}</h2>
					<p
						class="markdown"
						innerHTML={md(props.room.description ?? "") as string}
					></p>
				</div>
				<div style="display:flex;flex-direction:column;gap:4px">
					<button
						type="button"
						class="button"
						onClick={() => leaveRoom(room_id())}
					>
						leave room
					</button>
					<A style="padding: 0 4px" href={`/room/${props.room.id}/settings`}>
						settings
					</A>
				</div>
			</div>
			<div style="display:flex; align-items:center">
				<h3 style="font-size:1rem; margin-top:8px;flex:1">
					{threadsResource().length} channels
				</h3>
				{/*
				<div class="thread-filter">
					<button type="button" class="button"
						classList={{ selected: threadFilter() === "active" }}
						onClick={[setThreadFilter, "active"]}
					>
						active
					</button>
					<button type="button" class="button"
						classList={{ selected: threadFilter() === "archived" }}
						onClick={[setThreadFilter, "archived"]}
					>
						archived
					</button>
					<Show when={perms.has("ThreadManage")}>
						<button type="button" class="button"
							classList={{ selected: threadFilter() === "removed" }}
							onClick={[setThreadFilter, "removed"]}
						>
							removed
						</button>
					</Show>
				</div>
				*/}
				<button
					type="button"
					class="button primary"
					style="margin-left: 8px;border-radius:4px"
					onClick={() => createThread(room_id())}
				>
					create channel
				</button>
			</div>
			<For each={categorizedChannels()}>
				{({ category, channels }) => (
					<>
						<h3 class="dim" style="margin-top:12px;margin-bottom:4px">
							{category?.name}
						</h3>
						<ul class="thread-group">
							<For each={channels}>
								{(thread) => (
									<li>
										<article
											class="thread menu-thread thread-card"
											data-thread-id={thread.id}
										>
											<button
												type="button"
												class="top"
												onClick={() => nav(`/thread/${thread.id}`)}
												onKeyDown={(e) =>
													e.key === "Enter" && nav(`/thread/${thread.id}`)
												}
											>
												<ChannelIcon channel={thread} />
												<div class="spacer">{thread.name}</div>
												<div class="time">
													Created{" "}
													<Time date={getTimestampFromUUID(thread.id)} />
												</div>
											</button>
											<button
												type="button"
												class="bottom"
												onClick={() => nav(`/thread/${thread.id}`)}
												onKeyDown={(e) =>
													e.key === "Enter" && nav(`/thread/${thread.id}`)
												}
											>
												<div class="dim">
													{thread.message_count} message(s) &bull; last msg{" "}
													<Time
														date={getTimestampFromUUID(
															thread.last_version_id ?? thread.id,
														)}
													/>
												</div>
												<Show when={thread.description}>
													<div
														class="description markdown"
														innerHTML={md(thread.description ?? "") as string}
													></div>
												</Show>
											</button>
										</article>
									</li>
								)}
							</For>
						</ul>
					</>
				)}
			</For>
		</div>
	);
};
