import {
	type Accessor,
	createContext,
	createSignal,
	type ParentComponent,
	type Setter,
	useContext,
} from "solid-js";

export type Menu =
	& {
		x: number;
		y: number;
	}
	& (
		| { type: "room"; room_id: string }
		| { type: "channel"; channel_id: string }
		| {
			type: "message";
			channel_id: string;
			message_id: string;
			version_id: string;
		}
		| {
			type: "user";
			user_id: string;
			channel_id?: string;
			room_id?: string;
			admin: boolean;
		}
		| { type: "folder"; folder_id: string }
	);

export type MenuContextT = {
	menu: Accessor<Menu | null>;
	setMenu: Setter<Menu | null>;
};

const MenuContext = createContext<MenuContextT>();

export const MenuProvider: ParentComponent = (props) => {
	const [menu, setMenu] = createSignal<Menu | null>(null);

	return (
		<MenuContext.Provider value={{ menu, setMenu }}>
			{props.children}
		</MenuContext.Provider>
	);
};

export const useMenu = () => {
	const context = useContext(MenuContext);
	if (!context) {
		throw new Error("useMenu must be used within a MenuProvider");
	}
	return context;
};
