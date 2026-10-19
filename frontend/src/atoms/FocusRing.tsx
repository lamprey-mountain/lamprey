import { autoUpdate, offset, size } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import { createSignal, Show } from "solid-js";
import { Portal } from "solid-js/web";

const getPadding = (reference: unknown) => {
	const el = reference instanceof HTMLElement ? reference : undefined;
	const inline = el ? Number(el.dataset.focusRingPaddingInline ?? 0) : 0;
	const block = el ? Number(el.dataset.focusRingPaddingBlock ?? 0) : 0;
	return {
		inline: Number.isFinite(inline) ? inline : 0,
		block: Number.isFinite(block) ? block : 0,
	};
};

/**
 * Floating focus ring to indicate which element is currently focused.
 */
export const FocusRing = (props: {
	reference: () => HTMLElement | null | undefined;
}) => {
	const [ringEl, setRingEl] = createSignal<HTMLDivElement>();

	const pos = useFloating(props.reference, ringEl, {
		whileElementsMounted: autoUpdate,
		strategy: "fixed",
		placement: "bottom-start",
		middleware: [
			offset(({ rects, elements }) => {
				const padding = getPadding(elements.reference);
				return {
					mainAxis: -(rects.reference.height + padding.block),
					crossAxis: -padding.inline,
				};
			}),
			size({
				apply({ rects, elements }) {
					const padding = getPadding(elements.reference);
					elements.floating.style.width = `${rects.reference.width + padding.inline * 2}px`;
					elements.floating.style.height = `${rects.reference.height + padding.block * 2}px`;
				},
			}),
		],
	});

	return (
		<Portal>
			<Show when={props.reference()}>
				<div
					ref={setRingEl}
					class="focus-ring"
					style={{
						position: pos.strategy,
						top: `${pos.y ?? 0}px`,
						left: `${pos.x ?? 0}px`,
						// hide until floating-ui calculates position
						visibility: pos.y == null ? "hidden" : "visible",
					}}
				/>
			</Show>
		</Portal>
	);
};
