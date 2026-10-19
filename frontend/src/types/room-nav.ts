import type { api, Room } from "sdk";

export type RoomNavView = {
	type: "view";
	id: string;
	name: string;
	uncategorized_channels?: api["ViewChannel"][];
	categories?: api["ViewCategory"][];
};

export type RoomNavItem =
	| {
			type: "room";
			room_id: string;
	  }
	| {
			type: "folder";
			id: string;
			name: string;
			items: { type: "room"; room_id: string }[];
	  }
	| RoomNavView;

export type RoomNavMappedItem =
	| { type: "room"; room: Room }
	| { type: "folder"; id: string; name: string; items: Room[] }
	| RoomNavView;

export type RoomNavConfig = RoomNavItem[];
