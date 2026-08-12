import {
	createEffect,
	createMemo,
	createSignal,
	onCleanup,
	Show,
} from "solid-js";
import { createTooltip } from "@/atoms/Tooltip";
import { useMenu } from "@/contexts/mod.tsx";
import { Color, oklchToRgb } from "@/lib/colors";
import { icGear } from "@/utils/icons";
import { Icon } from "./Icon";

// TODO: fine tune this const
const MAX_CHROMA = 0.4;

export type ColorpickerProps = {
	onInput?: (color: string) => void;
	value?: string;
	hasAlpha?: boolean;
};

export const ColorPicker = (props: ColorpickerProps) => {
	const menu = useMenu();

	let canvasRef: HTMLCanvasElement | undefined;
	let hueMapRef: HTMLDivElement | undefined;
	let alphaMapRef: HTMLDivElement | undefined;

	const settingsTooltip = createTooltip({ tip: () => "Color settings" });

	let cachedBackground: {
		img: ImageData;
		hue: number;
		width: number;
		height: number;
	} | null = null;
	const [color, setColor] = createSignal<Color>(
		new Color("oklch(0.5 0.1 200)"),
	);
	const [isDragging, setIsDragging] = createSignal(false);
	const [isDraggingHue, setIsDraggingHue] = createSignal(false);
	const [isDraggingAlpha, setIsDraggingAlpha] = createSignal(false);
	const oklch = createMemo(() => color().to("oklch"));

	const onPointerMove = (e: PointerEvent) => {
		if (isDraggingHue() && hueMapRef) {
			const rect = hueMapRef.getBoundingClientRect();
			const hue = Math.max(
				0,
				Math.min(360, ((e.clientX - rect.left) / rect.width) * 360),
			);
			const c = oklch();
			c.coords[2] = hue;
			setColor(c);
		} else if (isDraggingAlpha() && alphaMapRef) {
			const rect = alphaMapRef.getBoundingClientRect();
			const alpha = Math.max(
				0,
				Math.min(1, (e.clientX - rect.left) / rect.width),
			);
			const c = color();
			c.alpha = alpha;
			setColor(c.clone());
		}
	};
	const onPointerUp = () => {
		setIsDraggingHue(false);
		setIsDraggingAlpha(false);
	};

	window.addEventListener("pointermove", onPointerMove);
	window.addEventListener("pointerup", onPointerUp);

	onCleanup(() => {
		window.removeEventListener("pointermove", onPointerMove);
		window.removeEventListener("pointerup", onPointerUp);
	});

	const updateColorFromPointer = (e: PointerEvent) => {
		const rect = canvasRef!.getBoundingClientRect();
		const x = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
		const y = Math.max(0, Math.min(rect.height, e.clientY - rect.top));
		const c = oklch();
		c.coords[0] = 1 - y / rect.height; // L
		c.coords[1] = (x / rect.width) * MAX_CHROMA; // C
		setColor(c);
	};

	createEffect(() => {
		if (props.value) {
			try {
				setColor(new Color(props.value));
			} catch (e) {
				console.error("Failed to parse color", props.value, e);
			}
		}
	});

	const [hoverPosition, setHoverPosition] = createSignal<{
		x: number;
		y: number;
	} | null>(null);

	createEffect(() => {
		const canvas = canvasRef;
		if (!canvas) return;
		const ctx = canvas.getContext("2d");
		if (!ctx) return;

		const width = canvas.width || 1;
		const height = canvas.height || 1;
		const currentColor = oklch();
		const hue = currentColor.coords[2];

		if (
			!cachedBackground ||
			cachedBackground.hue !== hue ||
			cachedBackground.width !== width ||
			cachedBackground.height !== height
		) {
			const img = ctx.createImageData(width, height);
			renderGradientMap(hue, img);
			cachedBackground = { img, hue, width, height };
		}

		ctx.putImageData(cachedBackground.img, 0, 0);

		// Draw current color marker
		ctx.strokeStyle = "white";
		ctx.lineWidth = 2;
		ctx.beginPath();
		const currentX = (currentColor.coords[1] / MAX_CHROMA) * width;
		const currentY = (1 - currentColor.coords[0]) * height;
		ctx.arc(currentX, currentY, 5, 0, 2 * Math.PI);
		ctx.stroke();

		// Draw hover marker
		const hover = hoverPosition();
		if (hover) {
			ctx.strokeStyle = "rgba(255, 255, 255, 0.5)";
			ctx.lineWidth = 1;
			ctx.beginPath();
			ctx.moveTo(hover.x, 0);
			ctx.lineTo(hover.x, height);
			ctx.moveTo(0, hover.y);
			ctx.lineTo(width, hover.y);
			ctx.stroke();
		}
	});

	return (
		<div
			class="color-picker"
			style={{
				"--colorspace": "oklch", // TODO: allow switching colorspace
				"--luminance": oklch().coords[0] ?? 0,
				"--chroma": oklch().coords[1] ?? 0,
				"--hue": oklch().coords[2] ?? 0,
				"--alpha": oklch().coords[3] ?? 0,
			}}
		>
			<canvas
				ref={canvasRef}
				class="canvas"
				onPointerDown={(e) => {
					setIsDragging(true);
					updateColorFromPointer(e);
				}}
				onPointerMove={(e) => {
					const rect = canvasRef!.getBoundingClientRect();
					setHoverPosition({
						x: e.clientX - rect.left,
						y: e.clientY - rect.top,
					});
					if (isDragging()) {
						updateColorFromPointer(e);
					}
				}}
				onPointerUp={() => setIsDragging(false)}
				onPointerLeave={() => {
					setIsDragging(false);
					setHoverPosition(null);
				}}
			></canvas>
			<div>
				<label>
					<h3 class="dim range-label">
						Hue: {Math.round(oklch().coords[2] ?? 0)}
					</h3>
					<div
						ref={hueMapRef}
						class="range hue-map"
						onPointerDown={(e) => {
							setIsDraggingHue(true);
							const rect = e.currentTarget.getBoundingClientRect();
							const hue = Math.max(
								0,
								Math.min(360, ((e.clientX - rect.left) / rect.width) * 360),
							);
							const c = oklch();
							c.coords[2] = hue;
							setColor(c);
						}}
					>
						<div class="reticle"></div>
					</div>
				</label>
			</div>
			<Show when={props.hasAlpha ?? false}>
				<div>
					<label>
						<h3 class="dim range-label">
							Alpha: {Math.round((color().alpha ?? 1) * 100)}%
						</h3>
						<div
							ref={alphaMapRef}
							class="range alpha-map"
							style={{
								background: `linear-gradient(to right, transparent, ${color().toString({ format: "oklch" })})`,
							}}
							onPointerDown={(e) => {
								setIsDraggingAlpha(true);
								const rect = e.currentTarget.getBoundingClientRect();
								const alpha = Math.max(
									0,
									Math.min(1, (e.clientX - rect.left) / rect.width),
								);
								const c = color();
								c.alpha = alpha;
								setColor(c.clone());
							}}
						>
							<div
								class="reticle"
								style={{ left: `${(color().alpha ?? 1) * 100}%` }}
							></div>
						</div>
					</label>
				</div>
			</Show>
			<div style="display:flex;align-items:center">
				<button
					type="button"
					class="button icon-button"
					onClick={(e) => {
						if (menu.menu()) return;
						e.stopPropagation();
						menu.setMenu({
							type: "color_picker_options",
							onColorSpaceChange: (space) => {
								console.log("changed colorspace to", space);
								// TODO: actually implement colorspace switching
							},
							x: e.clientX,
							y: e.clientY,
						});
					}}
					ref={settingsTooltip.content}
				>
					<Icon src={icGear} />
				</button>
				<input
					type="text"
					placeholder="any color..."
					class="text-input"
					value={color().toString()}
					onInput={(e) => {
						try {
							setColor(new Color(e.currentTarget.value));
							// NOTE: do i want to call onInput with a parsed color?
							props.onInput?.(e.currentTarget.value);
						} catch (e) {
							// ignore invalid color
						}
					}}
					ref={(el) =>
						queueMicrotask(() => {
							el.focus();
							el.select();
						})
					}
				/>
			</div>
			<div class="presets">
				{/* TODO: preset colors (see frontend/src/lib/colors.ts, frontend/src/styles/theme.scss) */}
			</div>
		</div>
	);
};

// PERF: if i really wanted to make this go brrrr i could use webgl
function renderGradientMap(hue: number, img: ImageData) {
	const hRad = (hue * Math.PI) / 180;
	const cosH = Math.cos(hRad);
	const sinH = Math.sin(hRad);

	for (let y = 0; y < img.height; y++) {
		const l = 1 - y / img.height;
		let i = y * img.width * 4;
		for (let x = 0; x < img.width; x++) {
			const c = (x / img.width) * MAX_CHROMA;
			const [r, g, b] = oklchToRgb(l, c, cosH, sinH);
			img.data[i] = r;
			img.data[i + 1] = g;
			img.data[i + 2] = b;
			img.data[i + 3] = 255;
			i += 4;
		}
	}
}
