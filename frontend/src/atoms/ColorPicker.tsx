import { autoUpdate, flip, offset, shift } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import { createTooltip } from "@/atoms/Tooltip";
import { useMenu } from "@/contexts/mod.tsx";
import { Color, oklchToRgb } from "@/lib/colors";
import { compileShader, createWebGLProgram } from "@/lib/webgl";
import { icGear } from "@/utils/icons";
import colorPickerFrag from "./color-picker.frag?raw";
import colorPickerVert from "./color-picker.vert?raw";
import { Dropdown } from "./Dropdown";
import { Icon } from "./Icon";

// TODO: fine tune this const
const MAX_CHROMA = 0.4;

type ColorFormat = "hex" | "rgb" | "hsl" | "hsb" | "oklch";

export type ColorPickerProps = {
	onInput?: (color: string) => void;
	value?: string;
	hasAlpha?: boolean;
};

export const ColorPicker = (props: ColorPickerProps) => {
	const menu = useMenu();

	let bgCanvasRef: HTMLCanvasElement | undefined;
	let uiCanvasRef: HTMLCanvasElement | undefined;
	let hueMapRef: HTMLDivElement | undefined;
	let alphaMapRef: HTMLDivElement | undefined;

	let bgGl: WebGLRenderingContext | null = null;
	let uiGl: CanvasRenderingContext2D | null = null;
	let program: WebGLProgram | null = null;
	let hueUniformLocation: WebGLUniformLocation | null = null;

	const settingsTooltip = createTooltip({ tip: () => "Color settings" });

	const [format, setFormat] = createSignal<ColorFormat>("hex");

	const [color, setColor] = createSignal<Color>(
		new Color("oklch(0.5 0.1 200)"),
	);
	const [isDragging, setIsDragging] = createSignal(false);
	const [isDraggingHue, setIsDraggingHue] = createSignal(false);
	const [isDraggingAlpha, setIsDraggingAlpha] = createSignal(false);
	const oklch = createMemo(() => color().to("oklch"));

	const updateColor = (newColor: Color) => {
		setColor(newColor);
		props.onInput?.(newColor.toString());
	};

	const onPointerMove = (e: PointerEvent) => {
		if (isDraggingHue() && hueMapRef) {
			const rect = hueMapRef.getBoundingClientRect();
			const hue = Math.max(
				0,
				Math.min(360, ((e.clientX - rect.left) / rect.width) * 360),
			);
			const c = oklch();
			c.coords[2] = hue;
			updateColor(c);
		} else if (isDraggingAlpha() && alphaMapRef) {
			const rect = alphaMapRef.getBoundingClientRect();
			const alpha = Math.max(
				0,
				Math.min(1, (e.clientX - rect.left) / rect.width),
			);
			const c = color();
			c.alpha = alpha;
			updateColor(c.clone());
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
		const rect = bgCanvasRef!.getBoundingClientRect();
		const x = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
		const y = Math.max(0, Math.min(rect.height, e.clientY - rect.top));
		const c = oklch();
		c.coords[0] = 1 - y / rect.height; // L
		c.coords[1] = (x / rect.width) * MAX_CHROMA; // C
		updateColor(c);
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

	const obs = new ResizeObserver(() => resizeCanvases());
	onCleanup(() => obs.disconnect());

	const resizeCanvases = () => {
		const dpr = window.devicePixelRatio || 1;

		if (uiCanvasRef) {
			const rect = uiCanvasRef.getBoundingClientRect();
			uiCanvasRef.width = rect.width * dpr;
			uiCanvasRef.height = rect.height * dpr;
			uiGl?.scale(dpr, dpr);
		}

		if (bgCanvasRef) {
			const rect = bgCanvasRef.getBoundingClientRect();
			bgCanvasRef.width = rect.width * dpr;
			bgCanvasRef.height = rect.height * dpr;
		}
	};

	// setup canvases
	onMount(() => {
		uiGl = uiCanvasRef!.getContext("2d");

		bgGl = bgCanvasRef!.getContext("webgl2");
		if (bgGl) {
			// compile shaders
			const vs = compileShader(bgGl, bgGl.VERTEX_SHADER, colorPickerVert);
			const fs = compileShader(bgGl, bgGl.FRAGMENT_SHADER, colorPickerFrag);
			program = createWebGLProgram(bgGl, vs, fs);
			bgGl.useProgram(program);

			// quad covering (0, 0) to (1, 1)
			const positionBuffer = bgGl.createBuffer();
			bgGl.bindBuffer(bgGl.ARRAY_BUFFER, positionBuffer);
			bgGl.bufferData(
				bgGl.ARRAY_BUFFER,
				new Float32Array([0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1]),
				bgGl.STATIC_DRAW,
			);

			const positionLoc = bgGl.getAttribLocation(program, "a_position");
			bgGl.enableVertexAttribArray(positionLoc);
			bgGl.vertexAttribPointer(positionLoc, 2, bgGl.FLOAT, false, 0, 0);

			// get uniform locations
			hueUniformLocation = bgGl.getUniformLocation(program, "u_hue");
			const maxChromaLoc = bgGl.getUniformLocation(program, "u_maxChroma");
			bgGl.uniform1f(maxChromaLoc, MAX_CHROMA);
		}

		resizeCanvases();
	});

	// render gradient
	createEffect(() => {
		const currentHue = oklch().coords[2];
		if (!bgGl || !program || currentHue === null) return;

		// clear
		bgGl.viewport(0, 0, bgGl.canvas.width, bgGl.canvas.height);
		bgGl.clearColor(0, 0, 0, 0);
		bgGl.clear(bgGl.COLOR_BUFFER_BIT);

		// draw
		bgGl.uniform1f(hueUniformLocation, currentHue);
		bgGl.drawArrays(bgGl.TRIANGLES, 0, 6);
	});

	// render ui/reticles
	createEffect(() => {
		const canvas = uiCanvasRef;
		if (!canvas) return;
		if (!uiGl) return;

		const currentColor = oklch();
		const width = canvas.width;
		const height = canvas.height;

		// clear canvas
		uiGl.clearRect(0, 0, canvas.width, canvas.height);

		// current color marker
		uiGl.strokeStyle = "white";
		uiGl.lineWidth = 2;
		uiGl.beginPath();
		const currentX = (currentColor.coords[1] / MAX_CHROMA) * width;
		const currentY = (1 - currentColor.coords[0]) * height;
		uiGl.arc(currentX, currentY, 5, 0, 2 * Math.PI);
		uiGl.stroke();

		// hover marker
		const hover = hoverPosition();
		if (hover) {
			uiGl.strokeStyle = "rgba(255, 255, 255, 0.5)";
			uiGl.lineWidth = 1;
			uiGl.beginPath();
			uiGl.moveTo(hover.x, 0);
			uiGl.lineTo(hover.x, height);
			uiGl.moveTo(0, hover.y);
			uiGl.lineTo(width, hover.y);
			uiGl.stroke();
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
				"--alpha": oklch().coords[3] ?? 1,
			}}
		>
			<div
				class="canvas-container"
				onPointerDown={(e) => {
					setIsDragging(true);
					updateColorFromPointer(e);
				}}
				onPointerMove={(e) => {
					const rect = bgCanvasRef!.getBoundingClientRect();
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
				ref={(el) => obs.observe(el)}
			>
				<canvas ref={bgCanvasRef} class="canvas"></canvas>
				<canvas ref={uiCanvasRef} class="canvas"></canvas>
			</div>
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
							updateColor(c);
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
							onPointerDown={(e) => {
								setIsDraggingAlpha(true);
								const rect = e.currentTarget.getBoundingClientRect();
								const alpha = Math.max(
									0,
									Math.min(1, (e.clientX - rect.left) / rect.width),
								);
								const c = color();
								c.alpha = alpha;
								updateColor(c.clone());
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
			<div class="input-wrapper">
				<Dropdown
					options={[
						{ item: "hex", label: "hex" },
						{ item: "rgb", label: "rgb" },
						{ item: "hsl", label: "hsl" },
						{ item: "hsb", label: "hsb" },
						{ item: "oklch", label: "oklch" },
					]}
					onSelect={(item) => item && setFormat(item)}
					required
					selected={format()}
				/>
				<input
					type="text"
					placeholder="any color..."
					class="input"
					value={color().toString({ format: format() })}
					onInput={(e) => {
						try {
							updateColor(new Color(e.currentTarget.value));
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
			</div>
			<div class="presets">
				{/* TODO: preset colors (see frontend/src/lib/colors.ts, frontend/src/styles/theme.scss) */}
			</div>
		</div>
	);
};

export const ColorPickerButton = (props: ColorPickerProps) => {
	const [menuOpen, setMenuOpen] = createSignal(false);
	const [referenceEl, setReferenceEl] = createSignal<HTMLElement>();
	const [floatingEl, setFloatingEl] = createSignal<HTMLElement>();

	const position = useFloating(referenceEl, floatingEl, {
		whileElementsMounted: autoUpdate,
		middleware: [offset(8), flip(), shift()],
		placement: "right-end",
	});

	// TODO: animate color picker

	createEffect(() => {
		if (menuOpen()) {
			const onClickOutside = (e: MouseEvent) => {
				const target = e.target as HTMLElement;
				const ref = referenceEl();
				const float = floatingEl();
				if (ref && float && !ref.contains(target) && !float.contains(target)) {
					setMenuOpen(false);
				}
			};

			const onKeyDown = (e: KeyboardEvent) => {
				if (e.key === "Escape") {
					setMenuOpen(false);
				}
			};

			const onFocusOut = (e: FocusEvent) => {
				const target = e.relatedTarget as HTMLElement;
				const ref = referenceEl();
				const float = floatingEl();
				if (ref && float && !ref.contains(target) && !float.contains(target)) {
					setMenuOpen(false);
				}
			};

			window.addEventListener("click", onClickOutside);
			window.addEventListener("keydown", onKeyDown);
			window.addEventListener("focusout", onFocusOut);

			onCleanup(() => {
				window.removeEventListener("click", onClickOutside);
				window.removeEventListener("keydown", onKeyDown);
				window.removeEventListener("focusout", onFocusOut);
			});
		}
	});

	return (
		<>
			<button
				class="button color-picker-button"
				ref={setReferenceEl}
				onClick={() => setMenuOpen(!menuOpen())}
				style={{
					background: props.value ?? "transparent",
				}}
			></button>
			<Portal>
				<Show when={menuOpen()}>
					<div
						ref={setFloatingEl}
						style={{
							position: position.strategy,
							top: 0,
							left: 0,
							translate: `${position.x ?? 0}px ${position.y ?? 0}px`,
							"z-index": 1000,
						}}
						tabindex="0"
					>
						<ColorPicker {...props} />
					</div>
				</Show>
			</Portal>
		</>
	);
};

export type GradientPickerProps = {
	onInput?: (color: string) => void;
	value?: string;
	hasAlpha?: boolean;
};

export type GradientStop = {
	x: number;
	y: number;
	color: string;
};

// TODO: gradient picker is extremely work in progress, i'll finish implementing this later
export const GradientPicker = (props: GradientPickerProps) => {
	const [stops, setStops] = createSignal<GradientStop[]>([
		{ x: 0, y: 0.5, color: "#ff0000" },
		{ x: 1, y: 0.5, color: "#0000ff" },
	]);
	const [activeStopIndex, setActiveStopIndex] = createSignal<number | null>(
		null,
	);

	// get angle between start and end point
	const angle = createMemo(() => {
		const st = stops();
		const s = st[0];
		const e = st[st.length - 1];
		const dx = e.x - s.x;
		const dy = e.y - s.y;
		if (dx === 0 && dy === 0) return 90;
		let deg = Math.round((Math.atan2(dy, dx) * 180) / Math.PI + 90);
		if (deg < 0) deg += 360;
		return deg;
	});

	let containerRef: HTMLDivElement | undefined;

	// get position along the gradent angle
	const getStopPosition = (stop: GradientStop) => {
		const st = stops();
		const s = st[0];
		const e = st[st.length - 1];
		const dx = e.x - s.x;
		const dy = e.y - s.y;
		const len2 = dx * dx + dy * dy;
		if (len2 === 0) return 0;
		return ((stop.x - s.x) * dx + (stop.y - s.y) * dy) / len2;
	};

	const gradientStyle = createMemo(() => {
		const stopsWithPos = stops().map((stop) => ({
			...stop,
			position: getStopPosition(stop),
		}));
		const sortedStops = stopsWithPos.sort((a, b) => a.position - b.position);
		const stopStrings = sortedStops.map(
			(stop) => `${stop.color} ${stop.position * 100}%`,
		);
		return `linear-gradient(${angle()}deg, ${stopStrings.join(", ")})`;
	});

	return (
		<div class="gradient-picker">
			<div
				class="canvas-container"
				ref={containerRef}
				style={{
					background: gradientStyle(),
					position: "relative",
					height: "100px",
					cursor: "crosshair",
				}}
				onPointerDown={(e) => {
					// TODO: is this needed?
				}}
				onPointerMove={(e) => {
					if (!containerRef) return;
					const rect = containerRef.getBoundingClientRect();

					const idx = activeStopIndex();
					if (idx !== null) {
						const mouseX = Math.max(
							0,
							Math.min(1, (e.clientX - rect.left) / rect.width),
						);
						const mouseY = Math.max(
							0,
							Math.min(1, (e.clientY - rect.top) / rect.height),
						);

						setStops((prev) => {
							const newStops = [...prev];

							if (idx === 0 || idx === prev.length - 1) {
								// dragging start or end stop
								// get 1d positions of center stops
								const oldS = prev[0];
								const oldE = prev[prev.length - 1];
								const dxOld = oldE.x - oldS.x;
								const dyOld = oldE.y - oldS.y;
								const len2Old = dxOld * dxOld + dyOld * dyOld;

								const centerPositions = prev.map((stop) => {
									if (len2Old === 0) return 0;
									return (
										((stop.x - oldS.x) * dxOld + (stop.y - oldS.y) * dyOld) /
										len2Old
									);
								});

								// move the end stop
								newStops[idx] = { ...newStops[idx], x: mouseX, y: mouseY };

								const newS = newStops[0];
								const newE = newStops[newStops.length - 1];

								// update center stops
								for (let i = 1; i < newStops.length - 1; i++) {
									const t = centerPositions[i];
									newStops[i] = {
										...newStops[i],
										x: newS.x + (newE.x - newS.x) * t,
										y: newS.y + (newE.y - newS.y) * t,
									};
								}
							} else {
								// dragging a center stop
								const s = prev[0];
								const e2 = prev[prev.length - 1];
								const dx = e2.x - s.x;
								const dy = e2.y - s.y;
								const len2 = dx * dx + dy * dy;
								if (len2 > 0) {
									const t = Math.max(
										0,
										Math.min(
											1,
											((mouseX - s.x) * dx + (mouseY - s.y) * dy) / len2,
										),
									);
									newStops[idx] = {
										...newStops[idx],
										x: s.x + dx * t,
										y: s.y + dy * t,
									};
								}
							}
							return newStops;
						});
					}
				}}
				onPointerUp={() => {
					setActiveStopIndex(null);
				}}
				onPointerLeave={() => {
					setActiveStopIndex(null);
				}}
			>
				<For each={stops()}>
					{(stop, index) => (
						<div
							class="stop-marker"
							style={{
								"--x": stop.x,
								"--y": stop.y,
								"--position": getStopPosition(stop),
							}}
							onPointerDown={(e) => {
								e.stopPropagation();
								setActiveStopIndex(index());
							}}
						/>
					)}
				</For>
			</div>
			<div class="dim">Angle: {angle()}°</div>
			<Dropdown
				options={[
					{ item: "linear", label: "linear" },
					{ item: "radial", label: "radial" },
				]}
				// onSelect={(item) => item && setFormat(item)}
				required
				selected="linear"
			/>
			<div class="stops">
				<h3 class="dim">stops</h3>
				<For each={stops()}>
					{(stop) => (
						// TODO: button to add stops
						// TODO: button to remove stop
						// TODO: open color picker for a stop
						// TODO: change stop position
						<div class="stop">
							<input type="text" value={getStopPosition(stop)} />
							<ColorPickerButton
								hasAlpha={props.hasAlpha}
								// onInput={}
								value={stop.color}
							/>
						</div>
					)}
				</For>
			</div>
		</div>
	);
};
