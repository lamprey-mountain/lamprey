import { z } from "npm:@hono/zod-openapi";

export const PermissionAssignable = z.enum([
	"Admin",
	"RoomManage",
	"ThreadCreate",
	"ThreadManage",
	"ThreadDelete",
	"MessageCreate",
	"MessageFilesEmbeds",
	"MessagePin",
	"MessageDelete",
	"MessageMassMention",
	"MemberKick",
	"MemberBan",
	"MemberManage",
	"InviteCreate",
	"InviteManage",
	"RoleManage",
	"RoleApply",
]).describe("permissions that can be assigned through a role").openapi("PermissionAssignable");

export const PermissionSystem = z.enum([
	"View",
	"MessageEdit",
]).describe("permissions calculated by the system that cannot be overridden").openapi("PermissionSystem");

export const Permission = z.union([PermissionAssignable, PermissionSystem])
	.openapi("Permission");
