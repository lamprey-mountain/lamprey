import type { Room } from "sdk";

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
	| {
			type: "view";
			name: string;
			id: string;
			// TODO: views
	  };

export type RoomNavMappedItem =
	| { type: "room"; room: Room }
	| { type: "folder"; id: string; name: string; items: Room[] }
	| { type: "view"; name: string; id: string };

export type RoomNavConfig = RoomNavItem[];
