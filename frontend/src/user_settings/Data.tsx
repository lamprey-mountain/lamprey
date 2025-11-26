import { useModals } from "../contexts/modal";

export function Data() {
	const [, modalCtl] = useModals();

	const handleExportClick = () => {
		modalCtl.open({ type: "export_data" });
	};

	return (
		<div>
			<h2>data</h2>
			<p>manage your data on lamprey</p>
			<br />
			<h3>export</h3>
			<p>export all of your data in one big data dump</p>
			<button onClick={handleExportClick}>export</button>
		</div>
	);
}
