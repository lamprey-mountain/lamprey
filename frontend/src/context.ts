import { Accessor, createContext, Setter } from "solid-js";
import { Client, Room, Thread } from "sdk";

type ChatProps = {
	client: Client;
	roomId: Accessor<string | undefined>;
	threadId: Accessor<string | undefined>;
	room: Accessor<Room | undefined>;
	thread: Accessor<Thread | undefined>;
	setRoomId: Setter<string | undefined>;
	setThreadId: Setter<string | undefined>;
};

export const chatctx = createContext<ChatProps>();
