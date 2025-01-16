import { ValidComponent, ParentProps, createSignal, Show, JSX, Accessor } from "solid-js";
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

export function tooltip(props: TooltipProps, tip: ValidComponent, wrap: JSX.Element) {
  const [contentEl, setContentEl] = createSignal<HTMLDivElement>();
  const [tipEl, setTipEl] = createSignal<HTMLDivElement>();
  const [title, setTitle] = createSignal(getTitle());
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
      console.log(s);
      setAnimate(s.shouldAnim);
      clearTimeout(s.timeout);
    }
    setVisible(true);
    setTitle("");
  }
  
  function hideTip() {
    // TODO: exit animations? might be too much
    setTitle(getTitle());
    setVisible(false);
    if (props.animGroup) {
      const s = tooltipAnimSuppress.get(props.animGroup)!;
      s.shouldAnim = false;
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
    middleware: [shift({ padding: 8 }), offset({ mainAxis: 8 }), flip()],
  });
  
  return (
    <>
      <Dynamic
        component={props.component ?? "span"}
        classList={{ "has-tooltip": true }}
        ref={setContentEl!}
        title={title()}
        onMouseEnter={showTip}
        onMouseLeave={considerHidingTip}
        {...props.attrs}
      >{wrap}</Dynamic>
      <Show when={visible()}>
        <Portal mount={overlayEl}>
          <div
            onMouseEnter={showTipIfInteractive}
            onMouseLeave={considerHidingTip}
            onFocusOut={handleFocusOff}
            ref={setTipEl}
            style={{
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
