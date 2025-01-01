import { createEffect, createResource, createSignal, For, Show } from "solid-js";
import { createStore } from "solid-js/store";
import * as styles from "./Thread.module.css";
import Editor from "./Editor.tsx";
import { Messages } from "./Messages.tsx";
import { uuidv7 } from "uuidv7";
// import type { paths } from "../../openapi.d.ts";
// import createFetcher from "npm:openapi-fetch";

import { Client, Data, getTimestampFromUUID } from "./client.ts";

export const ChatMain = (props: ChatProps) => {
  let scollEl: HTMLDivElement;
  
  const messages = () => {
    const thread = props.data.threads[props.threadId];
    if (thread) {
      return thread.messages.map(id => {
        const msg = props.data.messages[id];
        return {
          id: msg.message_id,
          body: msg.content,
          origin_ts: getTimestampFromUUID(msg.message_id),
          type: "message",
          sender: "tester",
          is_local: msg.is_local,
        };
      });
    } else {
      return null;
    }
  }
  
  
  // const [msgs, setMsgs] = createSignal([...messages]);
  // createEffect(() => console.log(data));
  
  // const iso = new IntersectionObserver(() => {
    
  // });
  async function handleSubmit({ text }: { text: string }) {
    if (text.startsWith("/thread")) {
      const name = text.slice("/thread ".length);
      await props.client.http("POST", `/api/v1/rooms/${props.roomId}/threads`, {
        name,
      });
      return;
    }
    const nonce = uuidv7();
    const rect = scollEl.getBoundingClientRect();
    const autoscroll = scollEl.scrollTop > rect.y - rect.height - 1;
    props.client.handleMessage({
      type: "upsert.message",
      message: {
    		"room_id": props.roomId,
    		"thread_id": props.threadId,
    		"message_id": nonce,
    		"version_id": nonce,
    		"content": text,
    		"attachments": [],
    		"embeds": [],
    		"reply": null,
    		"metadata": {},
    		"mentions_users": [],
    		"mentions_roles": [],
    		"mentions_everyone": false,
    		"mentions_threads": [],
    		"mentions_rooms": [],
    		"author_id": props.data.user.user_id,
    		"is_pinned": false,
    		"nonce": null,
    		is_local: true,
      }
    });
    if (autoscroll) scollEl.scrollBy(0, 999);
    // await new Promise(res => setTimeout(res, 1000));
    await props.client.http("POST", `/api/v1/rooms/${props.roomId}/threads/${props.threadId}/messages`, {
      content: text,
  		nonce: nonce,
    });
  }
  
  return (
    <div class={styles.thread}>
      <header>Here is a header.</header>
      <div class={styles.scroll} ref={scollEl!}>
        <Show when={messages()}>
          <Messages notime={false} messages={messages()!} />
        </Show>
        <div class={styles.editor}>
          <Editor onSubmit={handleSubmit} placeholder="send a message..." />
        </div>
     </div>
    </div>
  )
}

type ChatProps = {
  client: Client,
  data: Data,
  roomId: string,
  threadId: string,
}

export const ChatNav = (props: ChatProps) => {
  return (
    <nav style="width:180px;background:var(--color-bg3)">
      <ul>
        <For each={Object.values(props.data.rooms)}>{room =>
          <li style={`color:${props.roomId === room.room_id ? "red" : ""}`}>{room.name}
            <Show when={props.roomId === room.room_id}>
            <ul style="margin-left: 10px">
              <For each={Object.values(props.data.threads).filter(i => i.data.room_id === props.roomId)}>{thread =>
                <li style={`color:${props.threadId === thread.data.thread_id ? "blue" : ""}`}>thread {thread.data.name}</li>
              }</For>
            </ul>
            </Show>
          </li>
        }</For>
      </ul>
    </nav>
  )
}
