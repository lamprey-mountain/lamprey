import { A } from "@solidjs/router";
import { Show } from "solid-js";
import { useApi } from "@/api";
import { AnimatedText } from "@/atoms/AnimatedText";
import { useCurrentUser } from "@/contexts/currentUser";
import { useModals } from "@/contexts/modal";
import { flags } from "@/lib/flags";

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
			<p>
				<AnimatedText>we're in schrodinger's toaster right now</AnimatedText>
			</p>
			<p>
				<AnimatedText animation="wave">
					this is a new category of entertainment, we haven't invented yet
				</AnimatedText>
			</p>

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
		</div>
	);
};
