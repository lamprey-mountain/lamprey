import { onCleanup, onMount } from "solid-js";
import { useModals } from "../contexts/modal";
import { Modal } from "./mod";

interface ModalCameraPreviewProps {
	stream: MediaStream;
}

export const ModalCameraPreview = (props: ModalCameraPreviewProps) => {
	const [, modalCtl] = useModals();

	let video: HTMLVideoElement | undefined;

	onMount(() => {
		if (video) {
			video.srcObject = props.stream;
			video.play().catch(console.error);
		}
	});

	onCleanup(() => {
		// Stop all tracks when modal closes
		props.stream.getTracks().forEach((track) => track.stop());
	});

	return (
		<Modal>
			<div class="modal-camera-preview">
				<h3>camera preview</h3>
				<video
					ref={video}
					autoplay
					playsinline
					muted
					style="width: 100%; max-width: 480px; border-radius: 8px; background: #000;"
				/>
				<div class="bottom">
					<button onClick={() => modalCtl.close()}>done</button>
				</div>
			</div>
		</Modal>
	);
};
