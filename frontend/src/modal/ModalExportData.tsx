import { createSignal } from "solid-js";
import { Modal } from "./mod";
import { useModals } from "../contexts/modal";
import { Checkbox } from "../icons";

export const ModalExportData = () => {
	const [, modalCtl] = useModals();
	const [includeMessages, setIncludeMessages] = createSignal(true);

	const handleExport = () => {
		// TODO: exporting data
		modalCtl.close();
	};

	const handleCancel = () => {
		modalCtl.close();
	};

	return (
		<Modal>
			<h3>export data</h3>
			<form
				class="export-data-form"
				onSubmit={(e) => {
					e.preventDefault();
					handleExport();
				}}
			>
				<div class="option">
					<label class="option">
						<input
							type="checkbox"
							checked={includeMessages()}
							onInput={(e) => setIncludeMessages(e.currentTarget.checked)}
							style="display: none;"
						/>
						<Checkbox checked={includeMessages()} />
						<div>
							<div>Include messages</div>
						</div>
					</label>
				</div>

				<div class="bottom">
					<button type="button" onClick={handleCancel}>
						Cancel
					</button>
					<button type="submit" class="primary">
						Export
					</button>
				</div>
			</form>
		</Modal>
	);
};
