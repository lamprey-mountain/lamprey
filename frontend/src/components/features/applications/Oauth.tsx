import { createSignal, Index, Show } from "solid-js";
import { useApi } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Checkbox } from "@/atoms/icons";
import { createTooltip } from "@/atoms/Tooltip";
import { Copyable2 } from "@/utils/general";
import { icDelete } from "@/utils/icons";
import { type ApplicationDraft, useApplications } from "./context";

export const Oauth = (props: { draft: ApplicationDraft }) => {
	const apps = useApplications();
	const api = useApi();
	const [secret, setSecret] = createSignal<string>();
	const [focusIdx, setFocusIdx] = createSignal<number | null>(null);

	const isCreate = () => props.draft.state === "create";
	const appId = () => (isCreate() ? props.draft.nonce : props.draft.data.id);

	const updateApp = (key: string, value: any) => {
		apps.updateDraft(appId(), { [key]: value });
	};

	const handleRotate = async () => {
		const { data } = await api.client.http.POST(
			"/api/v1/app/{app_id}/rotate-secret",
			{
				params: { path: { app_id: appId() } },
			},
		);
		setSecret(data?.oauth_secret);
	};

	const currentData = () => {
		const d = props.draft;
		if (d.state === "create") return d.create;
		if (d.state === "update") return { ...d.data, ...d.update };
		return d.data;
	};

	const oauth_confidential = () => (currentData() as any).oauth_confidential;
	const oauth_redirect_uris = () =>
		(currentData() as any).oauth_redirect_uris ?? [];

	return (
		<div class="oauth">
			<h3>oauth</h3>
			<p>configure lamprey as an oauth provider</p>
			<label class="option">
				<input
					type="checkbox"
					checked={!!oauth_confidential()}
					onInput={(e) =>
						updateApp("oauth_confidential", e.currentTarget.checked)
					}
					style="display: none;"
				/>
				<Checkbox
					checked={!!oauth_confidential()}
					seed={`app-${appId()}-oauth-confidential`}
				/>
				<div>
					<div>Confidential client</div>
					<div class="dim">
						Check if this client can keep secrets (ie. client secret is stored
						server side). Don't check for web browser clients.
					</div>
				</div>
			</label>
			<br />
			<div class="redirect-uris">
				<h3 class="dim">redirect uris</h3>
				<Index
					each={oauth_redirect_uris()}
					fallback={<div class="empty">no redirect uris</div>}
				>
					{(uriAccessor, uriIndex) => {
						const tip = createTooltip({ tip: () => "Remove" });

						return (
							<div class="redirect-uri">
								<input
									type="text"
									placeholder="https://example.com/redirect"
									value={uriAccessor()}
									onInput={(e) => {
										const newUris = [...oauth_redirect_uris()];
										newUris[uriIndex] = e.currentTarget.value;
										updateApp("oauth_redirect_uris", newUris);
									}}
									ref={(el) => {
										if (focusIdx() === uriIndex) {
											queueMicrotask(() => el.focus());
											setFocusIdx(null);
										}
									}}
								/>
								<button
									type="button"
									class="button icon-button danger"
									onClick={() => {
										const newUris = [...oauth_redirect_uris()];
										newUris.splice(uriIndex, 1);
										updateApp("oauth_redirect_uris", newUris);
									}}
									ref={tip.content}
								>
									<Icon src={icDelete} />
								</button>
							</div>
						);
					}}
				</Index>
				<button
					type="button"
					class="button"
					onClick={() => {
						const uris = oauth_redirect_uris();
						setFocusIdx(uris.length);
						const newUris = [...uris, ""];
						updateApp("oauth_redirect_uris", newUris);
					}}
				>
					add uri
				</button>
			</div>
			<br />
			<br />
			<Show when={!isCreate()}>
				<div class="rotate-secret">
					<h3 class="label">rotate secret</h3>
					<p class="markdown">
						This will <b>immediately</b> reset your OAuth2 client secret. The
						secret will only be shown once.
					</p>
					<Show when={secret()}>
						<div class="secret">
							Your new secret is
							<Copyable2 name="secret">{secret()!}</Copyable2>
						</div>
					</Show>
					<button type="button" class="button danger" onClick={handleRotate}>
						rotate
					</button>
				</div>
			</Show>
		</div>
	);
};
