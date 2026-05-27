// TODO: use this for VoiceDebug component
// TODO: use this for RoomAnalytics component
// TODO: fix tooltips

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

export type ChartsProps = {
	/** start data points index, end index */
	onZoom?: (start: number, end: number) => void;

	/** extract additional hover info (eg. time) */
	extractHoverInfo?: (index: number) => string;

	formatYAxisLabel?: (index: number) => string;
};

export type ChartsContextData = [ChartsState, ChartsActions];

export type ChartsState = {
	hoveredIndex: number | null;
	selectionStartIndex: number | null;
	extractHoverInfo?: (index: number) => string;
	formatYAxisLabel?: (index: number) => string;
};

export type ChartsActions = {
	setHoveredIndex: (index: number | null) => void;
	setSelectionStartIndex: (index: number | null) => void;
	triggerZoom: (endIndex: number) => void;
	resetZoom: () => void;
};

export type ChartProps = {
	points: Array<number>;
	height?: number;
	unit?: string;
	format?: (value: number, index: number) => string;
};

const ChartsContext = createContext<ChartsContextData>();

const useCharts = () => useContext(ChartsContext);

export const Charts = (props: ParentProps<ChartsProps>) => {
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
		get extractHoverInfo() {
			return props.extractHoverInfo;
		},
		get formatYAxisLabel() {
			return props.formatYAxisLabel;
		},
	};

	const actions: ChartsActions = {
		setHoveredIndex,
		setSelectionStartIndex,
		triggerZoom(endIndex: number) {
			const start = selectionStartIndex();
			if (start !== null && start !== endIndex && props.onZoom) {
				props.onZoom(Math.min(start, endIndex), Math.max(start, endIndex));
			}
			setSelectionStartIndex(null);
		},
		resetZoom() {
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

export const Chart = (props: VoidProps<ChartProps>) => {
	const context = useCharts();

	// fallback state if the chart is rendered outside of a <Charts> parent wrapper
	const [localHoveredIndex, setLocalHoveredIndex] = createSignal<number | null>(
		null,
	);
	const [localSelectionStartIndex, setLocalSelectionStartIndex] = createSignal<
		number | null
	>(null);

	// getters to abstract whether we use context or local state
	const hoveredIndex = () => {
		return context ? context[0].hoveredIndex : localHoveredIndex();
	};

	const selectionStartIndex = () => {
		return context
			? context[0].selectionStartIndex
			: localSelectionStartIndex();
	};

	const setHoveredIndex = (val: number | null) => {
		if (context) {
			context[1].setHoveredIndex(val);
		} else {
			setLocalHoveredIndex(val);
		}
	};

	const setSelectionStartIndex = (val: number | null) => {
		if (context) {
			context[1].setSelectionStartIndex(val);
		} else {
			setLocalSelectionStartIndex(val);
		}
	};

	let svgRef: SVGSVGElement | undefined;

	const maxHeight = createMemo(() => {
		if (props.height !== undefined && props.height > 0) {
			return props.height;
		}
		const pts = props.points;
		return pts.length > 0 ? Math.max(...pts, 1) : 1;
	});

	// Coordinate mapping to a 600x100 virtual viewbox
	const scaleX = createMemo(
		() => 600 / (props.points.length > 1 ? props.points.length - 1 : 1),
	);
	const scaleY = createMemo(() => 100 / maxHeight());

	// generate svg paths
	const pathStroke = createMemo(() => {
		const pts = props.points;
		if (pts.length === 0) return "";
		const sx = scaleX();
		const sy = scaleY();
		return [
			`M 0 ${-pts[0] * sy}`,
			...pts.slice(1).map((d, i) => `L ${(i + 1) * sx} ${-d * sy}`),
		].join(" ");
	});

	const pathFill = createMemo(() => {
		const pts = props.points;
		if (pts.length === 0) return "";
		const sx = scaleX();
		const sy = scaleY();
		return [
			`M 0 0`,
			`L 0 ${-pts[0] * sy}`,
			...pts.slice(1).map((d, i) => `L ${(i + 1) * sx} ${-d * sy}`),
			`L ${sx * (pts.length - 1)} 0`,
		].join(" ");
	});

	// generate tick marks
	const ticks = createMemo(() => {
		const pts = props.points;
		if (pts.length === 0) return [];

		const sx = scaleX();
		const count = Math.min(6, pts.length);
		const result = [];

		for (let i = 0; i < count; i++) {
			let index: number;
			if (i === 0) {
				index = 0;
			} else if (i === count - 1) {
				index = pts.length - 1;
			} else {
				index = Math.floor((i * (pts.length - 1)) / (count - 1));
			}

			// render hover info custom format or a default numeric marker
			const label = context?.[0].extractHoverInfo
				? context[0].formatYAxisLabel?.(index)
				: `${index}`;

			result.push({
				x: index * sx,
				label,
				anchor:
					i === 0 ? "start" : i === count - 1 ? "end" : ("middle" as const),
			});
		}
		return result;
	});

	// translate mouse events to series indexes
	const getIndexFromMouseEvent = (e: MouseEvent) => {
		const pts = props.points;
		if (!svgRef || pts.length === 0) return null;
		const rect = svgRef.getBoundingClientRect();
		const x = e.clientX - rect.left;
		const ratio = x / rect.width;
		const index = Math.round(ratio * (pts.length - 1));
		return Math.max(0, Math.min(pts.length - 1, index));
	};

	const onMouseMove = (e: MouseEvent) => {
		const index = getIndexFromMouseEvent(e);
		if (index !== null) {
			setHoveredIndex(index);
		}
	};

	const onMouseLeave = () => {
		setHoveredIndex(null);
	};

	const onMouseDown = (e: MouseEvent) => {
		const index = getIndexFromMouseEvent(e);
		if (index !== null) {
			setSelectionStartIndex(index);
		}
	};

	const onMouseUp = (e: MouseEvent) => {
		const index = getIndexFromMouseEvent(e);
		const start = selectionStartIndex();

		if (index !== null && start !== null) {
			if (context) {
				context[1].triggerZoom(index);
			} else {
				setLocalSelectionStartIndex(null);
			}
		}
	};

	// theme
	const gradientId = createUniqueId();
	const strokeColor = (alpha = 1) => `oklch(var(--color-link-500) / ${alpha})`;
	const gridColor = () => "oklch(var(--color-sep-400))";

	return (
		<div class="chart-container">
			<div class="chart">
				<Show when={props.points.length > 0} fallback="No data available">
					<svg
						aria-hidden="true"
						ref={svgRef}
						viewBox="0 -105 600 120"
						style="width: 100%; overflow: visible;"
						onMouseMove={onMouseMove}
						onMouseLeave={onMouseLeave}
						onMouseDown={onMouseDown}
						onMouseUp={onMouseUp}
					>
						<defs>
							<linearGradient id={gradientId} x1="0" x2="0" y1="0" y2="1">
								<stop offset="0%" stop-color={strokeColor(0.4)} />
								<stop offset="100%" stop-color={strokeColor(0.05)} />
							</linearGradient>
						</defs>

						{/* grid & y axis labels */}
						<For each={[-25, -50, -75, -100]}>
							{(y) => {
								const value = createMemo(() => maxHeight() * (-y / 100));
								return (
									<>
										<line
											x1="0"
											x2="600"
											y1={y}
											y2={y}
											stroke={gridColor()}
											stroke-width="1"
											stroke-dasharray="4 4"
										/>
										<text
											x="0"
											y={y - 2}
											fill="oklch(var(--color-fg5, #aaa))"
											font-size="10"
										>
											{props.format
												? props.format(value(), y)
												: `${value().toFixed(1)}${props.unit ? " " + props.unit : ""}`}
										</text>
									</>
								);
							}}
						</For>

						{/* chart paths */}
						<path
							d={pathStroke()}
							fill="none"
							stroke={strokeColor()}
							stroke-width="2"
						/>
						<path d={pathFill()} fill={`url(#${gradientId})`} />

						{/* drag selection overlay */}
						<Show
							when={selectionStartIndex() !== null && hoveredIndex() !== null}
						>
							<rect
								x={Math.min(selectionStartIndex()!, hoveredIndex()!) * scaleX()}
								y="-100"
								width={
									Math.abs(selectionStartIndex()! - hoveredIndex()!) * scaleX()
								}
								height="100"
								fill="oklch(var(--color-link-500, #08f) / 0.3)"
							/>
						</Show>

						{/* active hover marker */}
						<Show when={hoveredIndex() !== null}>
							<line
								x1={hoveredIndex()! * scaleX()}
								x2={hoveredIndex()! * scaleX()}
								y1="0"
								y2="-100"
								stroke="oklch(var(--color-link-500, #08f) / 0.6)"
								stroke-width="1"
							/>
							<circle
								cx={hoveredIndex()! * scaleX()}
								cy={-props.points[hoveredIndex()!] * scaleY()}
								r="4"
								fill={strokeColor()}
							/>
						</Show>

						{/* x axis labels */}
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
				</Show>
			</div>
		</div>
	);
};
