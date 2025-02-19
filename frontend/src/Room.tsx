import { For, Show } from "solid-js";
import { RoomT } from "./types.ts";
import { useCtx } from "./context.ts";
import { getTimestampFromUUID } from "sdk";
import { A, useNavigate } from "@solidjs/router";
import { useApi } from "./api.tsx";

export const RoomMembers = (props: { room: RoomT }) => {
	const api = useApi();
	const room_id = () => props.room.id;

	const members = api.room_members.list(room_id);

	return (
		<ul class="room-members">
			<For each={members()?.items}>
				{(i) => {
					const user = api.users.fetch(() => i.user_id);
					const room_member = api.room_members.fetch(
						room_id,
						() => i.user_id,
					);

					function name() {
						let name: string | undefined | null = null;
						const rm = room_member?.();
						if (rm?.membership === "Join") name ??= rm.override_name;

						name ??= user()?.name;
						return name;
					}

					return (
						<li data-user-id={i.user_id}>{name()}</li>
					);
				}}
			</For>
		</ul>
	);
};

export const RoomHome = (props: { room: RoomT }) => {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const room_id = () => props.room.id;

	const threads = api.threads.list(room_id);

	function createThread(room_id: string) {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.POST("/api/v1/room/{room_id}/thread", {
					params: {
						path: { room_id },
					},
					body: { name },
				});
			},
		});
	}

	function leaveRoom(_room_id: string) {
		ctx.dispatch({
			do: "modal.confirm",
			text: "are you sure you want to leave?",
			cont(confirmed) {
				if (!confirmed) return;
				ctx.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
					params: {
						path: {
							room_id: props.room.id,
							user_id: api.users.cache.get("@self")!.id,
						},
					},
				});
			},
		});
	}

	// <div class="date"><Time ts={props.thread.baseEvent.originTs} /></div>
	return (
		<div class="room-home">
			<h2>{props.room.name}</h2>
			<p>{props.room.description}</p>
			<button onClick={() => createThread(room_id())}>create thread</button>
			<br />
			<button onClick={() => leaveRoom(room_id())}>leave room</button>
			<br />
			<A href={`/room/${props.room.id}/settings`}>settings</A>
			<br />
			<ul>
				<For
					each={[
						...threads()?.items.filter((i) =>
							i.room_id === props.room.id && i.state !== "Deleted"
						) ??
						[],
					]}
				>
					{(thread) => (
						<li>
							<article class="thread">
								<header onClick={() => nav(`/thread/${thread.id}`)}>
									<div class="top">
										<div class="icon"></div>
										<div class="spacer">{thread.name}</div>
										<div class="time">
											Created at{" "}
											{getTimestampFromUUID(thread.id).toDateString()}
										</div>
									</div>
									<div
										class="bottom"
										onClick={() => nav(`/thread/${thread.id}`)}
									>
										{thread.message_count} messages &bull; last msg{" "}
										{getTimestampFromUUID(thread.last_version_id ?? thread.id)
											.toDateString()}
										<Show when={thread.description}>
											<br />
											{thread.description}
										</Show>
									</div>
								</header>
								<Show when={true}>
									<div class="preview">
										<For each={[]}>
											{(_msg) => "todo: show message here?"}
										</For>
										<details>
											<summary>json data</summary>
											<pre>
												{JSON.stringify(thread, null, 4)}
											</pre>
										</details>
									</div>
								</Show>
								<Show when={false}>
									<footer>message.remaining</footer>
								</Show>
							</article>
						</li>
					)}
				</For>
			</ul>
		</div>
	);
};
