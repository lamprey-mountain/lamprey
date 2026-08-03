import type { Object3D } from "three";
import type { Media } from "ts-sdk";

export const COLOR_BACKGROUND = 0x0c1012;
export const COLOR_OBJECT = 0x999999;
export const COLOR_GROUND = 0x444444;
export const COLOR_LIGHT = 0xffffff;

// TODO: split COLOR_LIGHT into ambient (hemisphere)/directional consts?

export const get3dKind = (media: Media) => {
	const ty = media.content_type.split(";")[0];
	const ext = media.filename.split(".").at(-1);

	if (
		ty === "application/vnd.ms-pki.stl" ||
		ty === "model/stl" ||
		ty === "application/sla" ||
		ext === "stl"
	) {
		return "stl";
	} else if (
		ty === "model/obj" ||
		ty === "application/x-tgif" ||
		ext === "obl"
	) {
		return "obj";
	} else {
		return null;
	}
};

export const is3D = (media: Media) => {
	return get3dKind(media) !== null;
};

// TODO: use LoadingManager?
export class Loader {
	async load(media: Media, url: string): Promise<Object3D> {
		const ty = media.content_type.split(";")[0];
		const ext = media.filename.split(".").at(-1);

		// PERF: cache STLLoader and OBJLoader?
		// instead of doing new FooLoader on every load(0)

		if (
			ty === "application/vnd.ms-pki.stl" ||
			ty === "model/stl" ||
			ty === "application/sla" ||
			ext === "stl"
		) {
			const [THREE, { STLLoader }] = await Promise.all([
				import("three"),
				import("three/examples/jsm/loaders/STLLoader.js"),
			]);
			const { promise, resolve, reject } = Promise.withResolvers<Object3D>();

			new STLLoader().load(
				url,
				(geometry) => {
					const material = new THREE.MeshPhongMaterial({ color: COLOR_OBJECT });
					const mesh = new THREE.Mesh(geometry, material);
					resolve(mesh);
				},
				(_progress) => {},
				(error) => {
					reject(error);
				},
			);

			return promise;
		} else if (
			ty === "model/obj" ||
			ty === "application/x-tgif" ||
			ext === "obl"
		) {
			const [THREE, { OBJLoader }] = await Promise.all([
				import("three"),
				import("three/examples/jsm/loaders/OBJLoader.js"),
			]);
			const { promise, resolve, reject } = Promise.withResolvers<Object3D>();

			new OBJLoader().load(
				url,
				(geometry) => {
					// const material = new THREE.MeshPhongMaterial({ color: COLOR_OBJECT });
					// const mesh = new THREE.Mesh(geometry, material);

					// geometry.traverse(child => {
					//   if (child.) {
					//     child.
					//   }
					// })

					resolve(geometry);
				},
				(_progress) => {},
				(error) => {
					reject(error);
				},
			);

			return promise;
		} else {
			throw new Error("unknown file type");
		}
	}
}
