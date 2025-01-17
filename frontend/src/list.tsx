import { JSX } from "solid-js/jsx-runtime";
import { For, on, onMount } from "solid-js";
import { Accessor, createSignal, createEffect, onCleanup } from "solid-js";
import { reconcile } from "solid-js/store";
import { MessageT } from "./types.ts";
import { Data } from "./context.ts";

export type TimelineStatus = "loading" | "update" | "ready";

export type SliceInfo = {
  start: number,
  end: number,
};

// TODO: dynamically calculate how many events are needed
const SLICE_COUNT = 100;
// const PAGINATE_COUNT = SLICE_COUNT * 3;
const PAGINATE_COUNT = SLICE_COUNT;
const AUTOSCROLL_MARGIN = 5;
const SCROLL_MARGIN = 100;
const PAGINATE_MARGIN = SCROLL_MARGIN + 50;

// export function shouldSplit(msg: Event, prev?: Event) {
//   if (!prev) return true;
//   if (msg.sender !== prev.sender) return true;
//   if (msg.originTs - prev.originTs > 1000 * 60 * 5) return true;
//   return false;
// }

export function createList<T>(options: {
  items: Accessor<Array<T>>,
  autoscroll?: Accessor<boolean>,
  // topPos?: Accessor<number>,
  // bottomPos?: Accessor<number>,
  topQuery: string,
  bottomQuery: string,
  onPaginate?: (dir: "forwards" | "backwards") => void,
  onScroll?: (pos: number) => void,
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
  const intersections = new IntersectionObserver((entries) => {
    // PERF: run intersection callback takes too long
    console.log("list::intersection", entries);
    for (const el of entries) {
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
    }
  }, {
    rootMargin: `${margin}px 0px ${margin}px 0px`,
  });

  const resizes = new ResizeObserver((_entries) => {
    // NOTE: fine for instantaneous resizes, janky when trying to smoothly resize
    if (shouldAutoscroll) {
      console.log("list::autoscroll");
      wrapperEl()!.scrollTo({ top: 999999, behavior: "instant" });
    }
  });

  function setRefs() {
    setTopEl(wrapperEl()!.querySelector(options.topQuery)! as HTMLElement);
    setBottomEl(wrapperEl()!.querySelector(options.bottomQuery)! as HTMLElement);
  }

  onCleanup(() => {
    intersections.disconnect();
    resizes.disconnect();
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
        console.log(shouldAutoscroll)
        if (!wrap || !anchorRef) return setRefs();
        console.time("perf::reanchor");
        if (shouldAutoscroll) {
          // console.log("list::autoscroll");
          wrap.scrollTo({ top: 999999, behavior: "instant" });
        } else {
          // FIXME: tons of reflow and jank
          console.time("perf::forceReflow");
          const currentRect = anchorRef.getBoundingClientRect();
          console.timeEnd("perf::forceReflow");
          const diff = (currentRect.y - anchorRect.y) + (currentRect.height - anchorRect.height);
          console.log({ diff, currentRect, anchorRect });
          wrapperEl()?.scrollBy(0, diff);
        }
        setRefs();
        console.timeEnd("perf::reanchor");
      }
      
      createEffect(on(options.items, () => {
        queueMicrotask(reanchor);
      }));

      createEffect(on(topEl, (topEl) => {
        if (!topEl) return;
        if (topRef) intersections.unobserve(topRef);
        topRef = topEl;
        intersections.observe(topEl);
      }));

      createEffect(on(bottomEl, (bottomEl) => {
        if (!bottomEl) return;
        if (bottomRef) intersections.unobserve(bottomRef);
        bottomRef = bottomEl;
        intersections.observe(bottomEl);
      }));

      onMount(() => {
        resizes.observe(wrapperEl()!);
      });
      
      return (
        <ul
          class="list"
          ref={setWrapperEl}
          onContextMenu={options.onContextMenu}
          onScroll={() => options.onScroll?.(wrapperEl()!.scrollTop)}
        >
          <For each={options.items()}>
            {(item, idx) => props.children(item, idx)}
          </For>
        </ul>
      );
    },
  };
}
