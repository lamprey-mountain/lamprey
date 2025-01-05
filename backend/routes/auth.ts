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

		// resolve referenced objects and calculate permissions
		const r = await fetchDataAndPermissions({
			message_id: c.req.param("message_id"),
			room_id: c.req.param("room_id"),
			thread_id: c.req.param("thread_id"),
			user_id: c.req.param("user_id"),
			user_id_self: user_id,
		});
		if (r.room) c.set("room", r.room);
		if (r.thread) c.set("thread", r.thread);
		if (r.message) c.set("message", r.message);
		if (r.member) c.set("member", r.member);
		if (r.member_self) c.set("member_self", r.member_self);
		if (r.user) c.set("user", r.user);
		if (r.user_self) c.set("user_self", r.user_self);
		
		console.log(r.permissions)
		c.set("permissions", r.permissions);
		await next();
	};

type FetchDataRequest = {
	user_id_self?: string,
	user_id?: string,
	room_id?: string,
	thread_id?: string,
	message_id?: string,
}

type FetchDataResponse = {
	permissions: Permissions,
	user_self?: UserT,
	user?: UserT,
	member_self?: MemberT,
	member?: MemberT,
	room?: RoomT,
	thread?: ThreadT,
	message?: MessageT,
}

export async function fetchDataAndPermissions(d: FetchDataRequest): Promise<FetchDataResponse> {
	const u = undefined;
	const r: FetchDataResponse = { permissions: Permissions.none };
	if (d.thread_id) {
		r.thread = await data.threadSelect(d.thread_id) ?? u;
		if (d.message_id) r.message = await data.messageSelect(d.thread_id, d.message_id) ?? u;
	}
	if (d.room_id || r.thread) {
		r.room = await data.roomSelect(d.room_id ?? r.thread!.room_id) ?? u;
	}
	if (d.user_id_self) {
		if (r.room) {
			r.member_self = await data.memberSelect(r.room.id, d.user_id_self) ?? u;
			r.user_self = r.member_self?.user;
		} else {
			r.user_self = await data.userSelect(d.user_id_self) ?? u;
		}
	}
	if (d.user_id) {
		if (r.room) {
			r.member = await data.memberSelect(r.room.id, d.user_id) ?? u;
			r.user = r.member?.user;
		} else {
			r.user = await data.userSelect(d.user_id) ?? u;
		}
	}
	if (r.member_self) {
		r.permissions = new Permissions(r.member_self.roles.flatMap(i => i.permissions));
		r.permissions.add("View");
		if (r.thread?.creator_id === r.member_self.user.id) {
			r.permissions.add("ThreadManage");
		}
		if (r.message?.author.id === r.member_self.user.id) {
			r.permissions.add("MessageEdit");
			r.permissions.add("MessageDelete");
		}
	}
	// console.log(r)
	return r;
}
