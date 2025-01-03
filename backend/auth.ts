import { Context, Next } from "npm:hono";
import { db, HonoEnv, Permissions, sql } from "./data.ts";
import { RouteConfig } from "npm:@hono/zod-openapi";

type AuthOptions = {
	strict: boolean;
};

export const auth =
	(opts: AuthOptions) => async (c: Context<HonoEnv>, next: Next) => {
		const auth = c.req.header("authorization");
		if (!auth) return c.json({ error: "Missing authorization token" }, 401);
		const q = await db.connect();
		const { rows: [row] }= await q.queryObject`SELECT * FROM sessions WHERE token = ${auth}`;
		if (!row) return c.json({ error: "Invalid or expired token" }, 401);
		if (opts.strict && row.level as number < 1) {
			return c.json({ error: "Unauthorized" }, 403);
		}
		c.set("user_id", row.user_id as string);
		c.set("session_id", row.session_id as string);
		c.set("session_level", row.level as number);
		await next();
	};

export function withAuth<T extends RouteConfig>(
	route: T,
	opts: AuthOptions = { strict: true },
) {
	const m = route.middleware;
	const middleware = [...Array.isArray(m) ? m : m ? [m] : [], auth(opts)];
	return { ...route, middleware } as T;
}
