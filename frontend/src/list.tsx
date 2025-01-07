import { JSX } from "solid-js/jsx-runtime";
import { For, on } from "solid-js";
import { Accessor, createSignal, createEffect, onCleanup } from "solid-js";
import { TimelineSet, Message } from "sdk";
import { reconcile } from "solid-js/store";
import { MessageT } from "./types.ts";
import { Data, Timeline } from "./context.ts";

export type TimelineStatus = "loading" | "update" | "ready";

export type SliceInfo = {
  start: number,
  end: number,
};

// TODO: dynamically calculate how many events are needed
const SLICE_COUNT = 100;
// const PAGINATE_COUNT = SLICE_COUNT * 3;
const PAGINATE_COUNT = SLICE_COUNT;

export type TimelineItemT = { key: string, class?: string } & (
  { type: "info", header: boolean } |
  { type: "editor" } |
  { type: "spacer" } |
  { type: "spacer-mini" } |
  { type: "unread-marker" } |
  { type: "time-split" } |
  { type: "message", message: MessageT, separate: boolean });


// export function shouldSplit(msg: Event, prev?: Event) {
//   if (!prev) return true;
//   if (msg.sender !== prev.sender) return true;
//   if (msg.originTs - prev.originTs > 1000 * 60 * 5) return true;
//   return false;
// }

const AUTOSCROLL_MARGIN = 5;
const SCROLL_MARGIN = 100;
const PAGINATE_MARGIN = SCROLL_MARGIN + 50;

export function createList<T>(options: {
  items: Accessor<Array<T>>,
  autoscroll?: Accessor<boolean>,
  topPos?: Accessor<number>,
  bottomPos?: Accessor<number>,
  onPaginate?: (dir: "forwards" | "backwards") => void,
  onUpdate?: () => void,
  onContextMenu?: (e: MouseEvent) => void,
}) {
  const [wrapperEl, setWrapperEl] = createSignal<HTMLElement>();
  const [topEl, setTopEl] = createSignal<HTMLElement>();
  const [bottomEl, setBottomEl] = createSignal<HTMLElement>();
  let topRef: HTMLElement | undefined;
  let bottomRef: HTMLElement | undefined;
  let anchorRef: Element;
  let anchorRect: DOMRect;
  let shouldAutoscroll = false;
  
  const margin = 0;
  const observer = new IntersectionObserver((entries) => {
    const el = entries[0];
    // console.log("list::intersection", entries);
    if (el.target === topEl()) {
      if (el.isIntersecting) {
        // console.log("list::up");
        anchorRef = el.target;
        anchorRect = el.boundingClientRect;
        options.onPaginate?.("backwards");
      }
    } else if (entries[0].target === bottomEl()) {
      if (el.isIntersecting) {
        // console.log("list::down");
        shouldAutoscroll = options.autoscroll?.() || false;
        anchorRef = el.target;
        anchorRect = el.boundingClientRect;
        options.onPaginate?.("forwards");
      } else {
        shouldAutoscroll = false;
        // console.log({ shouldAutoscroll })
      }
    } else {
      console.warn("list::unknownIntersectionEntry");
    }
  }, {
    rootMargin: `${margin}px 0px ${margin}px 0px`,
  });

  function setRefs() {
    const children = [...wrapperEl()?.children ?? []] as Array<HTMLElement>;
    setTopEl(children[options.topPos?.() ?? 0]);
    setBottomEl(children[options.bottomPos?.() ?? options.items().length - 1]);
  }

  onCleanup(() => {
    observer.disconnect();
  });
  
  return {
    scrollBy(pos: number, smooth = false) {
      wrapperEl()?.scrollBy({
        top: pos,
        behavior: smooth ? "smooth" : "instant",
      });
    },
    scrollTo(pos: number, smooth = false) {
      wrapperEl()?.scrollTo({
        top: pos,
        behavior: smooth ? "smooth" : "instant",
      });
    },
    List(props: { children: (item: T, idx: Accessor<number>) => JSX.Element }) {
      function reanchor() {
        const wrap = wrapperEl();
        // console.log("list::reanchor", wrap, anchorRef);
        if (!wrap || !anchorRef) return setRefs();
        if (shouldAutoscroll) {
          // console.log("list::autoscroll");
          wrap.scrollBy({ top: 999999, behavior: "instant" });
        } else {
          // FIXME: tons of reflow and jank
          // console.time("perf::forceReflow");
          const currentRect = anchorRef.getBoundingClientRect();
          // console.timeEnd("perf::forceReflow");
          const diff = (currentRect.y - anchorRect.y) + (currentRect.height - anchorRect.height);
          wrapperEl()?.scrollBy(0, diff);
        }
        setRefs();
      }
      
      createEffect(on(options.items, () => {
        queueMicrotask(reanchor);
        // requestAnimationFrame(reanchor);
      }));

      createEffect(on(topEl, (topEl) => {
        if (!topEl) return;
        if (topRef) observer.unobserve(topRef);
        topRef = topEl;
        observer.observe(topEl);
      }));

      createEffect(on(bottomEl, (bottomEl) => {
        if (!bottomEl) return;
        if (bottomRef) observer.unobserve(bottomRef);
        bottomRef = bottomEl;
        observer.observe(bottomEl);
      }));
      
      return (
        <ul
          class="list-none py-[8px] flex flex-col overflow-y-auto [overflow-anchor:none]"
          ref={setWrapperEl}
          onContextMenu={options.onContextMenu}
        >
          <For each={options.items()}>
            {(item, idx) => props.children(item, idx)}
          </For>
        </ul>
      );
    },
  };
}
