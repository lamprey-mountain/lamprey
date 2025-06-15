import { For, from, Show } from "solid-js";
import { useCtx } from "./context.ts";
import { A, useMatch } from "@solidjs/router";
import { useApi } from "./api.tsx";
import type { Room, Thread } from "sdk";
import { flags } from "./flags.ts";

export const ChatNav = (props: { room_id?: string }) => {
	const ctx = useCtx();
	const api = useApi();

	return (
		<nav id="nav">
			<Show when={flags.has("nav_header")}>
				<header style="background: #eef1;padding:8px">header</header>
			</Show>
			<ul>
				<li>
					<Show when={!!props.room_id}>
						<A href={`/room/${props.room_id}`} class="menu-thread">
							home
						</A>
					</Show>
				</li>
				<For
					each={[...api.threads.cache.values()].filter((i) =>
						i.room_id === props.room_id && !i.deleted_at
					)}
				>
					{(thread) => <ItemThread thread={thread} />}
				</For>
			</ul>
			<div style="margin: 8px">
			</div>
		</nav>
	);
};

const ChatNav_ = () => {
	const ctx = useCtx();
	const api = useApi();

	const rooms = api.rooms.list();

	return (
		<nav id="nav">
			<ul>
				<li>
					<A href="/" end>home</A>
				</li>
				<For each={rooms()?.items}>
					{(room) => <ItemRoom room={room} />}
				</For>
			</ul>
			<div style="margin: 8px">
			</div>
		</nav>
	);
};

const ItemRoom = (props: { room: Room }) => {
	const api = useApi();

	// TODO: send self room member in api? this works for now though
	const shouldShow = () => {
		const user_id = api.users.cache.get("@self")?.id;
		const c = api.room_members.cache.get(props.room.id);
		const m = c?.get(user_id!);
		if (m && m.membership !== "Join") return false;
		return true;
	};

	return (
		<Show when={shouldShow()}>
			<li>
				<A
					class="menu-room"
					data-room-id={props.room.id}
					href={`/room/${props.room.id}`}
				>
					{props.room.name}
				</A>
				<Show when={true}>
					<ul>
						<li>
							<A
								class="menu-room"
								href={`/room/${props.room.id}`}
								data-room-id={props.room.id}
							>
								home
							</A>
						</li>
					</ul>
				</Show>
			</li>
		</Show>
	);
};

const ItemThread = (props: { thread: Thread }) => {
	return (
		<li>
			<A
				href={`/thread/${props.thread.id}`}
				class="menu-thread"
				classList={{
					"closed": !!props.thread.archived_at,
					"unread": props.thread.is_unread,
				}}
				data-thread-id={props.thread.id}
			>
				{props.thread.name}
			</A>
		</li>
	);
};
