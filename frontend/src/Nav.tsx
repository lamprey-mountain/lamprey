import { For, from, Show } from "solid-js";
import { useCtx } from "./context.ts";
import { A } from "@solidjs/router";
import { useApi } from "./api.tsx";

export const ChatNav = () => {
	const ctx = useCtx();
	const api = useApi();
	const state = from(ctx.client.state);

	const rooms = api.rooms.list();

	// should i only show threads from the currently active rooms? or show less threads until the room is selected?
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
								href={`/room/${room.id}`}
								onContextMenu={(e) => {
									e.stopPropagation();
									if (e.shiftKey) return;
									e.preventDefault();
									ctx.dispatch({
										do: "menu",
										menu: { type: "room", x: e.x, y: e.y, room },
									});
								}}
							>
								{room.name}
							</A>
							<Show when={true}>
								<ul>
									<li>
										<A
											href={`/room/${room.id}`}
											onContextMenu={(e) => {
												e.stopPropagation();
												if (e.shiftKey) return;
												e.preventDefault();
												ctx.dispatch({
													do: "menu",
													menu: { type: "room", x: e.x, y: e.y, room },
												});
											}}
										>
											home
										</A>
									</li>
									<For
										each={[
											...api.threads.cache.values().filter((i) =>
												i.room_id === room.id
											),
										]}
									>
										{(thread) => (
											<li>
												<A
													href={`/thread/${thread.id}`}
													classList={{
														"closed": thread.state === "Archived",
														"unread":
															thread.last_read_id !== thread.last_version_id,
													}}
													onContextMenu={(e) => {
														e.stopPropagation();
														if (e.shiftKey) return;
														e.preventDefault();
														ctx.dispatch({
															do: "menu",
															menu: { type: "thread", x: e.x, y: e.y, thread },
														});
													}}
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
