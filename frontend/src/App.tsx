import { Accessor, batch, Component, JSX, Match, Show, Switch, untrack } from "solid-js";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import { Dynamic } from "solid-js/web";
import { ChatMain, ChatNav } from "./Chat.tsx";
import { Client, Room, Thread } from "sdk";
import { Action, chatctx, Data, Timeline, View } from "./context.ts";
import { RoomSettings } from "./Settings.tsx";
import { createStore } from "solid-js/store";
import { MessageT, Pagination } from "./types.ts";

const BASE_URL = "http://localhost:8000";
// const TOKEN = "0a11b93f-ff19-4c56-9bd2-d25bede776de";
const TOKEN = localStorage.getItem("token") ?? "abcdefg";

const App: Component = () => {
	const [hash, setHash] = createSignal(location.hash.slice(1));
	const [title, setTitle] = createSignal(document.title);
	const [isReady, setIsReady] = createSignal(false);

	const [user, setUser] = createSignal<any>();
	const [rooms, setRooms] = createSignal<Array<Accessor<Room>>>([])
	const [threads, setThreads] = createSignal<Array<Accessor<Thread>>>([])
	const [data, updateData] = createStore<Data>({
		rooms: {},
		threads: {},
		messages: {},
		timelines: {},
		user: null,
		view: { view: "home" },
	});

	const client = new Client(TOKEN, BASE_URL);
	client.events.on("ready", () => setIsReady(true));
	client.events.on("close", () => setIsReady(false));
	client.events.on("update", (msg) => {
		console.log("update");
		// setRooms([...client.rooms.values().map(i => untrack(i))]);
		// setThreads([...client.threads.values().map(i => untrack(i))]);
		// if (roomId()) client.fetchRoom(roomId()!).then(r => setRoom(r));
		// if (threadId()) client.fetchThread(threadId()!).then(r => setThread(r));
		setUser(client.user);

		if (msg.type === "ready") {
			updateData("user", msg.user);
  	} else if (msg.type === "upsert.room") {
			updateData("rooms", msg.room.id, msg.room);
  	} else if (msg.type === "upsert.thread") {
			updateData("threads", msg.thread.id, msg.thread);
  	} else if (msg.type === "upsert.message") {
			updateData("messages", msg.message.id, msg.message);
			if (!data.timelines[msg.message.thread_id]) {
				updateData("timelines", msg.message.thread_id, {
					list: [{
						is_at_beginning: false,
						is_at_end: true,
						thread_id: msg.message.thread_id,
						messages: [msg.message],
					}],
				});
			} else {
				updateData("timelines", msg.message.thread_id, "list", (i) => i.is_at_end, "messages", (i) => [...i, msg.message]);
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

	const handleHashChange = () => setHash(location.hash.slice(1));
	globalThis.addEventListener("hashchange", handleHashChange);
	onCleanup(() => {
		globalThis.removeEventListener("hashchange", handleHashChange);
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
					<div class="flex-1 bg-bg2 text-fg2">
						no room selected<br />
						<button onClick={createRoom}>create room</button><br />
						<button onClick={useInvite}>use invite</button><br />
					</div>
				);
			}
			case "room": {
				const room_id = data.view.room.id;
				return (
					<div class="flex-1 bg-bg2 text-fg2">
						no thread selected<br />
						<button onClick={() => createThread(room_id)}>create thread</button><br />
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
				const { dir, thread_id, timeline } = action;
				if (!timeline) {
			    const batch = await client.http("GET", `/api/v1/threads/${thread_id}/messages?dir=b&from=ffffffff-ffff-ffff-ffff-ffffffffffff`);
		    	const tl: Timeline = {
		    		is_at_beginning: !batch.has_more,
		    		is_at_end: true,
		    		thread_id,
		    		messages: batch.items,
		    	}
		    	if (!data.timelines[thread_id]) {
						updateData("timelines", thread_id, {
							list: [tl],
						});
					} else {
						updateData("timelines", thread_id, "list", (i) => i.is_at_end, tl);
					}
					return;
				}
				
		    if (dir === "b" && timeline.is_at_beginning) return;
		    if (dir === "f" && timeline.is_at_end) return;
    
		    const url = new URL(`/api/v1/threads/${thread_id}/messages`, client.baseUrl);
		    url.searchParams.set("dir", dir);
		    url.searchParams.set("limit", "10");
		    const before = timeline.messages[0]?.id ?? "ffffffff-ffff-ffff-ffff-ffffffffffff";
		    const after = timeline.messages.at(-1)?.id ?? "00000000-0000-0000-0000-000000000000";
		    if (dir === "f") {
		      url.searchParams.set("from", after);
		    } else {
		      url.searchParams.set("from", before);
		    }

		    const batch: Pagination<MessageT> = await client.httpDirect("GET", url.href)
				if (dir === "f") {
					updateData("timelines", thread_id, "list", (i) => i === timeline, "messages", i => [...i, ...batch.items]);
					updateData("timelines", thread_id, "list", (i) => i === timeline, "is_at_end", !batch.has_more);
				} else {
					updateData("timelines", thread_id, "list", (i) => i === timeline, "messages", i => [...batch.items, ...i]);
					updateData("timelines", thread_id, "list", (i) => i === timeline, "is_at_beginning", !batch.has_more);
				}
			}
		}
	}
	
	return (
		<div id="root" class="flex h-screen font-sans">
			<chatctx.Provider value={{ client, data, dispatch }}>
				<ChatNav />
				{getComponent()}
			</chatctx.Provider>
		</div>
	);
};

export default App;
