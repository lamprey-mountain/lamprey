// import { z, OpenAPIHono } from "@hono/zod-openapi";
// import { RoomCreate, RoomGet, RoomList, RoomUpdate, DmInitialize, DmGet, RoomAck } from "./routes/rooms.ts";
// import { ThreadAck, ThreadBulkUpdate, ThreadCreate, ThreadGet, ThreadList, ThreadUpdate } from "./routes/threads.ts";
// import { MessageAck, MessageCreate, MessageDelete, MessageList, MessageUpdate, MessageVersionsDelete, MessageVersionsGet, MessageVersionsList } from "./routes/messages.ts";
// import { UserCreate, UserDelete, UserGet, UserUpdate } from "./routes/users.ts";
// import { SessionCreate, SessionDelete, SessionList, SessionGet, SessionUpdate } from "./routes/sessions.ts";
// import { InviteCreate, InviteDelete, InviteResolve, InviteRoomList, InviteUse } from "./routes/invite.ts";
// import { RoleCreate, RoleDelete, RoleGet, RoleList, RoleUpdate } from "./routes/roles.ts";
// import { MemberGet, MemberKick, MemberList, MemberRoleApply, MemberRoleRemove, MemberUpdate } from "./routes/members.ts";
// import { BanCreate, BanDelete } from "./routes/bans.ts";
// import { SearchMessages, SearchRooms, SearchThreads } from "./routes/search.ts";
// import { MediaClone, MediaCreate } from "./routes/media.ts";
// import { SyncInit } from "./routes/sync.ts";
// import { ServerInfo } from "./routes/core.ts";
// // import { LogList, ReportCreateMedia, ReportCreateMessage, ReportCreateUser, ReportCreateRoom, ReportCreateThread, ReportCreateMember } from "./routes/moderation.ts";
// import { withAuth } from "./auth.ts";
// import { queries as q, db, Permissions, HonoEnv, SessionStatus } from "./data.ts";
// import { uuidv7 } from "uuidv7";
// import { upgradeWebSocket } from "npm:hono/deno";
// import { Session, Room, Thread, User } from "./types.ts";
// import { MessageFromDb, ThreadFromDb, UserFromDb } from "./types/db.ts";
// import { MessageClient, MessageServer } from "./types/sync.ts";
// import { AuthPasswordSet, AuthTotpSet, AuthPasswordDo, AuthTotpDo, AuthDiscordStart, AuthDiscordFinish } from "./routes/auth.ts";
// import * as bcrypt from "bcrypt";
// import * as otpauth from "otpauth";
// import * as discord from "./oauth2.ts";

// const UUID_MIN = "00000000-0000-0000-0000-000000000000";
// const UUID_MAX = "ffffffff-ffff-ffff-ffff-ffffffffffff";

// const sushi = new Set<(msg: z.infer<typeof MessageServer>) => void>();

// function broadcast(msg: z.infer<typeof MessageServer>) {
//   for (const listener of sushi) listener(msg);
// }

// export function setup(app: OpenAPIHono<HonoEnv>) {
//   // app.openapi(InviteCreate, async (c) => { throw "todo" });
//   // app.openapi(InviteResolve, async (c) => { throw "todo" });
//   // app.openapi(InviteUse, async (c) => { throw "todo" });
//   // app.openapi(InviteRoomList, async (c) => { throw "todo" });
//   // app.openapi(InviteDelete, async (c) => { throw "todo" });
//   // app.openapi(RoleCreate, async (c) => { throw "todo" });
//   // app.openapi(RoleList, async (c) => { throw "todo" });
//   // app.openapi(RoleGet, async (c) => { throw "todo" });
//   // app.openapi(RoleUpdate, async (c) => { throw "todo" });
//   // app.openapi(RoleDelete, async (c) => { throw "todo" });
//   // app.openapi(MemberList, async (c) => { throw "todo" });
//   // app.openapi(MemberGet, async (c) => { throw "todo" });
//   // app.openapi(MemberKick, async (c) => { throw "todo" });
//   // app.openapi(MemberUpdate, async (c) => { throw "todo" });
//   // app.openapi(MemberRoleApply, async (c) => { throw "todo" });
//   // app.openapi(MemberRoleRemove, async (c) => { throw "todo" });
//   // app.openapi(BanCreate, async (c) => { throw "todo" });
//   // app.openapi(BanDelete, async (c) => { throw "todo" });
//   // app.openapi(DmInitialize, async (c) => { throw "todo" });
//   // app.openapi(DmGet, async (c) => { throw "todo" });
//   // app.openapi(SearchMessages, async (c) => { throw "todo" });
//   // app.openapi(SearchThreads, async (c) => { throw "todo" });
//   // app.openapi(SearchRooms, async (c) => { throw "todo" });
//   // app.openapi(ThreadAck, async (c) => { throw "todo" });
//   // app.openapi(MessageAck, async (c) => { throw "todo" });
//   // app.openapi(RoomAck, async (c) => { throw "todo" });
//   // app.openapi(MediaCreate, async (c) => { throw "todo" });
//   // app.openapi(MediaClone, async (c) => { throw "todo" });

//   // app.openapi(ServerInfo, async (c) => { throw "todo"; });
// }
