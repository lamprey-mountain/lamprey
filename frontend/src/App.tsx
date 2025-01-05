import { Accessor, batch, Component, JSX, Match, Show, Switch, untrack } from "solid-js";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import { Dynamic, Portal } from "solid-js/web";
import { ChatMain } from "./Chat.tsx";
import { Client } from "sdk";
import { Action, chatctx, Data, Timeline, View } from "./context.ts";
import { RoomSettings } from "./Settings.tsx";
import { createStore, reconcile } from "solid-js/store";
import { MessageT, Pagination } from "./types.ts";
import { Menu, MessageMenu, RoomMenu, ThreadMenu } from "./Menu.tsx";
import { useFloating } from "solid-floating-ui";
import { ClientRectObject, ReferenceElement, shift } from "@floating-ui/dom";
import { ChatNav } from "./Nav.tsx";

const BASE_URL = "http://localhost:8000";
// const TOKEN = "0a11b93f-ff19-4c56-9bd2-d25bede776de";
const TOKEN = localStorage.getItem("token") ?? "abcdefg";

const CLASS_BUTTON = "px-1 bg-bg3 hover:bg-bg4 my-0.5";
const SLICE_LEN = 100;
const PAGINATE_LEN = 30;

const App: Component = () => {
	const [hash, setHash] = createSignal(location.hash.slice(1));
	const [title, setTitle] = createSignal(document.title);
	const [isReady, setIsReady] = createSignal(false);

	const [data, updateData] = createStore<Data>({
		rooms: {},
		threads: {},
		messages: {},
		timelines: {},
		slices: {},
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
      }
    })
  });

	const client = new Client(TOKEN, BASE_URL);
	client.events.on("ready", () => setIsReady(true));
	client.events.on("close", () => setIsReady(false));
	client.events.on("update", (msg) => {
		console.log("update");
		if (msg.type === "ready") {
			updateData("user", msg.user);
  	} else if (msg.type === "upsert.room") {
			updateData("rooms", msg.room.id, msg.room);
  	} else if (msg.type === "upsert.thread") {
			updateData("threads", msg.thread.id, msg.thread);
  	} else if (msg.type === "upsert.message") {
			updateData("messages", msg.message.id, msg.message);
			const { thread_id } = msg.message;
			if (!data.timelines[thread_id]) {
				const tl = {
					is_at_beginning: false,
					is_at_end: true,
					thread_id: thread_id,
					messages: [msg.message],
				};
				updateData("timelines", thread_id, { list: [tl] });
				updateData("slices", thread_id, { ...tl, parent: tl });
			} else {
				updateData("timelines", msg.message.thread_id, "list", (i) => i.is_at_end, "messages", (i) => [...i, msg.message]);
				if (data.slices[thread_id].is_at_end) {
					updateData("slices", thread_id, "messages", (msgs) => [...msgs, msg.message].slice(-SLICE_LEN));
				}
			}
  	}
	});
	client.connect();
	globalThis.client = client;
	
	(async () => {
    const data = await client.http("GET", `/api/v1/rooms?dir=f`);
		for (const room of data.items) {
			updateData("rooms", room.id, room);
		}
	})();

	const handleClick = () => {
		dispatch({ do: "menu", menu: null });
	}

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

	function createRoom() {
		client.http("POST", "/api/v1/rooms", {
			name: prompt("name?")
		})
	}
	
	function createThread(room_id: string) {
		client.http("POST", `/api/v1/rooms/${room_id}/threads`, {
			name: prompt("name?")
		})
	}

  function useInvite() {
		client.http("POST", `/api/v1/invites/${prompt("invite code?")}`, {});
  }

	function getComponent() {
		switch (data.view.view) {
			case "home": {
				return (
					<div class="flex-1 bg-bg2 text-fg2 p-4">
						<h2 class="text-xl">home</h2>
						<p>work in progress. expect bugs and missing polish.</p>
						<button class={CLASS_BUTTON} onClick={createRoom}>create room</button><br />
						<button class={CLASS_BUTTON} onClick={useInvite}>use invite</button><br />
					</div>
				);
			}
			case "room": {
				const room_id = data.view.room.id;
				return (
					<div class="flex-1 bg-bg2 text-fg2 p-4">
						<h2 class="text-xl">{data.view.room.name}</h2>
						<p>{data.view.room.description}</p>
						<button class={CLASS_BUTTON} onClick={() => createThread(room_id)}>create thread</button><br />
					</div>
				);
			}
			case "room-settings": {
				const room = data.view.room;
				return (
					<RoomSettings room={room} />
				);
			}
			case "thread": {
				const room = data.view.room;
				const thread = data.view.thread;
				return (
					<ChatMain room={room} thread={thread} />
				);
			}
		}
	}

	async function dispatch(action: Action) {
		switch (action.do) {
			case "setView": {
				updateData("view", action.to);
				if ("room" in action.to) {
					const room_id = action.to.room.id;
					const roomThreadCount = [...Object.values(data.threads)].filter(i => i.room_id === room_id).length;
					if (roomThreadCount === 0) {
				    const data = await client.http("GET", `/api/v1/rooms/${room_id}/threads?dir=f`);
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
			    const batch = await client.http("GET", `/api/v1/threads/${thread_id}/messages?dir=b&from=ffffffff-ffff-ffff-ffff-ffffffffffff`);
		    	const tl: Timeline = {
		    		is_at_beginning: !batch.has_more,
		    		is_at_end: true,
		    		thread_id,
		    		messages: batch.items,
		    	}
		    	// TODO: is there a race condition here that might drop messages?
					updateData("timelines", thread_id, { list: [tl] });
					updateData("slices", thread_id, { ...tl, parent: tl });
					return;
				}
				
		    if (dir === "b" && slice.is_at_beginning) return;
		    if (dir === "f" && slice.is_at_end) return;

				const tl = slice.parent;
				if (dir === "b") {
					const firstIdx = tl.messages.findIndex(i => i.id === slice.messages[0]?.id);
					if (firstIdx < PAGINATE_LEN && !tl.is_at_beginning) {
				    await paginateTimeline(dir, thread_id, slice.parent);
					}
					
					const newFirstIdx = tl.messages.findIndex(i => i.id === slice.messages[0]?.id);
					console.log("paginate", { firstIdx, newFirstIdx });
					const newStartIdx = Math.max(newFirstIdx - PAGINATE_LEN, 0);
			    const newEndIdx = Math.min(newStartIdx + SLICE_LEN, tl.messages.length - 1);
			    updateData("slices", thread_id, "is_at_beginning", tl.is_at_beginning && newStartIdx === 0);
			    updateData("slices", thread_id, "is_at_end", tl.is_at_end && newEndIdx === tl.messages.length - 1);
			    updateData("slices", thread_id, "messages", tl.messages.slice(newStartIdx, newEndIdx));
				} else {
					const lastIdx = tl.messages.findIndex(i => i.id === slice.messages.at(-1)?.id);
					if (tl.messages.length - lastIdx < PAGINATE_LEN && !tl.is_at_end) {
				    await paginateTimeline(dir, thread_id, slice.parent);
					}

					const newLastIdx = tl.messages.findIndex(i => i.id === slice.messages.at(-1)?.id);
			    const newEndIdx = Math.min(newLastIdx + SLICE_LEN, tl.messages.length - 1);
					const newStartIdx = Math.max(newEndIdx - PAGINATE_LEN, 0);
			    updateData("slices", thread_id, "is_at_beginning", tl.is_at_beginning && newStartIdx === 0);
			    updateData("slices", thread_id, "is_at_end", tl.is_at_end && newEndIdx === tl.messages.length - 1);
			    updateData("slices", thread_id, "messages", tl.messages.slice(newStartIdx, newEndIdx));
				}
    
				return;
			}
			case "menu": {
				console.log("handle menu", action.menu)
				updateData("menu", action.menu);
			}
		}
	}

	async function paginateTimeline(dir: "b" | "f", thread_id: string, tl: Timeline) {
    if (dir === "b" && tl.is_at_beginning) return;
    if (dir === "f" && tl.is_at_end) return;
    
    const url = new URL(`/api/v1/threads/${thread_id}/messages`, client.baseUrl);
    url.searchParams.set("dir", dir);
    url.searchParams.set("limit", "10");
    const before = tl.messages[0]?.id ?? "ffffffff-ffff-ffff-ffff-ffffffffffff";
    const after = tl.messages.at(-1)?.id ?? "00000000-0000-0000-0000-000000000000";
    if (dir === "f") {
      url.searchParams.set("from", after);
    } else {
      url.searchParams.set("from", before);
    }

    const batch: Pagination<MessageT> = await client.httpDirect("GET", url.href)
		if (dir === "f") {
			updateData("timelines", thread_id, "list", (i) => i === tl, "messages", i => [...i, ...batch.items]);
			updateData("timelines", thread_id, "list", (i) => i === tl, "is_at_end", !batch.has_more);
		} else {
			updateData("timelines", thread_id, "list", (i) => i === tl, "messages", i => [...batch.items, ...i]);
			updateData("timelines", thread_id, "list", (i) => i === tl, "is_at_beginning", !batch.has_more);
		}
	}
	
	        // <For each={globals.dialogs}>
	        //   {(dialog) => <Switch>
	        //     <Match when={dialog.type === "media"}>
	        //       <MediaDialog file={dialog.file} />
	        //     </Match>
	        //   </Switch>}
	        // </For>
	return (
		<div id="root" class="flex h-screen font-sans">
			<chatctx.Provider value={{ client, data, dispatch }}>
				<ChatNav />
				{getComponent()}
	      <Portal>
	        <Show when={data.menu}>
	          <div ref={setMenuRef} class="fixed" style={{ top: `${menuFloating.y}px`, left: `${menuFloating.x}px` }}>
	            <Switch>
	              <Match when={data.menu.type === "room"} children={<RoomMenu />} />
	              <Match when={data.menu.type === "thread"} children={<ThreadMenu />} />
	              <Match when={data.menu.type === "message"} children={<MessageMenu />} />
	            </Switch>
	          </div>
	        </Show>
	      </Portal>
			</chatctx.Provider>
		</div>
	);
};

export default App;
