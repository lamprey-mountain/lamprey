import { createEffect, onCleanup } from "solid-js";
import { batch as solidBatch } from "solid-js";
import { SetStoreFunction } from "solid-js/store";
import { Data } from "../context";

export function useMouseTracking(update: SetStoreFunction<Data>) {
	let lastUpdateTime = 0;
	const THROTTLE_INTERVAL = 50; // milliseconds

	const handleMouseMove = (e: MouseEvent) => {
		const now = Date.now();
		if (now - lastUpdateTime < THROTTLE_INTERVAL) {
			return;
		}
		lastUpdateTime = now;

		update((s) => {
			const pos = [...s.cursor.pos, [e.x, e.y] as [number, number]];
			if (pos.length > 5) pos.shift();

			let vx = 0,
				vy = 0;
			for (let i = 1; i < pos.length; i++) {
				vx += pos[i - 1][0] - pos[i][0];
				vy += pos[i - 1][1] - pos[i][1];
			}

			return {
				...s,
				cursor: {
					...s.cursor,
					pos: pos,
					vel: vx / Math.hypot(vx, vy) || 0,
				},
			};
		});
	};

	createEffect(() => {
		window.addEventListener("mousemove", handleMouseMove);
		onCleanup(() => {
			window.removeEventListener("mousemove", handleMouseMove);
		});
	});
}
