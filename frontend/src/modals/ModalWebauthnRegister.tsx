import { createSignal, onMount, Show } from "solid-js";
import { useApi } from "@/api";
import { useModals } from "@/contexts/modal";
import { createPasskey, webauthnErrorMessage } from "@/lib/webauthn";
import { Modal } from "./mod";

export const ModalWebauthnRegister = (props: { onRegistered?: () => void }) => {
	const api = useApi();
	const [, modalCtl] = useModals();

	const [name, setName] = createSignal("");
	const [error, setError] = createSignal<string | null>(null);
	const [busy, setBusy] = createSignal(false);

	let nameRef: HTMLInputElement | undefined;
	onMount(() => queueMicrotask(() => nameRef?.focus()));

	const register = async () => {
		setError(null);
		setBusy(true);
		try {
			const challenge = await api.auth.webauthnRegisterChallenge();
			const credential = await createPasskey(challenge);
			const authenticator = await api.auth.webauthnRegisterFinish(credential);

			const trimmed = name().trim();
			if (trimmed) await api.auth.webauthnRename(authenticator.id, trimmed);

			props.onRegistered?.();
			modalCtl.close();
		} catch (err) {
			console.error("failed to register passkey", err);
			setError(webauthnErrorMessage(err));
		} finally {
			setBusy(false);
		}
	};

	return (
		<Modal
			class="webauthn-register"
			closeOnBackdrop={!busy()}
			onKeyDown={(e) => {
				if (e.key === "Escape") {
					modalCtl.close();
				} else if (e.key === "Enter" && !busy()) {
					void register();
				}
			}}
		>
			<h3>add passkey</h3>
			<div class="hint">
				your device will ask for your fingerprint, face, or security key.
			</div>
			<label class="field">
				<span class="field-label dim">name</span>
				<input
					ref={nameRef}
					class="input"
					type="text"
					placeholder="yubikey"
					maxlength={64}
					value={name()}
					disabled={busy()}
					onInput={(e) => setName(e.currentTarget.value)}
				/>
			</label>
			<Show when={error()}>
				{(err) => (
					<div class="error" role="alert">
						{err()}
					</div>
				)}
			</Show>
			<div class="bottom">
				<button
					type="button"
					class="button"
					disabled={busy()}
					onClick={() => modalCtl.close()}
				>
					cancel
				</button>
				<button
					type="button"
					class="button primary"
					disabled={busy()}
					onClick={register}
				>
					{busy() ? "waiting for passkey..." : "register"}
				</button>
			</div>
		</Modal>
	);
};
