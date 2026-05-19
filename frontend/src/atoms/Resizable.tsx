import { createSignal, onMount, type ParentProps } from "solid-js";

type ResizableProps = ParentProps<{
	storageKey: string;
	initialWidth: number;
	minWidth?: number;
	maxWidth?: number;
	side?: "left" | "right";
	classList?: Record<string, boolean>;
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
			classList={props.classList}
			data-side={side()}
			style={{ "--width": `${width()}px` }}
		>
			<div class="resize-handle" onMouseDown={handleMouseDown} />
			{props.children}
		</div>
	);
};

export const PaneResizeHandle = (props: {
	isHorizontal: boolean;
	onResize: (size: number) => void;
}) => {
	const handleMouseDown = (e: MouseEvent) => {
		e.preventDefault();
		document.body.classList.add("resizing");

		const startX = e.clientX;
		const startY = e.clientY;
		const prevSibling = (e.currentTarget as HTMLElement)
			.previousElementSibling as HTMLElement;
		const startSize = props.isHorizontal
			? prevSibling.offsetWidth
			: prevSibling.offsetHeight;

		const handleMouseMove = (ev: MouseEvent) => {
			const dx = ev.clientX - startX;
			const dy = ev.clientY - startY;
			let newSize = startSize + (props.isHorizontal ? dx : dy);
			if (newSize < 100) newSize = 100;
			props.onResize(newSize);
		};

		const handleMouseUp = () => {
			window.removeEventListener("mousemove", handleMouseMove);
			window.removeEventListener("mouseup", handleMouseUp);
			document.body.classList.remove("resizing");
		};

		window.addEventListener("mousemove", handleMouseMove);
		window.addEventListener("mouseup", handleMouseUp);
	};

	return (
		<div
			class={`pane-resize-handle ${props.isHorizontal ? "horizontal" : "vertical"}`}
			onMouseDown={handleMouseDown}
		/>
	);
};
