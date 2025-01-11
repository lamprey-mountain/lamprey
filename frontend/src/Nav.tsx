import { For, Show } from "solid-js";
import { useCtx } from "./context.ts";

export const ChatNav = () => {
	const ctx = useCtx();
	const v = ctx.data.view;
	const roomId = () => (v.view === "room" || v.view === "room-settings" || v.view === "thread") ? v.room.id : null;
	const threadId = () => v.view === "thread" ? v.thread.id : null;
	const isRoomSelected = (id: string) => roomId() === id;
	return (
		<nav id="nav">
			<ul>
				<li>
					<button
						classList={{ "selected": v.view === "home" }}
						onClick={() => ctx.dispatch({ do: "setView", to: { view: "home" } })}
					>home</button>
				</li>
				<For each={Object.values(ctx.data.rooms)}>
					{(room) => (
						<li>
							<button
								classList={{ "selected": isRoomSelected(room.id) }}
								onClick={() => ctx.dispatch({ do: "setView", to: { view: "room", room }})}
								onContextMenu={(e) => { e.stopPropagation(); if (e.shiftKey) return; e.preventDefault(); ctx.dispatch({ do: "menu", menu: { type: "room", x: e.x, y: e.y, room }})}}
							>{room.name}</button>
							<Show when={isRoomSelected(room.id) || true}>
								<ul>
									<li>
										<button
											classList={{ "selected": v.view === "room" && v.room.id === room.id }}
											onClick={() => ctx.dispatch({ do: "setView", to: { view: "room", room }})}
										>home</button>
									</li>
									<For each={Object.values(ctx.data.threads).filter((i) => i.room_id === room.id)}>
										{(thread) => (
											<li>
												<button
													classList={{
														"selected": threadId() === thread.id,
														"closed": thread.is_closed,
														"unread": thread.last_read_id !== thread.last_version_id,
													}}
													onClick={() => ctx.dispatch({ do: "setView", to: { view: "thread", room, thread }})}
													onContextMenu={(e) => { e.stopPropagation(); if (e.shiftKey) return; e.preventDefault(); ctx.dispatch({ do: "menu", menu: { type: "thread", x: e.x, y: e.y, thread }})}}
												>{thread.name}</button>
											</li>
										)}
									</For>
								</ul>
							</Show>
						</li>
					)}
				</For>
			</ul>
		</nav>
	);
};
