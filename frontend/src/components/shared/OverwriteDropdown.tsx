import { UserRoleSelect } from "@/atoms/UserRoleSelect";

export function OverwriteDropdown(props: {
	room_id: string;
	onSelect: (id: string, type: "Role" | "User") => void;
	excludeIds?: string[];
}) {
	return (
		<UserRoleSelect
			room_id={props.room_id}
			onSelect={props.onSelect}
			excludeIds={props.excludeIds}
			placeholder="Select a user/role"
			class="overwrite-dropdown"
		/>
	);
}
