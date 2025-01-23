import { types } from "sdk";

export type RoomT = types.Room;
export type ThreadT = types.Thread;
export type MessageT = types.Message;
export type UserT = types.User;
export type RoleT = types.Role;
export type MemberT = types.RoomMember;
export type MediaT = types.Media;

export type InviteT = {
	code: string;
};

export type Pagination<T> = {
	total: number;
	items: Array<T>;
	has_more: boolean;
};

export enum MessageType {
	Default = "Default",
	ThreadUpdate = "ThreadUpdate",
}
