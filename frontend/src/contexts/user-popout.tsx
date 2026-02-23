import {
	type Accessor,
	createContext,
	createSignal,
	type ParentComponent,
	type Setter,
	useContext,
} from "solid-js";

export type UserViewData = {
	user_id: string;
	room_id?: string;
	channel_id?: string;
	ref: HTMLElement;
	source?: "member-list" | "message";
};

export type UserPopoutContextT = {
	userView: Accessor<UserViewData | null>;
	setUserView: Setter<UserViewData | null>;
};

const UserPopoutContext = createContext<UserPopoutContextT>();

export const UserPopoutProvider: ParentComponent = (props) => {
	const [userView, setUserView] = createSignal<UserViewData | null>(null);

	return (
		<UserPopoutContext.Provider value={{ userView, setUserView }}>
			{props.children}
		</UserPopoutContext.Provider>
	);
};

export const useUserPopout = () => {
	const context = useContext(UserPopoutContext);
	if (!context) {
		throw new Error("useUserPopout must be used within a UserPopoutProvider");
	}
	return context;
};
