import { For, JSX, ParentProps, VoidProps, createSelector, createSignal, createUniqueId, useContext } from "solid-js";
import { Portal, Show } from "solid-js/web";
import { useFloating } from "solid-floating-ui";
import { autoUpdate, flip, offset } from "@floating-ui/dom";
import { chatctx, useCtx } from "./context.ts";
import { MessageT, RoomT, ThreadT } from "./types.ts";

const CLASS_SUBTEXT = "text-fg5 text-sm mt-[-4px]";

const [preview, setPreview] = createSignal();
const [vel, setVel] = createSignal(0);

export function Menu(props: ParentProps<{ submenu?: boolean }>) {
  return (
    <menu
      class="bg-bg3 border-sep border-[1px] shadow-asdf shadow-bg1 text-fg4 overflow-hidden min-w-[128px]"
      onMouseDown={(e) => !props.submenu && e.stopPropagation()}
      onMouseLeave={() => setPreview()}
    >
      <ul>
        {props.children}
      </ul>
    </menu>
  )
}

// TODO: move this out of global scope
// TODO: use triangle to submenu corners instead of dot with x axis
const pos: Array<[number, number]> = [];
globalThis.addEventListener("mousemove", (e) => {
  pos.push([e.x, e.y]);
  if (pos.length > 5) pos.shift();
  let vx = 0, vy = 0;
  for (let i = 1; i < pos.length; i++) {
    vx += pos[i - 1][0] - pos[i][0];
    vy += pos[i - 1][1] - pos[i][1];
  }
  setVel((vx / Math.hypot(vx, vy)) || 0);
});

export function Submenu(props: ParentProps<{ content: JSX.Element, onClick?: (e: MouseEvent) => void }>) {
  const [itemEl, setItemEl] = createSignal<Element | undefined>();
  const [subEl, setSubEl] = createSignal<HTMLElement | undefined>();
  const [hovered, setHovered] = createSignal(false);
  
  const dims = useFloating(itemEl, subEl, {
    whileElementsMounted: autoUpdate,
    middleware: [flip()],
    placement: "right-start",
  });

  const menuId = createUniqueId();
  let timeout: number;

  function handleMouseEnter() {
    if (!preview()) setPreview(menuId);
    let s = 1;
    const attempt = () => {
      const a = -vel() * (1 / s);
      if (a <= 0.3) {
        setPreview(menuId);
      } else {
        s += .01
        timeout = setTimeout(attempt, a);
      }
    }
    attempt();
  }
  
  
  function handleMouseLeave() {
    clearTimeout(timeout);
  }
  
      // class="[&:hover>*]:visible"
  return (
    <li
      ref={setItemEl}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <button
        class="border-none px-[8px] py-[2px] w-full text-left hover:bg-bg1/50"
        onClick={(e) => { e.stopPropagation(); props.onClick?.(e) }}
      >
        {props.content}
      </button>
      <div
        ref={setSubEl}
        class="px-[8px] w-max"
        style={{ position: dims.strategy, left: `${dims.x}px`, top: `${dims.y}px`, visibility: hovered() || preview() === menuId ? "visible" : "hidden" }}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
      >
        <Menu submenu>
          {props.children}
        </Menu>
      </div>
    </li>
  );
}

export function Item(props: ParentProps<{ onClick?: (e: MouseEvent) => void }>) {
	const ctx = useContext(chatctx)!;

  let timeout: number;
  function handleMouseEnter() {
    if (!preview()) setPreview();
    const attempt = () => {
      const a = -vel() * 20;
      if (a <= 0) {
        setPreview();
      } else {
        timeout = setTimeout(attempt, a);
      }
    }
    attempt();
  }
  
  function handleMouseLeave() {
    clearTimeout(timeout);
  }
	
  return (
    <li>
      <button
        class="border-none px-[8px] py-[2px] w-full text-left hover:bg-bg1/50"
        onClick={(e) => {
          e.stopPropagation();
          props.onClick?.(e);
          if (!props.onClick) ctx.dispatch({ do: "modal.alert", text: "todo" });
          ctx.dispatch({ do: "menu", menu: null });
        }}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        >
        {props.children}
        </button>
    </li>
  );
}

export function Separator() {
  return <li><hr class="border-none h-[1px] bg-bg4" /></li>
}

// the context menu for rooms
export function RoomMenu(props: { room: RoomT }) {
  const copyId = () => navigator.clipboard.writeText(props.room.id);
  
  return (
    <Menu>
      <Item>mark as read</Item>
      <Item>copy link</Item>
      <RoomNotificationMenu />
      <Separator />
      <Submenu content={"edit"}>
        <Item>info</Item>
        <Item>invites</Item>
        <Item>roles</Item>
        <Item>members</Item>
      </Submenu>
      <Item>leave</Item>
      <Separator />
      <Item onClick={copyId}>copy id</Item>
      <Item>inspect</Item>
    </Menu>
  )
}

// the context menu for users
export function UserMenu() {
  return (
    <Menu>
      <Item>block</Item>
      <Item>dm</Item>
      <Separator />
      <Item>kick</Item>
      <Item>ban</Item>
      <Item>mute</Item>
      <Item>roles</Item>
      <Separator />
      <Item>copy id</Item>
    </Menu>
  )
}

function ThreadNotificationMenu() {
  return (
    <>
      <Submenu content={"notifications"}>
        <Item>
          <div>default</div>
          <div class={CLASS_SUBTEXT}>Uses the room's default notification setting.</div>
        </Item>
        <Item>
          <div>everything</div>
          <div class={CLASS_SUBTEXT}>You will be notified of all new messages in this thread.</div>
        </Item>
        <Item>
          <div>watching</div>
          <div class={CLASS_SUBTEXT}>Messages in this thread will show up in your inbox.</div>
        </Item>
        <Item>
          <div>mentions</div>
          <div class={CLASS_SUBTEXT}>You will only be notified on @mention</div>
        </Item>
        <Separator />
        <Item>bookmark</Item>
        <Submenu content={"remind me"}>
          <Item>in 15 minutes</Item>
          <Item>in 3 hours</Item>
          <Item>in 8 hours</Item>
          <Item>in 1 day</Item>
          <Item>in 1 week</Item>
        </Submenu>
      </Submenu>
      <Submenu content={"mute"}>
        <Item>for 15 minutes</Item>
        <Item>for 3 hours</Item>
        <Item>for 8 hours</Item>
        <Item>for 1 day</Item>
        <Item>for 1 week</Item>
        <Item>forever</Item>
      </Submenu>
    </>
  );
}

function RoomNotificationMenu() {
  return (
    <>
      <Submenu content={"notifications"}>
        <Item>
          <div>default</div>
          <div class={CLASS_SUBTEXT}>Uses your default notification setting.</div>
        </Item>
        <Item>
          <div>everything</div>
          <div class={CLASS_SUBTEXT}>You will be notified for all messages.</div>
        </Item>
        <Item>
          <div>new threads</div>
          <div class={CLASS_SUBTEXT}>You will be notified for new threads.</div>
        </Item>
        <Item>
          <div>watching</div>
          <div class={CLASS_SUBTEXT}>Threads and messages mark this room unread.</div>
        </Item>
        <Item>
          <div>mentions</div>
          <div class={CLASS_SUBTEXT}>You will only be notified on @mention</div>
        </Item>
      </Submenu>
      <Submenu content={"mute"}>
        <Item>for 15 minutes</Item>
        <Item>for 3 hours</Item>
        <Item>for 8 hours</Item>
        <Item>for 1 day</Item>
        <Item>for 1 week</Item>
        <Item>forever</Item>
      </Submenu>
    </>
  );
}

// the context menu for threads
export function ThreadMenu(props: { thread: ThreadT }) {
  const copyId = () => navigator.clipboard.writeText(props.thread.id);
  
  return (
    <Menu>
      <Item>mark as read</Item>
      <Item>copy link</Item>
      <ThreadNotificationMenu />
      <Separator />
      <Submenu content={"edit"}>
        <Item>info</Item>
        <Item>permissions</Item>
        <Submenu content={"tags"}>
          <Item>foo</Item>
          <Item>bar</Item>
          <Item>baz</Item>
        </Submenu>
      </Submenu>
      <Item>pin</Item>
      <Item>close</Item>
      <Item>lock</Item>
      <Item>delete</Item>
      <Separator />
      <Item onClick={copyId}>copy id</Item>
      <Item>view source</Item>
    </Menu>
  )
}

// the context menu for messages
// should i have a separate one for bulk messages?
export function MessageMenu(props: { message: MessageT }) {
  const ctx = useCtx();
  const copyId = () => navigator.clipboard.writeText(props.message.id);
  const setReply = () => ctx.dispatch({ do: "thread.reply", thread_id: props.message.thread_id, reply_id: props.message.id });
  
  return (
    <Menu>
      <Item>mark unread</Item>
      <Item>copy link</Item>
      <Item onClick={setReply}>reply</Item>
      <Item>edit</Item>
      <Item>fork</Item>
      <Item>pin</Item>
      <Item>redact</Item>
      <Separator />
      <Item onClick={copyId}>copy id</Item>
      <Item>view source</Item>
    </Menu>
  )
}
