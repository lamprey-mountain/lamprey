import { createContext, createMemo, type JSX, useContext } from "solid-js";
import type { UserWithRelationship } from "sdk";
import { useUsers2 } from "@/api";
import { logger } from "../logger";

const currentUserLog = logger.for("current_user");

const CurrentUserContext = createContext<
	() => UserWithRelationship | undefined
>();

export const CurrentUserProvider = (props: { children: JSX.Element }) => {
	const users2 = useUsers2();
	const currentUser = createMemo(() => {
		const user = users2.cache.get("@self");
		currentUserLog.debug("currentUser memo", {
			found: !!user,
			user_id: user?.id,
		});
		return user;
	});

	currentUserLog.info("CurrentUserProvider initialized");

	return (
		<CurrentUserContext.Provider value={currentUser}>
			{props.children}
		</CurrentUserContext.Provider>
	);
};

export const useCurrentUser = () => {
	const context = useContext(CurrentUserContext);
	if (!context) {
		throw new Error("useCurrentUser must be used within a CurrentUserProvider");
	}
	// const user = context();
	// currentUserLog.debug("useCurrentUser", {
	// 	found: !!user,
	// 	user_id: user?.id
	// });
	return context;
};
