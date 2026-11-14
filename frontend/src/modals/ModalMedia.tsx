import type { Media } from "sdk";
import {
	createEffect,
	createMemo,
	createSignal,
	on,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import { useModals } from "@/contexts/modal";
import { getHeight, getUrl, getWidth, Loader, Resize } from "@/media/util";

const MIN_SCALE = 1;
const MAX_SCALE = 8;
// keep at least this much of the image visible while panning
const PAN_MARGIN = 24;

// currently only supports images!
// though, it doesn't make much sense for video/audio/other media?
export const ModalMedia = (props: { media: Media; gallery?: Media[] }) => {
	const [modals, modalctl] = useModals();
	const [loaded, setLoaded] = createSignal(false);
	const [scale, setScale] = createSignal(1);
	const [x, setX] = createSignal(0);
	const [y, setY] = createSignal(0);
	const [dragging, setDragging] = createSignal(false);
	const [counterVisible, setCounterVisible] = createSignal(false);

	const gallery = createMemo(() => {
		const list = props.gallery?.length ? props.gallery : [props.media];
		return list.some((m) => m.id === props.media.id)
			? list
			: [props.media, ...list];
	});

	const [index, setIndex] = createSignal(
		Math.max(
			0,
			gallery().findIndex((m) => m.id === props.media.id),
		),
	);

	const current = () => gallery()[index()] ?? props.media;
	const height = () => getHeight(current());
	const width = () => getWidth(current());

	let viewport: HTMLDivElement | undefined;
	let zoomContent: HTMLDivElement | undefined;
	let counterTimer: number | undefined;
	let dragStart: { x: number; y: number; tx: number; ty: number } | null = null;
	let dragMoved = false;
	let startedOnImage = false;

	const clampOffset = (value: number, axis: "x" | "y", nextScale: number) => {
		if (!viewport || !zoomContent) return 0;
		const contentSize =
			axis === "x" ? zoomContent.offsetWidth : zoomContent.offsetHeight;
		const viewportSize =
			axis === "x" ? viewport.clientWidth : viewport.clientHeight;
		const max = Math.max(
			0,
			(viewportSize + contentSize * nextScale) / 2 - PAN_MARGIN,
		);
		return Math.max(-max, Math.min(max, value));
	};

	const reset = () => {
		setScale(1);
		setX(0);
		setY(0);
	};

	const zoomAt = (clientX: number, clientY: number, nextScale: number) => {
		if (!viewport) return;
		const prev = scale();
		const next = Math.max(MIN_SCALE, Math.min(MAX_SCALE, nextScale));
		if (next === prev) return;

		const rect = viewport.getBoundingClientRect();
		const px = clientX - (rect.left + rect.width / 2);
		const py = clientY - (rect.top + rect.height / 2);
		// keep the point under the cursor fixed while scaling
		const nx = px - ((px - x()) * next) / prev;
		const ny = py - ((py - y()) * next) / prev;

		setScale(next);
		setX(clampOffset(nx, "x", next));
		setY(clampOffset(ny, "y", next));
	};

	const onWheel = (e: WheelEvent) => {
		e.preventDefault();
		zoomAt(e.clientX, e.clientY, scale() * Math.exp(-e.deltaY * 0.0015));
	};

	const onPointerDown = (e: PointerEvent) => {
		if (e.button !== 0) return;
		e.preventDefault();
		if (e.currentTarget instanceof HTMLElement) {
			e.currentTarget.setPointerCapture(e.pointerId);
		}
		dragStart = { x: e.clientX, y: e.clientY, tx: x(), ty: y() };
		dragMoved = false;
		startedOnImage = zoomContent?.contains(e.target as Node) ?? false;
		setDragging(true);
	};

	const onPointerMove = (e: PointerEvent) => {
		if (!dragStart) return;
		const dx = e.clientX - dragStart.x;
		const dy = e.clientY - dragStart.y;
		if (Math.abs(dx) > 3 || Math.abs(dy) > 3) dragMoved = true;
		const s = scale();
		setX(clampOffset(dragStart.tx + dx, "x", s));
		setY(clampOffset(dragStart.ty + dy, "y", s));
	};

	const endDrag = (e: PointerEvent) => {
		const wasClick =
			e.type === "pointerup" && !dragMoved && !startedOnImage;
		dragStart = null;
		setDragging(false);
		if (
			e.currentTarget instanceof HTMLElement &&
			e.currentTarget.hasPointerCapture(e.pointerId)
		) {
			e.currentTarget.releasePointerCapture(e.pointerId);
		}
		if (wasClick) modalctl.close();
	};

	const flashCounter = () => {
		if (gallery().length < 2) return;
		setCounterVisible(true);
		if (counterTimer !== undefined) clearTimeout(counterTimer);
		counterTimer = window.setTimeout(() => setCounterVisible(false), 1000);
	};

	const go = (delta: number) => {
		const list = gallery();
		if (list.length < 2) return;
		setIndex((i) => (i + delta + list.length) % list.length);
		flashCounter();
	};

	const onKeyDown = (e: KeyboardEvent) => {
		if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey) return;
		if (modals.at(-1)?.type !== "media") return;
		if (gallery().length < 2) return;
		if (e.key === "ArrowLeft") {
			e.preventDefault();
			go(-1);
		} else if (e.key === "ArrowRight") {
			e.preventDefault();
			go(1);
		}
	};

	onMount(() => {
		document.addEventListener("keydown", onKeyDown);
		viewport?.addEventListener("wheel", onWheel, { passive: false });
		flashCounter();
	});

	onCleanup(() => {
		document.removeEventListener("keydown", onKeyDown);
		viewport?.removeEventListener("wheel", onWheel);
		if (counterTimer !== undefined) clearTimeout(counterTimer);
	});

	createEffect(
		on(
			() => current().id,
			() => {
				setLoaded(false);
				reset();
			},
		),
	);

	return (
		<div class="modal modal-media">
			<div class="bg" onClick={modalctl.close}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog" aria-modal>
					<div
						ref={viewport}
						class="viewport"
						classList={{ dragging: dragging() }}
						onPointerDown={onPointerDown}
						onPointerMove={onPointerMove}
						onPointerUp={endDrag}
						onPointerCancel={endDrag}
					>
						<div
							ref={zoomContent}
							class="zoom"
							style={{
								transform: `translate(${x()}px, ${y()}px) scale(${scale()})`,
							}}
							onDblClick={reset}
						>
							<Resize height={height()} width={width()}>
								<div class="media image full">
									<Loader loaded={loaded()} />
									<img
										src={getUrl(current())}
										alt={current().alt ?? undefined}
										height={height()!}
										width={width()!}
										draggable={false}
										onLoad={() => setLoaded(true)}
										onEmptied={() => setLoaded(false)}
									/>
								</div>
							</Resize>
						</div>
					</div>
					<Show when={gallery().length > 1}>
						<div class="counter" classList={{ visible: counterVisible() }}>
							{index() + 1} / {gallery().length}
						</div>
					</Show>
				</div>
			</div>
		</div>
	);
};
