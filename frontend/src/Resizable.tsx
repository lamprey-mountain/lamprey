import { createSignal, onCleanup, onMount, type ParentProps } from "solid-js";

type ResizableProps = ParentProps<{
	storageKey: string;
	initialWidth: number;
	minWidth?: number;
	maxWidth?: number;
}>;

export const Resizable = (props: ResizableProps) => {
	const [width, setWidth] = createSignal(props.initialWidth);
	const minWidth = () => props.minWidth ?? 240;
	const maxWidth = () => props.maxWidth ?? 800;

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
			let newWidth = startWidth - dx; // Resizing from the left edge
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
		<div class="resizable-sidebar" style={{ width: `${width()}px` }}>
			<div class="resize-handle" onMouseDown={handleMouseDown} />
			{props.children}
		</div>
	);
};
