import type { types } from "sdk";

export type RoomT = types.Room;
export type ThreadT = types.Channel;
export type MessageT = types.Message;
export type UserT = types.User;
export type RoleT = types.Role;
export type MemberT = types.RoomMember;
export type MediaT = types.Media;
export type SessionT = types.Session;

export type InviteT = {
	code: string;
};

export type Pagination<T> = {
	total: number;
	items: Array<T>;
	has_more: boolean;
};

export enum MessageType {
	DefaultMarkdown = "DefaultMarkdown",
	DefaultTagged = "DefaultTagged",
	ThreadUpdate = "ThreadUpdate",
	MessagesMoved = "MessagesMoved",
	Call = "Call",
	ChannelPingback = "ChannelPingback",
	ChannelMoved = "ChannelMoved",
	ChannelIcon = "ChannelIcon",
	ThreadCreated = "ThreadCreated",
	AutomodExecution = "AutomodExecution",
}
