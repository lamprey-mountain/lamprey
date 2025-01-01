import { OpenAPIHono } from "@hono/zod-openapi";
import { HonoEnv } from "../data.ts";
import setupRooms from "./impl/rooms.ts";

export function setup(app: OpenAPIHono<HonoEnv>) {
	setupRooms(app);
}
