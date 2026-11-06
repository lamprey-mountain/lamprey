import { debounce } from "@solid-primitives/scheduled";
import {
	createEffect,
	createMemo,
	createSignal,
	For,
	onMount,
	Show,
} from "solid-js";
import { useApi } from "@/api";
import { CheckboxOptionWithLabel } from "@/atoms/CheckboxOption";
import { OtpInput } from "@/atoms/OtpInput";
import { useModals } from "@/contexts/modal";
import { Copyable2 } from "@/utils/general";
import { Modal } from "./mod";

export const ModalTotpSetup = (props: { account_name: string }) => {
	const api = useApi();
	const [, modalCtl] = useModals();

	const [secret, setSecret] = createSignal<string | null>(null);
	const [code, setCode] = createSignal("");
	const [codes, setCodes] = createSignal<
		{ code: string; used_at?: string | null }[]
	>([]);
	const [done, setDone] = createSignal(false);
	const [error, setError] = createSignal<string | null>(null);
	const [busy, setBusy] = createSignal(false);
	const [savedCodes, setSavedCodes] = createSignal(false);
	const [copiedAll, setCopiedAll] = createSignal(false);
	const clearCopied = debounce(() => setCopiedAll(false), 2000);

	onMount(() => {
		void (async () => {
			// TODO: remove these casts when the openapi schema is regenerated
			// with the totp routes (mirror the casts in Oauth)
			const { data, error: err } = await api.client.http.POST(
				// biome-ignore lint/suspicious/noExplicitAny: schema is outdated
				"/api/v1/auth/totp/init" as any,
				{},
			);
			if (err) {
				setError(`failed to start: ${err.message ?? "unknown error"}`);
				return;
			}
			setSecret((data as { secret: string } | undefined)?.secret ?? null);
		})();
	});

	const otpauthUri = createMemo(() => {
		const s = secret();
		if (!s) return null;
		const label = encodeURIComponent(props.account_name);
		const issuer = encodeURIComponent("Lamprey Mountain");
		return `otpauth://totp/${label}?secret=${s}&issuer=${issuer}`;
	});

	let qrRef: HTMLDivElement | undefined;

	// lazy load the qr library only once there's a secret to scan
	createEffect(async () => {
		const uri = otpauthUri();
		if (!uri || !qrRef) return;
		try {
			const { encodeQR } = await import("qr");
			// svg comes from the qr library, not user input
			qrRef.innerHTML = encodeQR(uri, "svg", { border: 1 });
		} catch (err) {
			console.error("failed to render qr code", err);
		}
	});

	const secretFormatted = createMemo(() => {
		const s = secret();
		if (!s) return "";
		return s.replace(/(.{4})/g, "$1 ").trim();
	});

	const enable = async () => {
		setError(null);
		setBusy(true);
		const { error: err } = await api.client.http.POST(
			// biome-ignore lint/suspicious/noExplicitAny: schema is outdated
			"/api/v1/auth/totp/enable" as any,
			// biome-ignore lint/suspicious/noExplicitAny: schema is outdated
			{ body: { code: code() } } as any,
		);
		setBusy(false);
		if (err) {
			setError("invalid code");
			return;
		}
		const { data, error: codesErr } = await api.client.http.GET(
			// biome-ignore lint/suspicious/noExplicitAny: schema is outdated
			"/api/v1/auth/totp/recovery-codes" as any,
			{},
		);
		if (codesErr) {
			setError("failed to load recovery codes");
			return;
		}
		setCodes(
			(data as { codes: { code: string; used_at?: string | null }[] }).codes ??
				[],
		);
		setSavedCodes(false);
		setDone(true);
	};

	const copyAll = () => {
		navigator.clipboard.writeText(
			codes()
				.map((rc) => rc.code)
				.join("\n"),
		);
		setCopiedAll(true);
		clearCopied();
	};

	const enterCode = (value: string) => {
		setCode(value);
		setError(null);
	};

	const download = () => {
		const text = [
			"Lamprey Mountain TOTP recovery codes",
			`account: ${props.account_name}`,
			"",
			...codes().map((rc) => rc.code),
			"",
		].join("\n");
		const blob = new Blob([text], { type: "text/plain" });
		const url = URL.createObjectURL(blob);
		const a = document.createElement("a");
		a.href = url;
		a.download = "lamprey-mountain-recovery-codes.txt";
		a.click();
		URL.revokeObjectURL(url);
	};

	return (
		<Modal
			class="totp-setup"
			closeOnBackdrop={!done()}
			onKeyDown={(e) => {
				if (e.key === "Escape") {
					modalCtl.close();
				} else if (e.key === "Enter" && !done() && code() !== "") {
					void enable();
				}
			}}
		>
			<h3>{done() ? "totp enabled!" : "enable totp"}</h3>
			<Show
				when={done()}
				fallback={
					<Show when={secret()} fallback={<div class="hint">loading...</div>}>
						<div class="columns">
							<div class="qr-section">
								<div
									class="totp-qr"
									ref={qrRef}
									role="img"
									aria-label="QR code for authenticator app"
								/>
							</div>
							<div class="details">
								<div class="hint">
									Scan this QR code with your authenticator app, or enter the
									key manually.
								</div>
								<div class="field">
									<span class="field-label dim">manual entry key</span>
									<Copyable2 name="secret" copy={secret() ?? ""}>
										{secretFormatted()}
									</Copyable2>
								</div>
								<div class="field">
									<span class="field-label dim">verification code</span>
									<OtpInput
										value={code()}
										onInput={enterCode}
										disabled={busy()}
										error={error() !== null}
										autofocus
									/>
								</div>
							</div>
						</div>
						<Show when={error()}>
							<div class="error" role="alert">
								{error()}
							</div>
						</Show>
						<div class="bottom">
							<button
								type="button"
								class="button"
								onClick={() => modalCtl.close()}
							>
								cancel
							</button>
							<button
								type="button"
								class="button primary"
								disabled={busy() || code() === ""}
								onClick={enable}
							>
								confirm
							</button>
						</div>
					</Show>
				}
			>
				<div class="warning">
					<b>Save these recovery codes</b> in case you lose access to your
					authenticator app. Each code can only be used once.
				</div>
				<div class="recovery-codes">
					<For each={codes()}>{(rc) => <span>{rc.code}</span>}</For>
				</div>
				<CheckboxOptionWithLabel
					id="totp-codes-saved"
					seed="totp-codes-saved"
					label="I've saved these codes somewhere safe"
					checked={savedCodes()}
					onChange={setSavedCodes}
				/>
				<div class="bottom">
					<button type="button" class="button" onClick={copyAll}>
						{copiedAll() ? "copied!" : "copy all"}
					</button>
					<button type="button" class="button" onClick={download}>
						download
					</button>
					<button
						type="button"
						class="button primary"
						disabled={!savedCodes()}
						onClick={() => modalCtl.close()}
					>
						done
					</button>
				</div>
			</Show>
		</Modal>
	);
};
