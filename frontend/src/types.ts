export type RoomT = {
	id: string,
	name: string,
	description: string | null,
}

export type ThreadT = {
	id: string,
	room_id: string,
	creator_id: string,
	name: string,
	description: string | null,
	is_closed: boolean,
	is_locked: boolean,
	is_pinned: boolean,
	is_unread: boolean,
	last_version_id: string,
	last_read_id: string,
	message_count: number,
}

export type MessageT = {
	type: MessageType,
	id: string,
	thread_id: string,
	version_id: string,
	override_name: string | null,
	reply_id: string | null,
	nonce: string | null,
	content: string | null,
	author: UserT,
	metadata: any,
	attachments: Array<AttachmentT>,
}

export type AttachmentT = {
	id: string,
	filename: string,
	url: string,
	source_url: string | null,
	thumbnail_url: string | null,
	mime: string,
	alt: string | null,
	size: number,
	height: number | null,
	width: number | null,
	duration: number | null,
}

export type UserT = {
	id: string,
	name: string,
	description: string | null,
	status: string | null,
	is_bot: boolean,
	is_alias: boolean,
	is_system: boolean,
}

export type RoleT = {
	id: string,
	room_id: string,
	name: string,
	description: string | null,
	permissions: Array<string>,
}

export type MemberT = {
	user: UserT,
	room_id: string,
	membership: "join" | "ban",
	override_name: string | null,
	override_description: string | null,
	roles: Array<RoleT>,
}

export type InviteT = {
	code: string,
}

export type Pagination<T> = {
	count: number,
	items: Array<T>,
	has_more: boolean,
}

export enum MessageType {
	Default = 0,
	ThreadUpdate = 1,
}
