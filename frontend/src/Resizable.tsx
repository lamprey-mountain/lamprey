import { createSignal, onMount, type ParentProps } from "solid-js";

type ResizableProps = ParentProps<{
	storageKey: string;
	initialWidth: number;
	minWidth?: number;
	maxWidth?: number;
	side?: "left" | "right";
}>;

export const Resizable = (props: ResizableProps) => {
	const [width, setWidth] = createSignal(props.initialWidth);
	const minWidth = () => props.minWidth ?? 240;
	const maxWidth = () => props.maxWidth ?? 800;
	const side = () => props.side ?? "right";

	onMount(() => {
		const savedWidth = localStorage.getItem(props.storageKey);
		if (savedWidth) {
			setWidth(Number(savedWidth));
		}
	});

	const handleMouseDown = (e: MouseEvent) => {
		e.preventDefault();

		document.body.classList.add("resizing");

		const startX = e.clientX;
		const startWidth = width();

		const handleMouseMove = (e: MouseEvent) => {
			const dx = e.clientX - startX;
			let newWidth;
			if (side() === "right") {
				// Right sidebar, handle on left
				newWidth = startWidth - dx;
			} else {
				// Left sidebar, handle on right
				newWidth = startWidth + dx;
			}
			if (newWidth < minWidth()) newWidth = minWidth();
			if (newWidth > maxWidth()) newWidth = maxWidth();
			setWidth(newWidth);
		};

		const handleMouseUp = () => {
			window.removeEventListener("mousemove", handleMouseMove);
			window.removeEventListener("mouseup", handleMouseUp);
			document.body.classList.remove("resizing");
			localStorage.setItem(props.storageKey, String(width()));
		};

		window.addEventListener("mousemove", handleMouseMove);
		window.addEventListener("mouseup", handleMouseUp);
	};

	return (
		<div
			class="resizable-sidebar"
			data-side={side()}
			style={{ width: `${width()}px` }}
		>
			<div class="resize-handle" onMouseDown={handleMouseDown} />
			{props.children}
		</div>
	);
};
