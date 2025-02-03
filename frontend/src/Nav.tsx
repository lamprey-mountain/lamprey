import { For, from, Show } from "solid-js";
import { useCtx } from "./context.ts";
import { A } from "@solidjs/router";
import { useApi } from "./api.tsx";

export const ChatNav = () => {
	const ctx = useCtx();
	const api = useApi();
	const state = from(ctx.client.state);

	const rooms = api.rooms.list();

	return (
		<nav id="nav">
			<ul>
				<li>
					<A href="/" end>home</A>
				</li>
				<For each={rooms()?.items}>
					{(room) => (
						<li>
							<A
								class="has-menu"
								data-room-id={room.id}
								href={`/room/${room.id}`}
							>
								{room.name}
							</A>
							<Show when={true}>
								<ul>
									<li>
										<A
											class="has-menu"
											href={`/room/${room.id}`}
											data-room-id={room.id}
										>
											home
										</A>
									</li>
									<For
										each={[
											...api.threads.cache.values().filter((i) =>
												i.room_id === room.id && i.state !== "Deleted"
											),
										]}
									>
										{(thread) => (
											<li>
												<A
													href={`/thread/${thread.id}`}
													class="has-menu"
													classList={{
														"closed": thread.state === "Archived",
														"unread":
															thread.last_read_id !== thread.last_version_id,
													}}
													data-thread-id={thread.id}
												>
													{thread.name}
												</A>
											</li>
										)}
									</For>
								</ul>
							</Show>
						</li>
					)}
				</For>
			</ul>
			<div style="flex:1"></div>
			<div style="margin: 8px">
				state: {state()}
			</div>
		</nav>
	);
};
