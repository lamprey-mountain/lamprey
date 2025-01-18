import { ValidComponent, ParentProps, createSignal, Show, JSX, Accessor, onCleanup } from "solid-js";
import { Dynamic, Portal } from "solid-js/web";
import { shift, offset, autoUpdate, flip, Placement } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";

// WARNING: this is potentially very laggy
// TODO: defer tooltip
type TooltipProps = {
  // tip: ValidComponent,
  tipText?: string,
  component?: ValidComponent,
  attrs?: Record<string, string>,
  interactive?: boolean,
  placement?: Placement,
  animGroup?: string,
  // "bottom-start"
}

type TooltipAnimState = {
  shouldAnim: boolean,
  timeout: number,
}

const tooltipAnimSuppress = new Map<string, TooltipAnimState>();

// TODO: only use one tooltip + event listener instead of per element
export function tooltip(props: TooltipProps, tip: ValidComponent, wrap: HTMLElement) {
  const [contentEl, setContentEl] = createSignal<HTMLDivElement>();
  const [tipEl, setTipEl] = createSignal<HTMLDivElement>();
  // const [title, setTitle] = createSignal(getTitle());
  const [visible, setVisible] = createSignal(false);
  const [animate, setAnimate] = createSignal(false);
  let popupRemoveTimeout: number;
  let isHovered = false;
	const overlayEl = document.getElementById("overlay")!;

	if (props.animGroup) {
	  const s = tooltipAnimSuppress.get(props.animGroup);
	  if (!s) {
  	  tooltipAnimSuppress.set(props.animGroup, { shouldAnim: true, timeout: 0 });
	  }
	}

  function getTitle() {
    return props.tipText ?? (typeof tip === "string" ? tip : "");
  }
  
  function showTip() {
    clearTimeout(popupRemoveTimeout);
    isHovered = true;
    if (visible()) return;
    if (props.animGroup) {
      const s = tooltipAnimSuppress.get(props.animGroup)!;
      // console.log(s);
      setAnimate(s.shouldAnim);
      s.shouldAnim = false;
      clearTimeout(s.timeout);
    }
    setVisible(true);
    wrap.title = "";
  }
  
  function hideTip() {
    // TODO: exit animations? might be too much
    wrap.title = getTitle();
    setVisible(false);
    if (props.animGroup) {
      const s = tooltipAnimSuppress.get(props.animGroup)!;
      s.timeout = setTimeout(() => {
        s.shouldAnim = true;
      }, 500);
    }
    isHovered = false;
  }
  
  function considerHidingTip() {
    // FIXME: nested popups/tooltips can cause issues with isHovered
    // maybe have global set of what is hovered and what is a parent of what
    isHovered = false;
    if (!props.interactive) return hideTip();
    if (tipEl()?.contains(document.activeElement)) return;
    popupRemoveTimeout = setTimeout(hideTip, 0);
  }
  
  function showTipIfInteractive() {
    if (props.interactive) showTip();
  }

  function handleFocusOff() {
    if (!isHovered) hideTip();
  }

  const pos = useFloating(contentEl, tipEl, {
    whileElementsMounted: autoUpdate,
    strategy: "fixed",
    placement: props.placement,
    middleware: [shift({ padding: 8 }), offset({ mainAxis: 4 }), flip()],
  });

  wrap.addEventListener("mouseenter", showTip);
  wrap.addEventListener("mouseleave", considerHidingTip);
  setContentEl(wrap);

  onCleanup(() => {
    wrap.addEventListener("mouseenter", showTip);
    wrap.addEventListener("mouseleave", considerHidingTip);
  });
  
  return (
    <>
      {wrap}
      <Show when={visible()}>
        <Portal mount={overlayEl}>
          <div
            onMouseEnter={showTipIfInteractive}
            onMouseLeave={considerHidingTip}
            onFocusOut={handleFocusOff}
            ref={setTipEl}
            style={{
              position: pos.strategy,
              translate: `${pos.x}px ${pos.y}px`,
              visibility: visible() ? "visible" : "hidden",
            }}
            class="tooltip"
            classList={{
              animate: animate(),
              interactive: props.interactive,
            }}>
            <div class="base"></div>
            <div class="inner">{tip}</div>
          </div>
        </Portal>
      </Show>
    </>
  )
}
