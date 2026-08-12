import { A } from "@solidjs/router";
import { Show } from "solid-js";
import { useApi } from "@/api";
import { UnicodeEmoji } from "@/atoms/UnicodeEmoji";
import { useCurrentUser } from "@/contexts/currentUser";
import { useModals } from "@/contexts/modal";
import { flags } from "@/lib/flags";
import { Authenticate } from "./Authenticate";

export const Home = () => {
	const api = useApi();
	const user = useCurrentUser();
	const [, modalctl] = useModals();

	function openRoomModal() {
		modalctl.open({
			type: "room_create_or_join",
		});
	}

	async function logout() {
		await api.logout();
	}

	const isAuthorized = () => api.session()?.status === "Authorized";

	return (
		<div class="home">
			<h2>home</h2>
			<p>welcome to lamprey mountain, the internet's finest asylum</p>
			<p>work in progress. expect bugs and missing polish.</p>

			<Show when={isAuthorized()}>
				<button type="button" class="button" onClick={logout}>
					logout
				</button>
				<br />
				<br />
				<Show when={user()}>
					<button type="button" class="button" onClick={openRoomModal}>
						create or join room
					</button>
					<br />
				</Show>
			</Show>

			<A target="_self" href="/api/docs">
				api docs
			</A>
			<br />
			<Show when={flags.has("dev")}>
				<A href="/debug">debug</A>
				<br />
			</Show>
		</div>
	);
};
