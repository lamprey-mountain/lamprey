import {
	createEffect,
	createSignal,
	For,
	Match,
	ParentProps,
	Show,
	Switch,
} from "solid-js";
import { Portal } from "solid-js/web";
import { MessageMenu, RoomMenu, ThreadMenu } from "./Menu.tsx";
import { ChatNav } from "./Nav.tsx";
import { useCtx } from "./context.ts";
import { ChatMain } from "./Chat.tsx";
import { RoomHome } from "./Room.tsx";
import { RoomSettings } from "./Settings.tsx";
import { ClientRectObject, ReferenceElement, shift } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import { A, Route, Router } from "@solidjs/router";

export const Main = () => {
	const ctx = useCtx();

	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const menuFloating = useFloating(menuParentRef, menuRef, {
		middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
		placement: "right-start",
	});

	createEffect(() => {
		// force solid to track these properties
		ctx?.data.menu?.x;
		ctx?.data.menu?.y;

		setMenuParentRef({
			getBoundingClientRect(): ClientRectObject {
				return {
					x: ctx.data.menu?.x,
					y: ctx.data.menu?.y,
					left: ctx.data.menu?.x,
					top: ctx.data.menu?.y,
					right: ctx.data.menu?.x,
					bottom: ctx.data.menu?.y,
					width: 0,
					height: 0,
				};
			},
		});
	});

	// function getComponent() {
	// 	switch (ctx.data.view.view) {
	// 		case "home": {
	// 		}
	// 		case "room": {
	// 			return <RoomHome room={ctx.data.view.room} />;
	// 		}
	// 		case "room-settings": {
	// 			const room = ctx.data.view.room;
	// 			return <RoomSettings room={room} />;
	// 		}
	// 		case "thread": {
	// 			const room = ctx.data.view.room;
	// 			const thread = ctx.data.view.thread;
	// 			return <ChatMain room={room} thread={thread} />;
	// 		}
	// 	}
	// }

	// HACK: wrap in Show since ctx might be null during hmr
	// this router is extremely messy - i'm not sure if i'm going to keep it or if i'll roll my own
	return (
		<>
			<Show when={useCtx()}>
				<Router>
					<Route
						path="/"
						component={() => (
							<>
								<ChatNav />
								<Home />
							</>
						)}
					/>
					<Route
						path="/room/:room_id"
						component={(p) => {
							const room = () => ctx.data.rooms[p.params.room_id];
							return (
								<>
									<ChatNav />
									<Show when={room()}>
										<RoomHome room={room()} />
									</Show>
								</>
							);
						}}
					/>
					<Route
						path="/room/:room_id/settings/:page?"
						component={(p) => {
							const room = () => ctx.data.rooms[p.params.room_id];
							return (
								<>
									<ChatNav />
									<Show when={room()}>
										<RoomSettings room={room()} page={p.params.page} />
									</Show>
								</>
							);
						}}
					/>
					<Route
						path="/thread/:thread_id"
						component={(p) => {
							const thread = () => ctx.data.threads[p.params.thread_id];
							const room = () => ctx.data.rooms[thread()?.room_id];

							createEffect(() => {
								if (thread()?.room_id && !ctx.data.rooms[thread()?.room_id]) {
									ctx.dispatch({ do: "fetch.room", room_id: p.params.room_id });
								}
							});

							createEffect(() => {
								if (!ctx.data.threads[p.params.thread_id]) {
									ctx.dispatch({
										do: "fetch.thread",
										thread_id: p.params.thread_id,
									});
								}
							});

							return (
								<>
									<ChatNav />
									<Show when={room() && thread()}>
										<ChatMain room={room()} thread={thread()} />
									</Show>
								</>
							);
						}}
					/>
					<Route
						path="*404"
						component={() => (
							<div style="padding:8px">
								not found
							</div>
						)}
					/>
				</Router>
				<Portal mount={document.getElementById("overlay")!}>
					<For each={ctx.data.modals}>
						{(modal) => (
							<Switch>
								<Match when={modal.type === "alert"}>
									<Modal>
										<p>{modal.text}</p>
										<div style="height: 8px"></div>
										<button onClick={() => ctx.dispatch({ do: "modal.close" })}>
											okay!
										</button>
									</Modal>
								</Match>
								<Match when={modal.type === "confirm"}>
									<Modal>
										<p>{modal.text}</p>
										<div style="height: 8px"></div>
										<button
											onClick={() => {
												modal.cont(true);
												ctx.dispatch({ do: "modal.close" });
											}}
										>
											okay!
										</button>&nbsp;
										<button
											onClick={() => {
												modal.cont(false);
												ctx.dispatch({ do: "modal.close" });
											}}
										>
											nevermind...
										</button>
									</Modal>
								</Match>
								<Match when={modal.type === "prompt"}>
									<Modal>
										<p>{modal.text}</p>
										<div style="height: 8px"></div>
										<form
											onSubmit={(e) => {
												e.preventDefault();
												const form = e.target as HTMLFormElement;
												const input = form.elements.namedItem(
													"text",
												) as HTMLInputElement;
												modal.cont(input.value);
												ctx.dispatch({ do: "modal.close" });
											}}
										>
											<input type="text" name="text" autofocus />
											<div style="height: 8px"></div>
											<input type="submit" value="done!"></input>{" "}
											<button
												onClick={() => {
													modal.cont(null);
													ctx.dispatch({ do: "modal.close" });
												}}
											>
												nevermind...
											</button>
										</form>
									</Modal>
								</Match>
							</Switch>
						)}
					</For>
					<Show when={ctx.data.menu}>
						<div class="contextmenu">
							<div
								ref={setMenuRef}
								class="inner"
								style={{
									translate: `${menuFloating.x}px ${menuFloating.y}px`,
								}}
							>
								<Switch>
									<Match
										when={ctx.data.menu.type === "room"}
										children={<RoomMenu room={ctx.data.menu.room} />}
									/>
									<Match
										when={ctx.data.menu.type === "thread"}
										children={<ThreadMenu thread={ctx.data.menu.thread} />}
									/>
									<Match
										when={ctx.data.menu.type === "message"}
										children={<MessageMenu message={ctx.data.menu.message} />}
									/>
								</Switch>
							</div>
						</div>
					</Show>
				</Portal>
			</Show>
		</>
	);
};

const Modal = (props: ParentProps) => {
	const ctx = useCtx()!;
	return (
		<div class="modal">
			<div class="bg" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog">
					{props.children}
				</div>
			</div>
		</div>
	);
};

const Home = () => {
	const ctx = useCtx();

	function createRoom() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.POST("/api/v1/room", {
					body: { name },
				});
			},
		});
	}

	function useInvite() {
		ctx.dispatch({
			do: "modal.prompt",
			text: "invite code?",
			cont(code) {
				// TODO: fix
				// ctx.client.http.POST("/api/v1/invite")
				// ctx.client.http("POST", `/api/v1/invites/${code}`, {});
				queueMicrotask(() => {
					ctx.dispatch({ do: "modal.alert", text: "todo!" });
				});
			}
		});
	}

	return (
		<div class="home">
			<h2>home</h2>
			<p>work in progress. expect bugs and missing polish.</p>
			<button onClick={createRoom}>
				create room
			</button>
			<br />
			<button onClick={useInvite}>use invite</button>
			<br />
			<A target="_self" href="/api/v1/auth/discord">discord login</A>
			<br />
			<A target="_self" href="/api/docs">api docs</A>
			<br />
		</div>
	);
};
