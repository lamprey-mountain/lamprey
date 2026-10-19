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
	const [keyboardFocus, setKeyboardFocus] = createSignal(false);

	const reference = () => overrideEl() ?? focusedEl();

	const updateFocusedEl = () => {
		queueMicrotask(() => {
			const active = document.activeElement;
			if (
				keyboardFocus() &&
				active instanceof HTMLElement &&
				active !== document.body &&
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

	const onKeyDown = (e: KeyboardEvent) => {
		if (
			e.key === "Shift" ||
			e.key === "Control" ||
			e.key === "Alt" ||
			e.key === "Meta"
		) {
			return;
		}
		setKeyboardFocus(true);
		updateFocusedEl();
	};
	const onPointerDown = () => {
		setKeyboardFocus(false);
		setFocusedEl(null);
	};

	document.addEventListener("keydown", onKeyDown);
	document.addEventListener("pointerdown", onPointerDown);
	document.addEventListener("focusin", updateFocusedEl);
	document.addEventListener("focusout", updateFocusedEl);
	onCleanup(() => {
		document.removeEventListener("keydown", onKeyDown);
		document.removeEventListener("pointerdown", onPointerDown);
		document.removeEventListener("focusin", updateFocusedEl);
		document.removeEventListener("focusout", updateFocusedEl);
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
