import { useApi } from "@/api";

export const DocumentComments = (props: { channel_id: string }) => {
	const api = useApi();
	const prefs = api.preferences.useRead();

	const openInSidebar = () => prefs.frontend.threads_sidebar_text === "yes"; // TODO: use this

	return (
		<div class="document-comments">
			<div class="background"></div>
			<div class="content">todo: copy ThreadPopout for document comments</div>
		</div>
	);
};
