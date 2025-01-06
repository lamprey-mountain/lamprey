import { For, JSX, ParentProps, VoidProps, createSignal, useContext } from "solid-js";
import { Portal, Show } from "solid-js/web";
import { useFloating } from "solid-floating-ui";
import { autoUpdate, flip, offset } from "@floating-ui/dom";
import { chatctx } from "./context.ts";

const CLASS_SUBTEXT = "text-fg5 text-sm mt-[-4px]";

export function Menu(props: ParentProps<{ submenu?: boolean }>) {
  return (
    <menu
      class="bg-bg3 border-sep border-[1px] shadow-asdf shadow-bg1 text-fg4 overflow-hidden min-w-[128px]"
      onmousedown={(e) => !props.submenu && e.stopPropagation()}
    >
      <ul>
        {props.children}
      </ul>
    </menu>
  )
}

export function Submenu(props: ParentProps<{ content: JSX.Element, onClick?: (e: MouseEvent) => void }>) {
  const [itemEl, setItemEl] = createSignal<Element | undefined>();
  const [subEl, setSubEl] = createSignal<HTMLElement | undefined>();
  const dims = useFloating(itemEl, subEl, {
    whileElementsMounted: autoUpdate,
    middleware: [flip()],
    placement: "right-start",
  });
  
  return (
    <li class="[&:hover>*]:visible" ref={setItemEl}>
      <button
        class="border-none px-[8px] py-[2px] w-full text-left hover:bg-bg1/50"
        onClick={(e) => { e.stopPropagation(); props.onClick?.(e) }}
      >
        {props.content}
      </button>
      <div ref={setSubEl} class="px-[8px] invisible" style={{ position: "fixed", left: `${dims.x}px`, top: `${dims.y}px` }}>
        <Menu submenu>
          {props.children}
        </Menu>
      </div>
    </li>
  );
}

export function Item(props: ParentProps<{ onClick?: (e: MouseEvent) => void }>) {
	const ctx = useContext(chatctx)!;
  return (
    <li>
      <button
        class="border-none px-[8px] py-[2px] w-full text-left hover:bg-bg1/50"
        onClick={(e) => {
          e.stopPropagation();
          props.onClick?.(e);
          if (!props.onClick) ctx.dispatch({ do: "modal.alert", text: "todo" });
          ctx.dispatch({ do: "menu", menu: null });
        }}>
        {props.children}
        </button>
    </li>
  );
}

export function Separator() {
  return <li><hr class="border-none h-[1px] bg-bg4" /></li>
}

// the context menu for rooms
export function RoomMenu() {
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
      <Item>copy id</Item>
      <Item>inspect</Item>
    </Menu>
  )
}

// the context menu for users
export function UserMenu() {
      // <Item>block</Item>
      // <Item>dm</Item>
      // <Separator />
      // <Item>kick</Item>
      // <Item>ban</Item>
      // <Item>mute</Item>
      // <Item>roles</Item>
      // <Separator />
      // <Item>copy id</Item>
  return (
    <Menu>
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
export function ThreadMenu() {
      // <Item>mark as read</Item>
      // <Item>copy link</Item>
      // <ThreadNotificationMenu />
      // <Separator />
      // <Submenu content={"edit"}>
      //   <Item>info</Item>
      //   <Item>permissions</Item>
      //   <Submenu content={"tags"}>
      //     <Item>foo</Item>
      //     <Item>bar</Item>
      //     <Item>baz</Item>
      //   </Submenu>
      // </Submenu>
      // <Item>pin</Item>
      // <Item>close</Item>
      // <Item>lock</Item>
      // <Item>delete</Item>
      // <Separator />
      // <Item>copy id</Item>
      // <Item>view source</Item>
  return (
    <Menu>
    </Menu>
  )
}

// the context menu for messages
export function MessageMenu() {
      // <Item>mark unread</Item>
      // {
      //   // <Item>copy link</Item>
      // }
      // <Item>reply</Item>
      // <Item>edit</Item>
      // <Item>fork</Item>
      // <Item>pin</Item>
      // <Item>redact</Item>
      // <Separator />
      // <Item>copy id</Item>
      // <Item>view source</Item>
  return (
    <Menu>
    </Menu>
  )
}
