import { Component, JSX, Match, Show, Switch, untrack } from "solid-js";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import { Dynamic } from "solid-js/web";
import { ChatMain, ChatNav } from "./Chat.tsx";
import { Client, Room, Thread } from "sdk";
import { chatctx } from "./context.ts";
import { RoomSettings } from "./Settings.tsx";

const BASE_URL = "http://localhost:8000";
// const TOKEN = "0a11b93f-ff19-4c56-9bd2-d25bede776de";
const TOKEN = localStorage.getItem("token") ?? "abcdefg";

const App: Component = () => {
	const [hash, setHash] = createSignal(location.hash.slice(1));
	const [title, setTitle] = createSignal(document.title);
	const [isReady, setIsReady] = createSignal(false);
	const [roomId, setRoomId] = createSignal<string | undefined>();
	const [threadId, setThreadId] = createSignal<string | undefined>();

	const [room, setRoom] = createSignal<Room>();
	const [thread, setThread] = createSignal<Thread>();
	const [rooms, setRooms] = createSignal<Array<Room>>([]);
	const [threads, setThreads] = createSignal<Array<Thread>>([]);

	const client = new Client(TOKEN, BASE_URL);
	client.events.on("ready", () => setIsReady(true));
	client.events.on("close", () => setIsReady(false));
	client.events.on("update", () => {
		console.log("update");
		setRooms([...client.rooms.values()]);
		setThreads([...client.threads.values()]);
		roomId() && setRoom(client.rooms.get(roomId()!));
		threadId() && setThread(client.threads.get(threadId()!));
	});
	client.connect();
	globalThis.client = client;
	
	(async () => {
		await client.temp_fetchRooms();
		setRooms([...client.rooms.values()]);
	})();

	createEffect(async () => {
		if (roomId() && !client.rooms.has(roomId()!)) await client.fetchRoom(roomId()!);
		if (roomId()) setRoom(client.rooms.get(roomId()!));
	});
	
	createEffect(async () => {
		if (threadId() === "settings") return;
		if (threadId() && !client.threads.has(threadId()!)) await client.fetchThread(threadId()!);
		if (threadId()) setThread(client.threads.get(threadId()!));
	});
	
	createEffect(async () => {
		await (roomId() && client.temp_fetchThreadsInRoom(roomId()!));
		console.log("fetch threads", [...client.threads.values()])
		setThreads([...client.threads.values()]);
	});

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
	
	function createThread() {
		client.http("POST", `/api/v1/rooms/${roomId()}/threads`, {
			name: prompt("name?")
		})
	}

  function useInvite() {
		client.http("POST", `/api/v1/invites/${prompt("invite code?")}`, {});
  }
	

	// createEffect(() => console.log(thread()))
	// createEffect(() => console.log(threadId()))

	return (
		<div id="root" class="flex h-screen font-sans">
			<chatctx.Provider value={{ client, roomId, threadId, thread, room, setRoomId, setThreadId }}>
				<ChatNav rooms={rooms()} threads={threads()} />
				<Switch>
					<Match when={room() && threadId() === undefined}>
						<div class="flex-1 bg-bg2 text-fg2">
							room home
						</div>
					</Match>
					<Match when={threadId() === "settings"}>
						<RoomSettings room={room()!} />
					</Match>
					<Match when={thread()}>
						<ChatMain thread={thread()!} />
					</Match>
					<Match when={room()}>
						<div class="flex-1 bg-bg2 text-fg2">
							no thread selected<br />
							<button onClick={createThread}>create thread</button><br />
						</div>
					</Match>
					<Match when={true}>
						<div class="flex-1 bg-bg2 text-fg2">
							no room selected<br />
							<button onClick={createRoom}>create room</button><br />
							<button onClick={useInvite}>use invite</button><br />
						</div>
					</Match>
				</Switch>
			</chatctx.Provider>
		</div>
	);
};

export default App;
