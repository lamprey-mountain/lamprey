import { Show } from "solid-js";
import { useApi } from "@/api";
import { useModals } from "@/contexts/modal";
import { Item, Menu, Separator } from "./Parts.tsx";

type MediaAdminMenuProps = {
	media_id: string;
};

export function MediaAdminMenu(props: MediaAdminMenuProps) {
	const api2 = useApi();
	const media2 = api2.media;
	const media = media2.use(() => props.media_id);
	const [, modalCtl] = useModals();

	const copyMediaId = () => navigator.clipboard.writeText(props.media_id);
	const logToConsole = () => console.log(JSON.parse(JSON.stringify(media())));

	const quarantine = async () => {
		const { data } = await api2.client.http.POST(
			"/api/v1/media/{media_id}/quarantine",
			{ params: { path: { media_id: props.media_id } } },
		);
		if (data) media2.upsert(data);
	};

	const unquarantine = async () => {
		const { data } = await api2.client.http.DELETE(
			"/api/v1/media/{media_id}/quarantine",
			{ params: { path: { media_id: props.media_id } } },
		);
		if (data) media2.upsert(data);
	};

	const deleteMedia = () => {
		modalCtl.confirm(
			"Are you sure you want to delete this media? This action cannot be undone.",
			async (confirmed) => {
				if (!confirmed) return;
				await api2.client.http.DELETE("/api/v1/media/{media_id}", {
					params: { path: { media_id: props.media_id } },
				});
				media2.delete(props.media_id);
			},
		);
	};

	return (
		<Menu>
			<Show
				when={media()?.quarantine}
				fallback={
					<Item onClick={quarantine} color="danger">
						quarantine media
					</Item>
				}
			>
				<Item onClick={unquarantine}>unquarantine media</Item>
			</Show>
			<Item onClick={deleteMedia} color="danger">
				delete media
			</Item>
			<Separator />
			<Item onClick={copyMediaId}>copy media id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
