import { Context, Next } from "npm:hono";
import { data, Permissions, HonoEnv, MemberT, MessageT, RoomT, ThreadT, UserT } from "globals";
import { RouteConfig, z } from "npm:@hono/zod-openapi";
import { Permission } from "../types.ts";

type AuthOptions = {
	strict: boolean;
};

export function withAuth<T extends RouteConfig>(
	route: T,
	opts: AuthOptions = { strict: true },
) {
	const m = route.middleware;
	const middleware = [...Array.isArray(m) ? m : m ? [m] : [], auth(opts)];
	return { ...route, middleware } as T;
}

// how much can/should i push this down into sql?
export const auth =
	(opts: AuthOptions) => async (c: Context<HonoEnv>, next: Next) => {
		// verify session
		const authToken = c.req.header("authorization");
		if (!authToken) return c.json({ error: "Missing authorization token" }, 401);
		const session = await data.sessionSelectByToken(authToken);
		if (!session) return c.json({ error: "Invalid or expired token" }, 401);
		const { user_id } = session;
		c.set("user_id", user_id);
		c.set("session_id", session.id);

		const room_id = c.req.param("room_id");
		const thread_id = c.req.param("thread_id");
		let perms;
		if (thread_id) {
			perms = await data.permissionReadThread(user_id, thread_id);
		} else if (room_id) {
			perms = await data.permissionReadRoom(user_id, room_id);
		} else {
			perms = Permissions.none;
		}
		
		// console.log(r.permissions)
		c.set("permissions", perms);
		await next();
	};

/*
extra permissions

- room member: View
- thread creator: ThreadManage
- message creator: MessageEdit, MessageDelete
*/
