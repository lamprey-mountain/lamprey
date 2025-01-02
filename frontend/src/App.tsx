import { Component, JSX, Show, untrack } from "solid-js";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import { Dynamic } from "solid-js/web";
import { ChatMain, ChatNav } from "./Chat.tsx";
import { Client, Room, Thread } from "sdk";
import { chatctx } from "./context.ts";

const BASE_URL = "http://localhost:8000";
// const TOKEN = "0a11b93f-ff19-4c56-9bd2-d25bede776de";
const TOKEN = "abcdefg";

const App: Component = () => {
	const [hash, setHash] = createSignal(location.hash.slice(1));
	const [title, setTitle] = createSignal(document.title);
	const [isReady, setIsReady] = createSignal(false);
	const [roomId, setRoomId] = createSignal<string | undefined>(
		"0194241c-51ca-71b4-a473-87afe8f0754b",
	);
	const [threadId, setThreadId] = createSignal<string | undefined>(
		"0194241c-51e5-77d6-884e-1208fc013c98",
	);

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
	globalThis.client = client;
	client.connect();

	createEffect(() => roomId() && client.fetchRoom(roomId()!));
	createEffect(() => threadId() && client.fetchThread(threadId()!));
	createEffect(() => roomId() && client.fetchThreadsInRoom(roomId()!));

	const handleHashChange = () => setHash(location.hash.slice(1));
	globalThis.addEventListener("hashchange", handleHashChange);
	onCleanup(() => {
		globalThis.removeEventListener("hashchange", handleHashChange);
	});
	createEffect(() => document.title = title());
	createEffect(() => location.hash = hash());
	// createEffect(() => setTitle(parts.get(hash())?.title ?? "unknown"));

	return (
		<div id="root" class="flex h-screen font-sans">
			<chatctx.Provider value={{ client, roomId, threadId, thread, room, setRoomId, setThreadId }}>
				<ChatNav rooms={rooms()} threads={threads()} />
				<Show when={thread()} fallback={<div>thread not found...</div>}>
					<ChatMain thread={thread()!} />
				</Show>
			</chatctx.Provider>
		</div>
	);
};

export default App;
