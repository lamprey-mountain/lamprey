export type MessageT = {
	id: string,
	thread_id: string,
	version_id: string,
	reply_id: string | null,
	nonce: string | null,
	content: string | null,
	author: UserT,
	attachments: Array<any>,
	override_name: string | null,
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

export enum DiscordIntent {
  GUILDS = 1 << 0,
  GUILDS_MESSAGES = 1 << 9,
  GUILDS_MESSAGE_REACTIONS = 1 << 10,
  GUILDS_MESSAGE_TYPING = 1 << 11,
  MESSAGE_CONTENT = 1 << 15,
}
