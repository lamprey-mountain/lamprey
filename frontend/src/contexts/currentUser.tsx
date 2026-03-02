import { createContext, createMemo, JSX, useContext } from "solid-js";
import { UserWithRelationship } from "sdk";
import { useApi } from "../api";

const CurrentUserContext = createContext<
	() => UserWithRelationship | undefined
>();

export const CurrentUserProvider = (props: { children: JSX.Element }) => {
	const api = useApi();
	const currentUser = createMemo(() => api.users.cache.get("@self"));

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
	return context;
};
