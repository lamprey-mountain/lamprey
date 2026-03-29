import { createSignal } from "solid-js";
import { CheckboxOption } from "../atoms/CheckboxOption";
import { useModals } from "../contexts/modal";
import { Checkbox } from "../icons";
import { Modal } from "./mod";

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
