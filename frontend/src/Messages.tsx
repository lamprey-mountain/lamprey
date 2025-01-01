// import { Tooltip } from "./Atoms.tsx";
const Tooltip = (props) => props.children;
import * as styles from "./Messages.module.css";
import { onMount, createEffect, createSignal, Switch, Match, lazy } from "solid-js";

type UserProps = {
  name: string,
}

const User = (props: UserProps) => {
  return (
    <div>
      <h3>{props.name}</h3>
      <p>some info here</p>
      <p>more stuff</p>
      <p>click to view full profile</p>
    </div>
  );
}

type Message = any;

type MessageProps = {
  message: Message,
}

type MessagesProps = {
  messages: Array<Message>,
  notime?: boolean,
}

export const Message = (props: MessageProps) => {
  let bodyEl: HTMLSpanElement;
  
  createEffect(async () => {
    props.message.body; // make it react
    // FIXME: flash of unhighlighted code on update
    const hljs = await import("highlight.js");
    for (const code of bodyEl.querySelectorAll("code[class*=language-]")) {
      hljs.default.highlightElement(code);
    }
  });
  
  return (
    <div class={styles.messageWrap}>
      <span class={styles.sender}><Tooltip tip={() => <User name={props.message.sender} />} attrs={{ class: "" }}>{props.message.sender}</Tooltip></span>
      {props.message.type === "message_html"
        ? <span class={styles.body} ref={bodyEl!} innerHTML={props.message.body}></span>
        : <span class={styles.body} ref={bodyEl!}>{props.message.body}</span>}
      <span class={styles.time}>{new Date(props.message.origin_ts).toDateString()}</span>
    </div>
  );
}

export const Messages = (props: MessagesProps) => {  
  return <>
    <ul class={styles.messages} classList={{ [styles.notime]: props.notime }}>
      {props.messages.map(i => <Switch>
        <Match when={i.type === "message" || i.type === "message_html"}>
          <li classList={{
            [styles.message]: true,
            [styles.unread]: i.unread,
            [styles.mention]: i.mention,
            [styles.is_local]: i.is_local,
          }}>
            <Message message={i} />
          </li>
        </Match>
        <Match when={i.type === "unread-marker" && false}>
          <li classList={{ [styles.message]: true, [styles.unreadMarker]: true }}>
            <div class={styles.messageWrap}>
              <span class={styles.sender}>-----</span>
              <span class={styles.body}>new messages</span>
            </div>
          </li>
        </Match>
        <Match when={i.type === "unread-marker"}>
          <li classList={{ [styles.unreadMarker2]: true }}>
            <hr />
            <span>unread messages</span>
            <hr />
          </li>
        </Match>
        <Match when={i.type === "time-split" && false}>
          <li classList={{
            [styles.message]: true,
            [styles.timeSplit]: true,
          }}>
            <div class={styles.messageWrap}>
              <span class={styles.sender}>-----</span>
              <span class={styles.body}>time changed to <time>{new Date(i.origin_ts).toDateString()}</time></span>
            </div>
          </li>
        </Match>
        <Match when={i.type === "time-split"}>
          <li classList={{
            [styles.timeSplit2]: true,
          }}>
            <hr />
            <time>{new Date(i.origin_ts).toDateString()}</time>
            <hr />
          </li>
        </Match>
      </Switch>)}
    </ul>
  </>;
};
