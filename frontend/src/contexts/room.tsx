import { Message, Pagination } from "sdk";
import { createContext, useContext } from "solid-js";
import { createStore, SetStoreFunction, Store } from "solid-js/store";

export type RoomSearch = {
	query: string;
	results: Pagination<Message> | null;
	loading: boolean;
	author?: string[];
	before?: string;
	after?: string;
	channel?: string[];
};

export type RoomState = {
	search?: RoomSearch;
};

export function createInitialRoomState(): RoomState {
	return {};
}

export type RoomContextT = [
	Store<RoomState>,
	SetStoreFunction<RoomState>,
];

export const RoomContext = createContext<RoomContextT>();
export const useRoom = () => useContext(RoomContext);
