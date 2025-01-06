import {
	Accessor,
	batch,
	Component,
	JSX,
	Match,
	ParentProps,
	Show,
	Switch,
	untrack,
	useContext,
} from "solid-js";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import { Dynamic, Portal } from "solid-js/web";
import { ChatMain } from "./Chat.tsx";
import { Client } from "sdk";
import { Action, chatctx, Data, View } from "./context.ts";
import { RoomSettings } from "./Settings.tsx";
import { createStore, produce, reconcile } from "solid-js/store";
import { InviteT, MemberT, MessageT, Pagination, RoleT } from "./types.ts";
import { Menu, MessageMenu, RoomMenu, ThreadMenu } from "./Menu.tsx";
import { useFloating } from "solid-floating-ui";
import { ClientRectObject, ReferenceElement, shift } from "@floating-ui/dom";
import { ChatNav } from "./Nav.tsx";
import { RoomHome } from "./Room.tsx";
import { Route, Router } from "@solidjs/router";

const BASE_URL = "https://chat.celery.eu.org";
// const TOKEN = "0a11b93f-ff19-4c56-9bd2-d25bede776de";
const TOKEN = localStorage.getItem("token")!;

const CLASS_BUTTON = "px-1 bg-bg3 hover:bg-bg4 my-0.5";
const SLICE_LEN = 100;
const PAGINATE_LEN = 30;

const App: Component = () => {
	const [hash, setHash] = createSignal(location.hash.slice(1));
	const [title, setTitle] = createSignal(document.title);
	const [isReady, setIsReady] = createSignal(false);

	const [data, updateData] = createStore<Data>({
		rooms: {},
		room_members: {},
		room_roles: {},
		threads: {},
		messages: {},
		timelines: {},
		slices: {},
		invites: {},
		users: {},
		modals: [],
		user: null,
		menu: null,
		view: { view: "home" },
	});

	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const menuFloating = useFloating(menuParentRef, menuRef, {
		middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
		placement: "right-start",
	});

	createEffect(() => {
		// force solid to track these properties
		data.menu?.x;
		data.menu?.y;

		setMenuParentRef({
			getBoundingClientRect(): ClientRectObject {
				return {
					x: data.menu?.x,
					y: data.menu?.y,
					left: data.menu?.x,
					top: data.menu?.y,
					right: data.menu?.x,
					bottom: data.menu?.y,
					width: 0,
					height: 0,
				};
			},
		});
	});

	const ws = new WebSocket(`${BASE_URL}/api/v1/sync`);
	ws.onopen = () => {
		console.log("opened");
		ws.send(JSON.stringify({ type: "hello", token: TOKEN }));
	};

	ws.onclose = () => {
		console.log("closed");
	};

	ws.onmessage = (ev) => {
		const msg = JSON.parse(ev.data);
		console.log("recv", msg);

		console.log("update");
		if (msg.type === "ping") {
			ws.send(JSON.stringify({ type: "pong" }));
		} else if (msg.type === "ready") {
			updateData("user", msg.user);
		} else if (msg.type === "upsert.room") {
			updateData("rooms", msg.room.id, msg.room);
		} else if (msg.type === "upsert.thread") {
			updateData("threads", msg.thread.id, msg.thread);
		} else if (msg.type === "upsert.message") {
			updateData("messages", msg.message.id, msg.message);
			const { thread_id } = msg.message;
			if (!data.timelines[thread_id]) {
				updateData("timelines", thread_id, [{ type: "hole" }, {
					type: "remote",
					message: msg.message as MessageT,
				}]);
				updateData("slices", thread_id, { start: 0, end: 2 });
			} else {
				updateData(
					"timelines",
					msg.message.thread_id,
					(i) => [...i, { type: "remote" as const, message: msg.message }],
				);
				if (
					data.slices[thread_id].end === data.timelines[thread_id].length - 1
				) {
					const newEnd = data.timelines[thread_id].length;
					const newStart = Math.max(newEnd - PAGINATE_LEN, 0);
					updateData("slices", thread_id, { start: newStart, end: newEnd });
				}
			}
		} else if (msg.type === "upsert.role") {
			const role: RoleT = msg.role;
			const { room_id } = role;
			if (!data.room_roles[room_id]) updateData("room_roles", room_id, {});
			updateData("room_roles", room_id, role.id, role);
		} else if (msg.type === "upsert.member") {
			const member: MemberT = msg.member;
			const { room_id } = member;
			if (!data.room_members[room_id]) updateData("room_members", room_id, {});
			updateData("users", member.user.id, member.user);
			updateData("room_members", room_id, member.user.id, member);
		} else if (msg.type === "upsert.invite") {
			const invite: InviteT = msg.invite;
			updateData("invites", invite.code, invite);
		} else if (msg.type === "delete.member") {
			const { user_id, room_id } = msg
			updateData("room_members", room_id, produce((obj) => {
				if (!obj) return;
				delete obj[user_id];
			}));
			if (user_id === data.user?.id) {
				updateData("rooms", produce((obj) => {
					delete obj[room_id];
				}));
			}
		} else if (msg.type === "delete.invite") {
			const { code } = msg
			updateData("invites", produce((obj) => {
				delete obj[code];
			}));
		} else {
			console.warn("unknown message", msg);
			return;
		}
	};

	const client = new Client(TOKEN, BASE_URL);

	(async () => {
		const data = await client.http("GET", `/api/v1/rooms?dir=f&limit=100`);
		for (const room of data.items) {
			updateData("rooms", room.id, room);
		}
	})();

	const handleClick = () => {
		dispatch({ do: "menu", menu: null });
	};

	const handleHashChange = () => setHash(location.hash.slice(1));
	globalThis.addEventListener("hashchange", handleHashChange);
	globalThis.addEventListener("click", handleClick);
	onCleanup(() => {
		globalThis.removeEventListener("hashchange", handleHashChange);
		globalThis.removeEventListener("click", handleClick);
	});
	createEffect(() => document.title = title());
	createEffect(() => location.hash = hash());
	// createEffect(() => setTitle(parts.get(hash())?.title ?? "unknown"));

	async function createRoom() {
  	const name = await dispatch({ do: "modal.prompt", text: "name?" });
		client.http("POST", "/api/v1/rooms", {
			name,
		});
	}

	async function useInvite() {
  	const code = await dispatch({ do: "modal.prompt", text: "invite code?" });
		client.http("POST", `/api/v1/invites/${code}`, {});
	}

	function getComponent() {
		switch (data.view.view) {
			case "home": {
				return (
					<div class="flex-1 bg-bg2 text-fg2 p-4">
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
					</div>
				);
			}
			case "room": {
				return <RoomHome room={data.view.room} />;
			}
			case "room-settings": {
				const room = data.view.room;
				return <RoomSettings room={room} />;
			}
			case "thread": {
				const room = data.view.room;
				const thread = data.view.thread;
				return <ChatMain room={room} thread={thread} />;
			}
		}
	}

	createEffect(() => {
		console.log(data.rooms[hash()]);
	})
	
	globalThis.dispatch = dispatch;

	async function dispatch(action: Action) {
		switch (action.do) {
			case "setView": {
				updateData("view", action.to);
				if ("room" in action.to) {
					const room_id = action.to.room.id;
					const roomThreadCount = [...Object.values(data.threads)].filter((i) =>
						i.room_id === room_id
					).length;
					if (roomThreadCount === 0) {
						const data = await client.http(
							"GET",
							`/api/v1/rooms/${room_id}/threads?dir=f`,
						);
						for (const item of data.items) {
							updateData("threads", item.id, item);
						}
					}
				}
				return;
			}
			case "paginate": {
				const { dir, thread_id } = action;
				const slice = data.slices[thread_id];
				console.log("paginate", { dir, thread_id, slice });
				if (!slice) {
					const batch = await client.http(
						"GET",
						`/api/v1/threads/${thread_id}/messages?dir=b&from=ffffffff-ffff-ffff-ffff-ffffffffffff&limit=100`,
					);
					const tl = batch.items.map((i: MessageT) => ({
						type: "remote" as const,
						message: i,
					}));
					if (batch.has_more) tl.unshift({ type: "hole" });
					updateData("timelines", thread_id, tl);
					updateData("slices", thread_id, { start: 0, end: tl.length + 1 });
					return;
				}

				const tl = data.timelines[thread_id];
				console.log(tl);
				if (tl.length < 2) return; // needs startitem and nextitem
				if (dir === "b") {
					const startItem = tl[slice.start];
					const nextItem = tl[slice.start + 1];
					if (startItem?.type === "hole") {
						const from = (nextItem as any)?.message.id ??
							"ffffffff-ffff-ffff-ffff-ffffffffffff";
						const batch = await client.http(
							"GET",
							`/api/v1/threads/${thread_id}/messages?dir=b&limit=100&from=${from}`,
						);
						updateData("timelines", thread_id, (i) => [
							...batch.has_more ? [{ type: "hole" }] : [],
							...batch.items.map((j: MessageT) => ({
								type: "remote",
								message: j,
							})),
							...i.slice(slice.start + 1),
						]);
					}

					const newTl = data.timelines[thread_id];
					const newOff = newTl.indexOf(nextItem) - slice.start;
					const newStart = Math.max(slice.start + newOff - PAGINATE_LEN, 0);
					const newEnd = Math.min(newStart + SLICE_LEN, newTl.length);
					console.log({ start: newStart, end: newEnd });
					updateData("slices", thread_id, { start: newStart, end: newEnd });
				} else {
					const startItem = tl[slice.end - 1];
					const nextItem = tl[slice.end - 2];
					if (startItem.type === "hole") {
						const from = (nextItem as any)?.message.id ??
							"00000000-0000-0000-0000-000000000000";
						const batch = await client.http(
							"GET",
							`/api/v1/threads/${thread_id}/messages?dir=f&limit=100&from=${from}`,
						);
						updateData("timelines", thread_id, (i) => [
							...i.slice(0, slice.end - 1),
							...batch.items.map((j: MessageT) => ({
								type: "remote",
								message: j,
							})),
							...batch.has_more ? [{ type: "hole" }] : [],
						]);
					}

					const newTl = data.timelines[thread_id];
					const newOff = newTl.indexOf(nextItem) - slice.end - 1;
					const newEnd = Math.min(
						slice.end + newOff + PAGINATE_LEN,
						newTl.length,
					);
					const newStart = Math.max(newEnd - SLICE_LEN, 0);
					console.log({ start: newStart, end: newEnd });
					updateData("slices", thread_id, { start: newStart, end: newEnd });
				}
				return;
			}
			case "menu": {
				console.log("handle menu", action.menu);
				updateData("menu", action.menu);
				return;
			}
			// case "modal.open": {
			// 	updateData("modals", i => [action.modal, ...i ?? []]);
			// 	return;
			// }
			case "modal.close": {
				updateData("modals", i => i.slice(1));
				return;
			}
			case "modal.alert": {
				updateData("modals", i => [{ type: "alert", text: action.text }, ...i ?? []]);
				return;
			}
			case "modal.confirm": {
				const p = Promise.withResolvers();
				const modal = {
					type: "confirm",
					text: action.text,
					cont: p.resolve,
				};
				updateData("modals", i => [modal, ...i ?? []]);
				return p.promise;
			}
			case "modal.prompt": {
				const p = Promise.withResolvers();
				const modal = {
					type: "prompt",
					text: action.text,
					cont: p.resolve,
				};
				updateData("modals", i => [modal, ...i ?? []]);
				return p.promise;
			}
		}
	}
	
	globalThis.prompt = (text) => dispatch({ do: "modal.prompt", text }) as Promise<string | null>;
	globalThis.confirm = (text) => dispatch({ do: "modal.prompt", text }) as Promise<boolean>;

	globalThis.addEventListener("keydown", e => {
		if (e.key === "Escape") dispatch({ do: "modal.close" });
	});

	return (
		<div id="root" class="flex h-screen font-sans">
			<chatctx.Provider value={{ client, data, dispatch }}>
				<ChatNav />
				{getComponent()}
				<Portal>
					<For each={data.modals}>{(modal) => (
						<Switch>
							<Match when={modal.type === "alert"}>
								<Modal>
									<p>{modal.text}</p>
									<div class="h-0.5"></div>
									<button onClick={() => dispatch({ do: "modal.close" })} class={CLASS_BUTTON}>okay!</button>
								</Modal>
							</Match>
							<Match when={modal.type === "confirm"}>
								<Modal>
									<p>{modal.text}</p>
									<div class="h-0.5"></div>
									<button onClick={() => {modal.cont(true); dispatch({ do: "modal.close" })}} class={CLASS_BUTTON}>okay!</button>&nbsp;
									<button onClick={() => {modal.cont(false); dispatch({ do: "modal.close" })}} class={CLASS_BUTTON}>nevermind...</button>
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
										dispatch({ do: "modal.close" });
									}}>
										<input class="bg-bg3 border-[1px] border-sep" type="text" name="text" autofocus />
										<div class="h-0.5"></div>
										<input type="submit" class={CLASS_BUTTON} value="done!" />&nbsp;
										<button
											onClick={() => {modal.cont(null); dispatch({ do: "modal.close" })}}
											class={CLASS_BUTTON}
										>nevermind...</button>
									</form>
								</Modal>
							</Match>
						</Switch>
					)}</For>
					<Show when={data.menu}>
						<div
							ref={setMenuRef}
							class="fixed"
							style={{
								top: `${menuFloating.y}px`,
								left: `${menuFloating.x}px`,
							}}
						>
							<Switch>
								<Match
									when={data.menu.type === "room"}
									children={<RoomMenu />}
								/>
								<Match
									when={data.menu.type === "thread"}
									children={<ThreadMenu />}
								/>
								<Match
									when={data.menu.type === "message"}
									children={<MessageMenu />}
								/>
							</Switch>
						</div>
					</Show>
				</Portal>
			</chatctx.Provider>
		</div>
	);
};

const Modal = (props: ParentProps) => {
	const ctx = useContext(chatctx)!;
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

export default App;
