import {
	createContext,
	createMemo,
	createSignal,
	createUniqueId,
	For,
	type ParentProps,
	Show,
	useContext,
	type VoidProps,
} from "solid-js";

/** horizontal space reserved for the y-axis labels, in viewbox units */
const GUTTER = 64;
/** plot width in viewbox units */
const PLOT = 600;
const VIEWBOX_HEIGHT = 120;
const VIEWBOX = `${-GUTTER} -105 ${PLOT + GUTTER} ${VIEWBOX_HEIGHT}`;
const VIEWBOX_WIDTH = PLOT + GUTTER;

export type ChartsProps = {
	/** called with the start/end index of a dragged selection; only when they differ */
	onZoom?: (start: number, end: number) => void;
} & ParentProps;

export type ChartsContextData = [ChartsState, ChartsActions];

export type ChartsState = {
	hoveredIndex: number | null;
	selectionStartIndex: number | null;
};

export type ChartsActions = {
	setHoveredIndex: (index: number | null) => void;
	setSelectionStartIndex: (index: number | null) => void;
	triggerZoom: (endIndex: number) => void;
	clear: () => void;
};

type Tick = {
	x: number;
	label: string;
	anchor: "start" | "end" | "middle";
};

export type ChartProps = {
	points: Array<number>;
	/** fixed y-axis maximum, defaults to the largest data point */
	height?: number;
	/** value formatter used for the y-axis labels and the tooltip */
	format?: (value: number, index: number) => string;
	/** x-axis label formatter, receives the data point index */
	formatIndex?: (index: number) => string;
	/** tooltip content; omit to disable the tooltip */
	formatTooltip?: (index: number) => { label: string; value: string };
	emptyMessage?: string;
	ariaLabel?: string;
	onEmptyAction?: () => void;
	emptyActionLabel?: string;
};

const ChartsContext = createContext<ChartsContextData>();

export const useCharts = () => useContext(ChartsContext);

export const Charts = (props: ChartsProps) => {
	const [hoveredIndex, setHoveredIndex] = createSignal<number | null>(null);
	const [selectionStartIndex, setSelectionStartIndex] = createSignal<
		number | null
	>(null);

	const state: ChartsState = {
		get hoveredIndex() {
			return hoveredIndex();
		},
		get selectionStartIndex() {
			return selectionStartIndex();
		},
	};

	const actions: ChartsActions = {
		setHoveredIndex,
		setSelectionStartIndex,
		triggerZoom(endIndex: number) {
			const start = selectionStartIndex();
			if (start !== null && start !== endIndex) {
				props.onZoom?.(Math.min(start, endIndex), Math.max(start, endIndex));
			}
			setSelectionStartIndex(null);
		},
		clear() {
			setSelectionStartIndex(null);
			setHoveredIndex(null);
		},
	};

	return (
		<ChartsContext.Provider value={[state, actions]}>
			{props.children}
		</ChartsContext.Provider>
	);
};

const formatCompact = (value: number) => {
	if (Math.abs(value) < 10 && value % 1 !== 0) return value.toFixed(1);
	if (Math.abs(value) < 1000) return value.toFixed(0);
	return new Intl.NumberFormat(undefined, {
		notation: "compact",
		maximumFractionDigits: 1,
	}).format(value);
};

/** smallest nice step (1/2/2.5/5 x 10^n) that covers `value`, at least 1 */
const niceStep = (value: number) => {
	if (value <= 1) return 1;
	const magnitude = 10 ** Math.floor(Math.log10(value));
	for (const multiplier of [1, 2, 2.5, 5]) {
		const step = multiplier * magnitude;
		if (step >= value) return step;
	}
	return 10 * magnitude;
};

export const Chart = (props: VoidProps<ChartProps>) => {
	const context = useContext(ChartsContext);

	// fallback state so a chart works outside of a <Charts> wrapper
	const [localHoveredIndex, setLocalHoveredIndex] = createSignal<number | null>(
		null,
	);
	const [localSelectionStartIndex, setLocalSelectionStartIndex] = createSignal<
		number | null
	>(null);
	// only the chart under the pointer shows a tooltip, the crosshair is shared
	const [isPointerInside, setIsPointerInside] = createSignal(false);

	const hoveredIndex = () =>
		context ? context[0].hoveredIndex : localHoveredIndex();
	const selectionStartIndex = () =>
		context ? context[0].selectionStartIndex : localSelectionStartIndex();

	const setHoveredIndex = (value: number | null) => {
		if (context) context[1].setHoveredIndex(value);
		else setLocalHoveredIndex(value);
	};

	const setSelectionStartIndex = (value: number | null) => {
		if (context) context[1].setSelectionStartIndex(value);
		else setLocalSelectionStartIndex(value);
	};

	let svgRef: SVGSVGElement | undefined;

	// round the axis up to a multiple of 4 nice steps so grid labels stay readable
	const maxHeight = createMemo(() => {
		if (props.height !== undefined && props.height > 0) return props.height;
		const dataMax = props.points.reduce(
			(max, value) => Math.max(max, value),
			0,
		);
		if (dataMax <= 0) return 4;
		return niceStep(dataMax / 4) * 4;
	});

	// coordinate mapping to a PLOTx100 virtual viewbox, y grows upwards
	const scaleX = createMemo(
		() => PLOT / (props.points.length > 1 ? props.points.length - 1 : 1),
	);
	const scaleY = createMemo(() => 100 / maxHeight());

	const formatValue = (value: number, index: number) =>
		props.format?.(value, index) ?? formatCompact(value);

	const line = createMemo(() => {
		const pts = props.points;
		if (pts.length === 0) return "";
		const sx = scaleX();
		const sy = scaleY();
		return [
			`M 0 ${-pts[0] * sy}`,
			...pts.slice(1).map((value, i) => `L ${(i + 1) * sx} ${-value * sy}`),
		].join(" ");
	});

	const area = createMemo(() => {
		const pts = props.points;
		if (pts.length === 0) return "";
		const sx = scaleX();
		const sy = scaleY();
		return [
			"M 0 0",
			`L 0 ${-pts[0] * sy}`,
			...pts.slice(1).map((value, i) => `L ${(i + 1) * sx} ${-value * sy}`),
			`L ${sx * (pts.length - 1)} 0`,
			"Z",
		].join(" ");
	});

	const gridLines = createMemo(() => {
		let previous: string | null = null;
		return [0, -25, -50, -75, -100].map((y) => {
			const label = formatValue(maxHeight() * (-y / 100), 0);
			// skip labels that would just repeat the previous grid line
			const showLabel = label !== previous;
			previous = label;
			return { y, label, showLabel };
		});
	});

	const ticks = createMemo<Array<Tick>>(() => {
		const pts = props.points;
		if (pts.length === 0) return [];

		const count = Math.min(6, pts.length);
		const sx = scaleX();
		const result: Array<Tick> = [];

		for (let i = 0; i < count; i++) {
			const index =
				i === 0
					? 0
					: i === count - 1
						? pts.length - 1
						: Math.floor((i * (pts.length - 1)) / (count - 1));
			const label = props.formatIndex?.(index) ?? `${index}`;
			// skip labels that repeat the previous tick (eg. two ticks in one day)
			if (result[result.length - 1]?.label === label) continue;

			result.push({
				x: index * sx,
				label,
				anchor: i === 0 ? "start" : i === count - 1 ? "end" : "middle",
			});
		}

		return result;
	});

	/** pointer position -> data point index, accounting for the svg's letterboxing */
	const getIndexFromPointer = (e: PointerEvent | MouseEvent) => {
		if (!svgRef || props.points.length === 0) return null;
		const rect = svgRef.getBoundingClientRect();
		const scale = Math.min(
			rect.width / VIEWBOX_WIDTH,
			rect.height / VIEWBOX_HEIGHT,
		);
		const plotStart = (rect.width - VIEWBOX_WIDTH * scale) / 2 + GUTTER * scale;
		const plotWidth = PLOT * scale;
		const ratio = (e.clientX - rect.left - plotStart) / plotWidth;
		const index = Math.round(ratio * (props.points.length - 1));
		return Math.max(0, Math.min(props.points.length - 1, index));
	};

	const onPointerMove = (e: PointerEvent) => {
		const index = getIndexFromPointer(e);
		if (index !== null) setHoveredIndex(index);
	};

	const onPointerLeave = () => {
		setIsPointerInside(false);
		if (selectionStartIndex() === null) setHoveredIndex(null);
	};

	const onPointerEnter = () => {
		setIsPointerInside(true);
	};

	const onPointerDown = (e: PointerEvent) => {
		const index = getIndexFromPointer(e);
		if (index === null) return;
		svgRef?.setPointerCapture(e.pointerId);
		setSelectionStartIndex(index);
		setIsPointerInside(true);
	};

	const onPointerUp = (e: PointerEvent) => {
		if (svgRef?.hasPointerCapture(e.pointerId)) {
			svgRef.releasePointerCapture(e.pointerId);
		}

		const end = getIndexFromPointer(e);
		const start = selectionStartIndex();
		if (end === null || start === null) return;

		if (context) context[1].triggerZoom(end);
		else setLocalSelectionStartIndex(null);
	};

	const selection = createMemo(() => {
		const start = selectionStartIndex();
		const end = hoveredIndex();
		if (start === null || end === null) return null;
		return { from: Math.min(start, end), width: Math.abs(start - end) };
	});

	// the shared crosshair index may be out of range for series with fewer points
	const markerIndex = createMemo(() => {
		const index = hoveredIndex();
		if (index === null || index >= props.points.length) return null;
		return index;
	});

	const tooltip = createMemo(() => {
		const index = markerIndex();
		if (index === null || !props.formatTooltip) return null;
		const count = props.points.length;
		const ratio = count > 1 ? index / (count - 1) : 0;
		const left = ((GUTTER + ratio * PLOT) / VIEWBOX_WIDTH) * 100;
		// keep the tooltip fully inside the chart
		return {
			left: Math.min(94, Math.max(6, left)),
			...props.formatTooltip(index),
		};
	});

	const gradientId = createUniqueId();
	const stroke = (alpha = 1) => `oklch(var(--color-link-500) / ${alpha})`;

	return (
		<div class="chart-container">
			<Show
				when={props.points.length > 0}
				fallback={
					<div class="chart-empty">
						<span>{props.emptyMessage ?? "No data available"}</span>
						<Show when={props.onEmptyAction}>
							<button
								type="button"
								class="button"
								onClick={() => props.onEmptyAction?.()}
							>
								{props.emptyActionLabel ?? "Widen range"}
							</button>
						</Show>
					</div>
				}
			>
				<div class="chart">
					<svg
						ref={svgRef}
						viewBox={VIEWBOX}
						role="img"
						aria-label={props.ariaLabel}
						onPointerEnter={onPointerEnter}
						onPointerMove={onPointerMove}
						onPointerLeave={onPointerLeave}
						onPointerDown={onPointerDown}
						onPointerUp={onPointerUp}
					>
						<title>{props.ariaLabel}</title>
						<defs>
							<linearGradient id={gradientId} x1="0" x2="0" y1="0" y2="1">
								<stop offset="0%" stop-color={stroke(0.4)} />
								<stop offset="100%" stop-color={stroke(0.05)} />
							</linearGradient>
						</defs>

						<For each={gridLines()}>
							{(grid) => (
								<>
									<line
										x1="0"
										x2={PLOT}
										y1={grid.y}
										y2={grid.y}
										stroke="oklch(var(--color-sep-400))"
										stroke-width="1"
										stroke-dasharray={grid.y === 0 ? undefined : "4 4"}
									/>
									<Show when={grid.showLabel}>
										<text
											x="-8"
											y={grid.y - 3}
											text-anchor="end"
											fill="oklch(var(--color-fg5))"
											font-size="10"
										>
											{grid.label}
										</text>
									</Show>
								</>
							)}
						</For>

						<path d={area()} fill={`url(#${gradientId})`} />
						<path d={line()} fill="none" stroke={stroke()} stroke-width="2" />

						<Show when={selection()} keyed>
							{(selection) => (
								<rect
									x={selection.from * scaleX()}
									y="-100"
									width={selection.width * scaleX()}
									height="100"
									fill={stroke(0.3)}
								/>
							)}
						</Show>

						<Show when={markerIndex()} keyed>
							{(index) => (
								<>
									<line
										x1={index * scaleX()}
										x2={index * scaleX()}
										y1="0"
										y2="-100"
										stroke={stroke(0.6)}
										stroke-width="1"
									/>
									<circle
										cx={index * scaleX()}
										cy={-(props.points[index] ?? 0) * scaleY()}
										r="3.5"
										fill={stroke()}
									/>
								</>
							)}
						</Show>

						<For each={ticks()}>
							{(tick) => (
								<text
									x={tick.x}
									y="15"
									fill="oklch(var(--color-fg5))"
									font-size="10"
									text-anchor={tick.anchor}
								>
									{tick.label}
								</text>
							)}
						</For>
					</svg>

					<Show when={isPointerInside() && tooltip()}>
						{(tip) => (
							<div class="chart-tooltip" style={{ left: `${tip().left}%` }}>
								<span class="date">{tip().label}</span>
								<span class="value">{tip().value}</span>
							</div>
						)}
					</Show>
				</div>
			</Show>
		</div>
	);
};
