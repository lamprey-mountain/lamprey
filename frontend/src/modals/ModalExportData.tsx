import { createSignal, Show } from "solid-js";
import { useApi } from "@/api";
import { ApiErrorDisplay, isApiError } from "@/atoms/ApiErrorDisplay";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";
import { useModals } from "@/contexts/modal";
import { Modal } from "./mod";

export const ModalExportData = () => {
	const api = useApi();
	const [, modalCtl] = useModals();
	const [includeMessages, setIncludeMessages] = createSignal(true);
	const [includeReactions, setIncludeReactions] = createSignal(false);
	const [exporting, setExporting] = createSignal(false);
	const [error, setError] = createSignal<unknown>(null);
	const apiError = () => {
		const err = error();
		return isApiError(err) ? err : null;
	};

	const handleExport = async () => {
		setError(null);
		setExporting(true);

		try {
			await api.harvest.create({
				include_messages: includeMessages(),
				include_reactions: includeReactions(),
			});
			modalCtl.close();
		} catch (err) {
			setError(err);
		} finally {
			setExporting(false);
		}
	};

	return (
		<Modal>
			<h3>export data</h3>
			<form
				class="export-data-form"
				onSubmit={(e) => {
					e.preventDefault();
					void handleExport();
				}}
			>
				<CheckboxOption
					id="modal-export-data-include-messages"
					checked={includeMessages()}
					onChange={setIncludeMessages}
					seed="modal-export-data-include-messages"
				>
					<Checkbox
						checked={includeMessages()}
						seed="modal-export-data-include-messages"
					/>
					<div>
						<div>Include messages</div>
					</div>
				</CheckboxOption>

				<CheckboxOption
					id="modal-export-data-include-reactions"
					checked={includeReactions()}
					onChange={setIncludeReactions}
					seed="modal-export-data-include-reactions"
				>
					<Checkbox
						checked={includeReactions()}
						seed="modal-export-data-include-reactions"
					/>
					<div>
						<div>Include reactions</div>
					</div>
				</CheckboxOption>

				<Show when={apiError()}>
					{(err) => <ApiErrorDisplay error={err()} />}
				</Show>
				<Show when={error() && !apiError()}>
					<p class="error">failed to start export</p>
				</Show>

				<div class="bottom">
					<button
						type="button"
						class="button"
						disabled={exporting()}
						onClick={() => modalCtl.close()}
					>
						Cancel
					</button>
					<button type="submit" class="button primary" disabled={exporting()}>
						Export
					</button>
				</div>
			</form>
		</Modal>
	);
};
