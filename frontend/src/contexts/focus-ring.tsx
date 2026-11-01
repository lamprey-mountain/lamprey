import {
	createContext,
	createSignal,
	onCleanup,
	type ParentProps,
	useContext,
} from "solid-js";
import { FocusRing } from "@/atoms/FocusRing";

type FocusRingContextT = {
	/** the element the floating focus ring is anchored to */
	reference: () => HTMLElement | null;
	setOverride: (el: HTMLElement | null) => void;
};

const FocusRingContext = createContext<FocusRingContextT>();

export const FocusRingProvider = (props: ParentProps) => {
	const [focusedEl, setFocusedEl] = createSignal<HTMLElement | null>(null);
	const [overrideEl, setOverrideEl] = createSignal<HTMLElement | null>(null);

	const reference = () => overrideEl() ?? focusedEl();

	const updateFocusedEl = (target?: EventTarget | null) => {
		queueMicrotask(() => {
			const active = target ?? document.activeElement;
			if (
				active instanceof HTMLElement &&
				active !== document.body &&
				active.matches(":focus-visible") &&
				!active.isContentEditable &&
				!active.closest(".message-input")
			) {
				// HACK: janky way to override the focus ring's reference element
				const target =
					active.closest<HTMLElement>("[data-focus-ring-target]") ??
					active.querySelector<HTMLElement>("[data-focus-ring-target]") ??
					active;
				setFocusedEl(target);
			} else {
				setFocusedEl(null);
			}
		});
	};

	const onFocusIn = (e: FocusEvent) => updateFocusedEl(e.target);
	const onFocusOut = () => updateFocusedEl();
	const onPointerDown = () => setFocusedEl(null);

	document.addEventListener("pointerdown", onPointerDown);
	document.addEventListener("focusin", onFocusIn);
	document.addEventListener("focusout", onFocusOut);
	onCleanup(() => {
		document.removeEventListener("pointerdown", onPointerDown);
		document.removeEventListener("focusin", onFocusIn);
		document.removeEventListener("focusout", onFocusOut);
	});

	return (
		<FocusRingContext.Provider
			value={{ reference, setOverride: setOverrideEl }}
		>
			{props.children}
			<FocusRing reference={reference} />
		</FocusRingContext.Provider>
	);
};

export const useFocusRing = () => {
	const ctx = useContext(FocusRingContext);
	if (!ctx) {
		throw new Error("useFocusRing must be used within a FocusRingProvider");
	}
	return ctx;
};
