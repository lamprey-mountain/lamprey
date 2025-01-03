import { JSX } from "solid-js/jsx-runtime";
import { For, on } from "solid-js";
import { Accessor, createSignal, createEffect, onCleanup } from "solid-js";
import { TimelineSet, Timeline, Message } from "sdk";
import { reconcile } from "solid-js/store";

type TimelineStatus = "loading" | "update" | "ready";

type SliceInfo = {
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
  { type: "message", message: Message, separate: boolean });


// export function shouldSplit(msg: Event, prev?: Event) {
//   if (!prev) return true;
//   if (msg.sender !== prev.sender) return true;
//   if (msg.originTs - prev.originTs > 1000 * 60 * 5) return true;
//   return false;
// }

export function createTimeline(timelineSet: Accessor<TimelineSet>) {
  const [items, setItems] = createSignal<Array<TimelineItemT>>([]);
  const [info, setInfo] = createSignal<SliceInfo | null>(null);
  const [status, setStatus] = createSignal<TimelineStatus>("loading");
  const [timeline, setTimeline] = createSignal<Timeline>(null as any);
  const [isAtBeginning, setIsAtBeginning] = createSignal(true);
  const [isAtEnd, setIsAtEnd] = createSignal(true);
  const [isAutoscrolling, setIsAutoscrolling] = createSignal(true);

  function updateItems() {
    const { start, end } = info()!;
    const items: Array<TimelineItemT> = [];
    items.push({
      type: "info",
      key: "info" + isAtBeginning(),
      header: isAtBeginning(),
      class: "header",
    });
    if (!isAtBeginning()) {
      items.push({ type: "spacer", key: "space-begin" });
    } else {
      items.push({ type: "spacer", key: "space-begin" });
    }
    const messages = timeline().messages;
    // const lastAck = timelineSet()?.thread.unreads?.last_ack;

    for (let i = start; i < end; i++) {
      const msg = messages[i]
      items.push({
        type: "message",
        key: msg.id,
        message: msg,
        separate: true,
        // separate: shouldSplit(messages[i], messages[i - 1]),
      });
      // if (msg.id - prev.originTs > 1000 * 60 * 5) return true;
      // items.push({
      //   type: "message",
      //   key: messages[i].id,
      //   message: messages[i],
      //   separate: true,
      //   // separate: shouldSplit(messages[i], messages[i - 1]),
      // });
      // if (events[i].id === lastAck) {
      //   items.push({
      //     type: "unread-marker",
      //     key: "unread-marker",
      //   });
      // }
    }
    if (isAtEnd()) {
      items.push({ type: "spacer-mini", key: "space-end-mini" });
    } else {
      items.push({ type: "spacer", key: "space-end" });
    }
    // items.push({ type: "editor", key: "editor" });
    console.time("perf::updateItems");
    setItems((old) => [...reconcile(items, { key: "key" })(old)]);
    console.timeEnd("perf::updateItems");
  }

  async function init() {
    console.log("init");
    
    setTimeline(timelineSet().live);
    
    if (timeline().messages.length === 0) {
      await timeline().paginate("b", 30);
    }
    
    const totalEvents = timeline().messages.length;
    const newStart = Math.max(totalEvents - SLICE_COUNT, 0);
    const newEnd = Math.min(newStart + SLICE_COUNT, totalEvents);
    setInfo({ start: newStart, end: newEnd });
    // console.log({ totalEvents, ...info() });
    setIsAtBeginning(timeline().isAtBeginning && timeline().messages.length < SLICE_COUNT);
    setIsAtEnd(timeline() === timelineSet().live);
    setIsAutoscrolling(isAtEnd());
    setStatus("update");
    updateItems();
    setStatus("ready");
  }

  async function backwards() {
    if (status() !== "ready") return;
    if (isAtBeginning()) return;
    console.log("timeline::backwards");

    setStatus("loading");
    const currentInfo = info()!;
    const currentLen = timeline().messages.length;
    const count = currentInfo.start < SLICE_COUNT ? (await timeline().paginate("b", PAGINATE_COUNT)).messages.length - currentLen : 0;
    const newStart = Math.max(currentInfo.start + count - SLICE_COUNT / 2, 0);
    const newEnd = Math.min(newStart + SLICE_COUNT, timeline().messages.length);
    setInfo({ start: newStart, end: newEnd });
    setStatus("update");
    setIsAtBeginning(timeline().isAtBeginning && newStart === 0);
    setIsAtEnd(timeline().isAtEnd && newEnd === timeline().messages.length - 1);
    updateItems();
    setStatus("ready");
  }

  async function forwards() {
    if (status() !== "ready") return;
    if (isAtEnd()) return;
    console.log("timeline::forwards");

    setStatus("loading");
    const currentInfo = info()!;
    const currentLen = timeline().messages.length;
    const count = (await timeline().paginate("f", PAGINATE_COUNT)).messages.length - currentLen;
    const newEnd = Math.min(currentInfo.end + count + SLICE_COUNT / 2, timeline().messages.length);
    const newStart = Math.max(newEnd - SLICE_COUNT, 0);
    setInfo({ start: newStart, end: newEnd });
    setStatus("update");
    setIsAtBeginning(timeline().isAtBeginning && newStart === 0);
    setIsAtEnd(timeline().isAtEnd && newEnd === timeline().messages.length);
    updateItems();
    setStatus("ready");
  }

  // async function toEvent(_eventId: EventId) {
  //   throw new Error("todo!");
  // }

  // async function append(_event: Event) {
  async function append() {
    console.log("append", { status: status(), auto: isAutoscrolling(), timeline: timeline() });
    
    if (status() !== "ready") return;
    if (!isAutoscrolling()) return;

    const newEnd = timeline().messages.length;
    const newStart = Math.max(newEnd - SLICE_COUNT, 0);
    console.log({ start: newStart, end: newEnd });
    setInfo({ start: newStart, end: newEnd });
    setStatus("update");
    setIsAtBeginning(timeline().isAtBeginning && newStart === 0);
    setIsAtEnd(timeline().isAtEnd && newEnd === timeline().messages.length);
    updateItems();
    setStatus("ready");
  }

  createEffect(on(timelineSet, init));
  
  let oldTimeline: Timeline;
  createEffect(() => {
    timeline().events.on("append", append);
    timeline().events.on("prepend", append);
    oldTimeline?.events.off("append", append);
    oldTimeline?.events.off("prepend", append);
    oldTimeline = timeline();
  });

  onCleanup(() => {
    oldTimeline?.events.off("append", append);
    oldTimeline?.events.off("prepend", append);
  });
  
  return {
    items,
    status,
    backwards,
    forwards,
    // toEvent,
    isAtBeginning,
    isAtEnd,
    isAutoscrolling,
    setIsAutoscrolling,
  }
}

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
        <ul class="list-none py-[8px] flex flex-col overflow-y-auto [overflow-anchor:none]" ref={setWrapperEl}>
          <For each={options.items()}>
            {(item, idx) => props.children(item, idx)}
          </For>
        </ul>
      );
    },
  };
}
