import styles from './App.module.css';
import { JSX, Show, Component } from "solid-js";
import { onCleanup, createEffect, createSignal, For } from "solid-js";
import { Dynamic } from "solid-js/web";
import { ChatMain, ChatNav } from "./Chat.tsx";
import { blankData, Client } from "./client.ts";

const BASE_URL = "http://localhost:8000";
const TOKEN = "0a11b93f-ff19-4c56-9bd2-d25bede776de";

const App: Component = () => {
  const [hash, setHash] = createSignal(location.hash.slice(1));
  const [title, setTitle] = createSignal(document.title);
  const [data, setData] = createSignal(blankData);
  const [isReady, setIsReady] = createSignal(false);

  const client = new Client(
    TOKEN,
    BASE_URL,
    async () => {
      // console.log("ready")
    setIsReady(true);
    // (async () => {
    //   const http = createHttp(TOKEN);
    //   const rooms = await http("GET", "/api/v1/rooms?limit=100");
    //   console.log(rooms)
    //   for (const room of rooms) {
    //     setData(reconcile(data(), { type: "upsert.room", room }));
    //   }
    //   const threads = await http("GET", `/api/v1/rooms/${roomId()}/threads`);
    //   for (const thread of threads.threads) {
    //     setData(reconcile(data(), { type: "upsert.thread", thread }));
    //   }
    //   const thread = await http("GET", `/api/v1/rooms/${roomId()}/threads/${threadId()}`);
    //   setData(reconcile(data(), { type: "upsert.thread", thread }));
    //   let messages = await http("GET", `/api/v1/rooms/${roomId()}/threads/${threadId()}/messages?limit=100`);
    //   for (const message of messages.messages) {
    //     const rect = scollEl.getBoundingClientRect();
    //     const autoscroll = scollEl.scrollTop > rect.y - rect.height - 1;
    //     setData(reconcile(data(), { type: "upsert.message", message }));
    //     if (autoscroll) scollEl.scrollBy(0, 999);
    //   }
    //   if (messages.has_more) {
    //     while (true) {
    //       messages = await http("GET", `/api/v1/rooms/${roomId()}/threads/${threadId()}/messages?limit=100&after=${messages.messages.at(-1).message_id}`);
    //       for (const message of messages.messages) {
    //         const rect = scollEl.getBoundingClientRect();
    //         const autoscroll = scollEl.scrollTop > rect.y - rect.height - 1;
    //         setData(reconcile(data(), { type: "upsert.message", message }));
    //         if (autoscroll) scollEl.scrollBy(0, 999);
    //       }
    //       if (!messages.has_more) break;
    //     }
    //   }
    // })()
    },
    () => setIsReady(false),
    (data) => setData(data),
  );
  client.connect();
  
  const handleHashChange = () => setHash(location.hash.slice(1));
  globalThis.addEventListener("hashchange", handleHashChange);
  onCleanup(() => {
    globalThis.removeEventListener("hashchange", handleHashChange);
  });
  createEffect(() => document.title = title());
  createEffect(() => location.hash = hash());
  // createEffect(() => setTitle(parts.get(hash())?.title ?? "unknown"));

  const [roomId, setRoomId] = createSignal("0194203d-2150-7d45-bd2a-fcbe66f4f4a4");
  const [threadId, setThreadId] = createSignal("0194203d-2254-704d-ae2a-fe6c3779649b");

  return (
    <div id="root">
      <nav class={styles.nav}>
      </nav>
      <main class="flex-col" style="height: 100%;">
        <Show when={isReady()}>
          <ChatMain data={data()} client={client} roomId={roomId()} threadId={threadId()} />
          <ChatNav data={data()} client={client} roomId={roomId()} threadId={threadId()} />
        </Show>
      </main>
    </div>
  );
};

export default App;
