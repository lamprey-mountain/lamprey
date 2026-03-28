import { Show } from "solid-js";
import { useApi2 } from "@/api";
import { useCtx } from "../context.ts";
import { Item, Menu, Separator } from "./Parts.tsx";
import { useModals } from "../contexts/modal";

type UserAdminMenuProps = {
	user_id: string;
};

export function UserAdminMenu(props: UserAdminMenuProps) {
	const ctx = useCtx();
	const api2 = useApi2();
	const user = api2.users.fetch(props.user_id) as any;
	const [, modalCtl] = useModals();

	const copyUserId = () => navigator.clipboard.writeText(props.user_id);
	const logToConsole = () =>
		console.log(JSON.parse(JSON.stringify((user as any)())));

	const suspendUser = () => {
		modalCtl.prompt("suspend reason", (reason: any) => {
			if (!reason) return;
			api2.client.http.POST("/api/v1/user/{user_id}/suspend" as any, {
				params: {
					path: {
						user_id: props.user_id,
					},
				},
				headers: {
					"X-Reason": reason,
				},
				body: {},
			});
		});
	};

	const unsuspendUser = () => {
		modalCtl.prompt("unsuspend reason", (reason: any) => {
			if (!reason) return;
			api2.client.http.DELETE("/api/v1/user/{user_id}/suspend" as any, {
				params: {
					path: {
						user_id: props.user_id,
					},
				},
				headers: {
					"X-Reason": reason,
				},
			});
		});
	};

	const deleteUser = () => {
		modalCtl.confirm(
			"Are you sure you want to delete this user? This action cannot be undone.",
			(confirmed) => {
				if (!confirmed) return;
				api2.client.http.DELETE("/api/v1/user/{user_id}", {
					params: {
						path: {
							user_id: props.user_id,
						},
					},
				});
			},
		);
	};

	return (
		<Menu>
			<Show when={user()?.suspended}>
				<Item onClick={unsuspendUser}>unsuspend user</Item>
			</Show>
			<Show when={!user()?.suspended}>
				<Item onClick={suspendUser}>suspend user</Item>
			</Show>
			<Item onClick={deleteUser} color="danger">delete user</Item>
			<Separator />
			<Item onClick={copyUserId}>copy user id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
