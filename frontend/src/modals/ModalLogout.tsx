import { useApi } from "@/api";
import { useModals } from "@/contexts/modal";
import { Modal } from "./mod";

export const ModalLogout = () => {
	const api = useApi();
	const [, modalCtl] = useModals();

	const handleLogout = async () => {
		modalCtl.close();
		try {
			await api.logout();
		} catch (err) {
			console.error("Failed to log out:", err);
			modalCtl.alert("Failed to log out");
		}
	};

	return (
		<Modal
			onKeyDown={(e) => {
				if (e.key === "Enter") {
					handleLogout();
				} else if (e.key === "Escape") {
					modalCtl.close();
				}
			}}
		>
			<h3>log out</h3>
			<p>Are you sure you want to log out?</p>
			<div class="bottom">
				<button type="button" class="button" onClick={() => modalCtl.close()}>
					cancel
				</button>
				<button type="button" class="button danger" onClick={handleLogout}>
					log out
				</button>
			</div>
		</Modal>
	);
};
