import { z } from "npm:@hono/zod-openapi";

export const Permission = z.enum([
	"View",
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
]).openapi("Permission");
