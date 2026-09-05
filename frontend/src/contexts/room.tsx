import { createContext, useContext } from "solid-js";
import type { SetStoreFunction, Store } from "solid-js/store";

export type RoomState = {};

export function createInitialRoomState(): RoomState {
	return {};
}

export type RoomContextT = [Store<RoomState>, SetStoreFunction<RoomState>];

export const RoomContext = createContext<RoomContextT>();
export const useRoom = () => useContext(RoomContext);
