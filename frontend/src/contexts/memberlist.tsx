import { createContext, createEffect, ParentProps, useContext } from "solid-js";
import { useApi2, useMemberList2 } from "../api.tsx";
import type { MemberList } from "../api.tsx";
import { ReactiveMap } from "@solid-primitives/map";
import { useLocation } from "@solidjs/router";

const MemberListContext = createContext<ReactiveMap<string, MemberList>>();

export const MemberListProvider = (props: ParentProps) => {
	const store = useApi2();
	const service = useMemberList2();
	const location = useLocation();

	let currentSubscription: string | null = null;
	createEffect(() => {
		const roomIdMatch = location.pathname.match(/\/room\/([^/]+)/);
		if (roomIdMatch) {
			const id = roomIdMatch[1];
			if (currentSubscription !== id) {
				currentSubscription = id;
				store.roomMembers.subscribeList(id, [[0, 199]]);
			}
			return;
		}

		const channelIdMatch = location.pathname.match(
			/\/(channel|thread)\/([^/]+)/,
		);
		if (channelIdMatch) {
			const id = channelIdMatch[2];
			if (currentSubscription !== id) {
				currentSubscription = id;
				store.threadMembers.subscribeList(id, [[0, 199]]);
			}
			return;
		}

		if (currentSubscription !== null) {
			currentSubscription = null;
			store.client.send({ type: "MemberListSubscribe", ranges: [] });
		}
	});

	return (
		<MemberListContext.Provider value={service.lists}>
			{props.children}
		</MemberListContext.Provider>
	);
};

export const useMemberList = () => {
	return useContext(MemberListContext)!;
};
