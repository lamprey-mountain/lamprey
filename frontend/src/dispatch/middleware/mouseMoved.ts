import type { Middleware, WindowAction } from "../types";
import { batch as solidBatch } from "solid-js";

export const mouseMoved: Middleware = (
	ctx,
	api,
	update,
) =>
(next) =>
(action) => {
	if (action.do === "window.mouse_move") {
		const { e } = action as WindowAction;
		// TODO: use triangle to submenu corners instead of dot with x axis
		const pos = [
			...ctx.data.cursor.pos,
			[e.x, e.y] as [number, number],
		];
		if (pos.length > 5) pos.shift();
		let vx = 0, vy = 0;
		for (let i = 1; i < pos.length; i++) {
			vx += pos[i - 1][0] - pos[i][0];
			vy += pos[i - 1][1] - pos[i][1];
		}
		solidBatch(() => {
			update("cursor", "pos", pos);
			update("cursor", "vel", (vx / Math.hypot(vx, vy)) || 0);
		});
	} else {
		next(action);
	}
};
