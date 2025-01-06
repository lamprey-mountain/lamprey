import { useContext, For, Show } from "solid-js";
import { chatctx } from "./context.ts";

export const ChatNav = () => {
	const ctx = useContext(chatctx)!;
	const v = ctx.data.view;
	const roomId = () => (v.view === "room" || v.view === "room-settings" || v.view === "thread") ? v.room.id : null;
	const threadId = () => v.view === "thread" ? v.thread.id : null;
	const isRoomSelected = (id: string) => roomId() === id;
	return (
		<nav class="w-64 bg-bg1 text-fg2 overflow-y-auto">
			<ul class="p-1 flex flex-col">
				<li class="mt-1">
					<button
						class="px-1 py-0.25 w-full text-left hover:bg-bg4"
						classList={{ "bg-bg3": v.view === "home" }}
						onClick={() => ctx.dispatch({ do: "setView", to: { view: "home" } })}
					>home</button>
				</li>
				<For each={Object.values(ctx.data.rooms)}>
					{(room) => (
						<li class="mt-1">
							<button
								class="px-1 py-0.25 w-full text-left hover:bg-bg4"
								classList={{ "bg-bg3": isRoomSelected(room.id) }}
								onClick={() => ctx.dispatch({ do: "setView", to: { view: "room", room }})}
								onContextMenu={(e) => { e.stopPropagation(); if (e.shiftKey) return; e.preventDefault(); ctx.dispatch({ do: "menu", menu: { type: "room", x: e.x, y: e.y }})}}
							>{room.name}</button>
							<Show when={isRoomSelected(room.id)}>
								<ul class="ml-6">
									<li class="mt-1">
										<button
											class="px-1 py-0.25 w-full text-left hover:bg-bg4"
											classList={{ "bg-bg3": v.view === "room" }}
											onClick={() => ctx.dispatch({ do: "setView", to: { view: "room", room }})}
										>home</button>
									</li>
									<li class="mt-1">
										<button
											class="px-1 py-0.25 w-full text-left hover:bg-bg4"
											classList={{ "bg-bg3": v.view === "room-settings" }}
											onClick={() => ctx.dispatch({ do: "setView", to: { view: "room-settings", room }})}
										>settings</button>
									</li>
									<For each={Object.values(ctx.data.threads).filter((i) => i.room_id === roomId())}>
										{(thread) => (
											<li class="mt-1">
												<button
													class="px-1 py-0.25 w-full text-left hover:bg-bg4"
													classList={{
														"bg-bg3": threadId() === thread.id,
														"text-sep": thread.is_closed,
													}}
													onClick={() => ctx.dispatch({ do: "setView", to: { view: "thread", room, thread }})}
													onContextMenu={(e) => { e.stopPropagation(); if (e.shiftKey) return; e.preventDefault(); ctx.dispatch({ do: "menu", menu: { type: "thread", x: e.x, y: e.y }})}}
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
