import { For, Switch, Match, Show, ParentProps, createEffect, createSignal } from "solid-js";
import { Portal } from "solid-js/web";
import { RoomMenu, ThreadMenu, MessageMenu } from "./Menu.tsx";
import { ChatNav } from "./Nav.tsx";
import { useCtx } from "./context.ts";
import { ChatMain } from "./Chat.tsx";
import { RoomHome } from "./Room.tsx";
import { RoomSettings } from "./Settings.tsx";
import { ClientRectObject, ReferenceElement, shift } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";

export const Main = () => {
  const ctx = useCtx();

	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const menuFloating = useFloating(menuParentRef, menuRef, {
		middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
		placement: "right-start",
	});

	async function createRoom() {
  	const name = await ctx.dispatch({ do: "modal.prompt", text: "name?" });
		ctx.client.http("POST", "/api/v1/room", {
			name,
		});
	}

	async function useInvite() {
  	const code = await ctx.dispatch({ do: "modal.prompt", text: "invite code?" });
		ctx.client.http("POST", `/api/v1/invites/${code}`, {});
	}
	
	createEffect(() => {
		// force solid to track these properties
		ctx.data.menu?.x;
		ctx.data.menu?.y;

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

	function getComponent() {
		switch (ctx.data.view.view) {
			case "home": {
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
						<a href="/api/v1/auth/discord">
							<button>discord login</button>
						</a>
						<br />
						<a href="/api/docs">
							<button>api docs</button>
						</a>
						<br />
					</div>
				);
			}
			case "room": {
				return <RoomHome room={ctx.data.view.room} />;
			}
			case "room-settings": {
				const room = ctx.data.view.room;
				return <RoomSettings room={room} />;
			}
			case "thread": {
				const room = ctx.data.view.room;
				const thread = ctx.data.view.thread;
				return <ChatMain room={room} thread={thread} />;
			}
		}
	}  
  
  return (
    <>
			<ChatNav />
			{getComponent()}
			<Portal>
				<For each={ctx.data.modals}>{(modal) => (
					<Switch>
						<Match when={modal.type === "alert"}>
							<Modal>
								<p>{modal.text}</p>
								<div style="height: 8px"></div>
								<button onClick={() => ctx.dispatch({ do: "modal.close" })}>okay!</button>
							</Modal>
						</Match>
						<Match when={modal.type === "confirm"}>
							<Modal>
								<p>{modal.text}</p>
								<div style="height: 8px"></div>
								<button onClick={() => {modal.cont(true); ctx.dispatch({ do: "modal.close" })}}>okay!</button>&nbsp;
								<button onClick={() => {modal.cont(false); ctx.dispatch({ do: "modal.close" })}}>nevermind...</button>
							</Modal>
						</Match>
						<Match when={modal.type === "prompt"}>
							<Modal>
								<p>{modal.text}</p>
								<div style="height: 8px"></div>
								<form onSubmit={e => {
									e.preventDefault();
									const form = e.target as HTMLFormElement;
									const input = form.elements.namedItem("text") as HTMLInputElement;
									modal.cont(input.value);
									ctx.dispatch({ do: "modal.close" });
								}}>
									<input type="text" name="text" autofocus />
									<div style="height: 8px"></div>
									<input type="submit">done!</input>{" "}
									<button
										onClick={() => {modal.cont(null); ctx.dispatch({ do: "modal.close" })}}
									>nevermind...</button>
								</form>
							</Modal>
						</Match>
					</Switch>
				)}</For>
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
    </>
  )
}

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
}
