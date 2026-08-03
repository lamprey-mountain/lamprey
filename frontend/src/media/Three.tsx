import { createEffect, createMemo, createSignal, onCleanup } from "solid-js";
import type { Object3D, WebGLRenderer } from "three";
import type { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { useConfig } from "@/lib/config";
import {
	COLOR_BACKGROUND,
	COLOR_GROUND,
	COLOR_LIGHT,
	Loader,
} from "./three-util";
import {
	formatBytes,
	getUrl,
	Loader as MediaLoader,
	type MediaProps,
} from "./util";

export const ThreeView = (props: MediaProps) => {
	const ty = createMemo(() => props.media.content_type.split(";")[0]);
	const config = useConfig();
	const [loaded, setLoaded] = createSignal(false);
	const loader = new Loader();

	const three = Promise.all([
		import("three"),
		import("three/examples/jsm/controls/OrbitControls.js"),
	]);

	const init = (containerEl: HTMLDivElement) => {
		const destroyed = false;
		let obs: ResizeObserver;
		let renderRequestId: number | undefined;
		let controls: OrbitControls;
		let renderer: WebGLRenderer;

		three.then(([THREE, { OrbitControls }]) => {
			if (destroyed) return;
			const scene = new THREE.Scene();
			scene.background = new THREE.Color(COLOR_BACKGROUND);

			// TODO: dedupe this logic with getUrl somehow?
			const url = new URL(`/media/${props.media.id}`, config.cdn_url);
			loader.load(props.media, url.href).then((obj) => {
				scene.add(obj);
				fitCameraToObject(obj);
				setLoaded(true);
				// TODO: remove objects on cleanup and when switching props.media
				// scene.remove(obj);
				// FIXME:  m.geometry?.dispose?.();
				// FIXME: m.material?.dispose?.();
			});

			renderer = new THREE.WebGLRenderer({ antialias: true });
			renderer.setSize(containerEl.clientWidth, containerEl.clientHeight);
			containerEl.append(renderer.domElement);

			// add lights
			scene.add(new THREE.HemisphereLight(COLOR_LIGHT, COLOR_GROUND, 1.5));
			const dirLight = new THREE.DirectionalLight(COLOR_LIGHT, 1);
			dirLight.position.set(1, -1, 1);
			scene.add(dirLight);

			// setup camera
			const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1000);

			// setup orbit controls
			controls = new OrbitControls(camera, renderer.domElement);
			controls.enableDamping = true;

			function fitCameraToObject(object: Object3D) {
				const box = new THREE.Box3().setFromObject(object);
				const size = box.getSize(new THREE.Vector3());
				const center = box.getCenter(new THREE.Vector3());

				// center the model
				object.position.sub(center);

				const maxDim = Math.max(size.x, size.y, size.z);
				const fov = camera.fov * (Math.PI / 180);
				const dist = Math.abs(maxDim / Math.sin(fov / 2));
				camera.position.set(0, 0, dist);
				camera.lookAt(0, 0, 0);
				controls.target.set(0, 0, 0);
				controls.update();
			}

			const render = () => {
				renderRequestId = undefined;
				controls.update();
				renderer.render(scene, camera);
			};

			const renderIfNeeded = () => {
				if (!renderRequestId) {
					renderRequestId = requestAnimationFrame(render);
				}
			};

			// sync dimentions
			obs = new ResizeObserver((entries) => {
				for (const ent of entries) {
					const h = ent.borderBoxSize[0].blockSize;
					const w = ent.borderBoxSize[0].inlineSize;
					camera.aspect = w / h;
					camera.updateProjectionMatrix();
					renderer.setSize(w, h);
				}
				renderIfNeeded();
			});
			obs.observe(containerEl);

			render();
			controls.addEventListener("change", renderIfNeeded);
		});

		createEffect(() => {
			// TODO: update mesh when props.media changes
			// setLoaded(false);
		});

		onCleanup(() => {
			obs.disconnect();
			if (renderRequestId) cancelAnimationFrame(renderRequestId);
			controls.dispose();
			renderer.dispose();
		});
	};

	return (
		<div class="media three" classList={{ "hide-ui": false }} ref={init}>
			<MediaLoader loaded={loaded()} />
			<div class="top">
				<div class="info">
					<a download={props.media.filename} href={getUrl(props.media)}>
						download {props.media.filename}
					</a>
				</div>
				<div class="dim bottom">
					{ty()} - {formatBytes(props.media.size)}
				</div>
			</div>
		</div>
	);
};
