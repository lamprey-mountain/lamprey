import { For, Switch, Match, Show, ParentProps, createEffect, createSignal } from "solid-js";
import { Portal } from "solid-js/web";
import { RoomMenu, ThreadMenu, MessageMenu } from "./Menu.tsx";
import { ChatNav } from "./Nav.tsx";
import { useCtx } from "./context.ts";
import { CLASS_BUTTON } from "./styles.ts";
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
		middleware: [shift({ mainAxis: true, crossAxis: true })],
		placement: "right-start",
	});

	async function createRoom() {
  	const name = await ctx.dispatch({ do: "modal.prompt", text: "name?" });
		ctx.client.http("POST", "/api/v1/rooms", {
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
					<div class="flex-1 bg-bg2 text-fg2 p-4 overflow-y-auto">
						<h2 class="text-xl">home</h2>
						<p>work in progress. expect bugs and missing polish.</p>
						<button class={CLASS_BUTTON} onClick={createRoom}>
							create room
						</button>
						<br />
						<button class={CLASS_BUTTON} onClick={useInvite}>use invite</button>
						<br />
						<a class={CLASS_BUTTON} href="/api/v1/auth/discord">
							discord login
						</a>
						<br />
						<a class={CLASS_BUTTON} href="/api/docs">
							api docs
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
								<div class="h-0.5"></div>
								<button onClick={() => ctx.dispatch({ do: "modal.close" })} class={CLASS_BUTTON}>okay!</button>
							</Modal>
						</Match>
						<Match when={modal.type === "confirm"}>
							<Modal>
								<p>{modal.text}</p>
								<div class="h-0.5"></div>
								<button onClick={() => {modal.cont(true); ctx.dispatch({ do: "modal.close" })}} class={CLASS_BUTTON}>okay!</button>&nbsp;
								<button onClick={() => {modal.cont(false); ctx.dispatch({ do: "modal.close" })}} class={CLASS_BUTTON}>nevermind...</button>
							</Modal>
						</Match>
						<Match when={modal.type === "prompt"}>
							<Modal>
								<p>{modal.text}</p>
								<div class="h-0.5"></div>
								<form onSubmit={e => {
									e.preventDefault();
									const form = e.target as HTMLFormElement;
									const input = form.elements.namedItem("text") as HTMLInputElement;
									modal.cont(input.value);
									ctx.dispatch({ do: "modal.close" });
								}}>
									<input class="bg-bg3 border-[1px] border-sep" type="text" name="text" autofocus />
									<div class="h-0.5"></div>
									<input type="submit" class={CLASS_BUTTON} value="done!" />&nbsp;
									<button
										onClick={() => {modal.cont(null); ctx.dispatch({ do: "modal.close" })}}
										class={CLASS_BUTTON}
									>nevermind...</button>
								</form>
							</Modal>
						</Match>
					</Switch>
				)}</For>
				<Show when={ctx.data.menu}>
					<div
						ref={setMenuRef}
						class="fixed overflow-y-auto max-h-[calc(100vh)] p-[8px] [scrollbar-width:none]"
						style={{
							top: `${menuFloating.y}px`,
							left: `${menuFloating.x}px`,
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
				</Show>
			</Portal>
    </>
  )
}

const Modal = (props: ParentProps) => {
	const ctx = useCtx()!;
	return (
		<div class="fixed top-0 left-0 w-full h-full grid place-items-center">
			<div class="absolute animate-popupbg w-full h-full" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="absolute">
				<div class="absolute animate-popupbase bg-bg2 border-[1px] border-sep w-full h-full"></div>
				<div class="animate-popupcont p-[8px] text-fg3 max-w-[500px] min-w-[100px] min-h-[50px]" role="dialog">
					{props.children}
				</div>
			</div>
		</div>
	);
}
