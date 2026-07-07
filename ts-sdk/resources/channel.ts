import type { api } from "../index";
import type { MessagesManager } from "./message";
import type { Room } from "./room";

type ApiChannel = api["Channel"];

export type Channel = ApiChannel & ChannelExt;

export interface ChannelExt {
	hasText(): this is ChanText;
	// hasFoo() for room, thread, other components, etc...
	// async fetch() {}
	// toJSON(): ApiRoom {}

	// split to separate components
	// members
	// invites
	// webhooks
	// permissions(?)
	// threads
	// parent?: category channel
}

export interface ChanText {
	messages: MessagesManager;
}

export interface ChanRoom {
	roomId: string;
	delete(): Promise<void>;
	// update(): Promise<void>;
}

export interface ChanThread {
	parentId: string;
}

export type ChanThreadPrivate = {};

export type ChanVoice = {};

export type ChanDm = {};

export interface ChanGdm {
	leave(): Promise<void>;
}

// TODO: create types for all channel types
export type ChannelDm = Channel & ChanText;
export type ChannelGdm = Channel & ChanText;
export type ChannelText = Channel & ChanText & ChanRoom;
export type ChannelThreadPublic = Channel & ChanText & ChanRoom & ChanThread;
export type ChannelThreadPrivate = Channel &
	ChanText &
	ChanRoom &
	ChanThread &
	ChanThreadPrivate;
export type ChannelVoice = Channel & ChanRoom & ChanVoice;

export type RoomChannel =
	| ChannelText
	| ChannelThreadPublic
	| ChannelThreadPrivate
	| ChannelVoice; // etc...
export type DirectChannel = ChannelDm | ChannelGdm;
export type KnownChannel = RoomChannel | DirectChannel;

export abstract class ChannelsManager<T extends Channel = Channel> {
	async fetch(id: string): Promise<T> {
		throw "todo";
	}
}

export class DirectChannelsManager extends ChannelsManager<DirectChannel> {}

export class RoomChannelsManager extends ChannelsManager<RoomChannel> {
	constructor(public room: Room) {
		super();
	}
}
