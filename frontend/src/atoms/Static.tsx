import { type Accessor, createEffect, createSignal, onCleanup } from "solid-js";
import fragmentShaderSource from "@/atoms/static.frag?raw";
import vertexShaderSource from "@/atoms/static.vert?raw";
import { compileProgram, compileShader } from "@/lib/webgl";

export const createStaticShader = (
	gl: WebGL2RenderingContext,
	resolution: Accessor<[number, number]>,
) => {
	const [paused, setPaused] = createSignal(false);

	// compile shaders
	const vert = compileShader(gl, gl.VERTEX_SHADER, vertexShaderSource);
	const frag = compileShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource);
	const prog = compileProgram(gl, vert, frag);
	gl.useProgram(prog);

	// setup quad
	const positionBuffer = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
	gl.bufferData(
		gl.ARRAY_BUFFER,
		new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]),
		gl.STATIC_DRAW,
	);

	const positionLocation = gl.getAttribLocation(prog, "position");
	gl.enableVertexAttribArray(positionLocation);
	gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);

	const timeLoc = gl.getUniformLocation(prog, "u_time");
	const resLoc = gl.getUniformLocation(prog, "u_res");

	let requestId: number;
	const render = (time: number) => {
		if (paused()) return;

		const [width, height] = resolution();
		gl.uniform1f(timeLoc, time * 0.001);
		gl.uniform2f(resLoc, width, height);
		gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
		requestId = requestAnimationFrame(render);
	};

	requestId = requestAnimationFrame(render);

	onCleanup(() => {
		cancelAnimationFrame(requestId);
		gl.deleteShader(vert);
		gl.deleteShader(frag);
		gl.deleteProgram(prog);
		gl.deleteBuffer(positionBuffer);
	});

	createEffect(() => {
		if (paused()) {
			cancelAnimationFrame(requestId);
		} else {
			requestAnimationFrame(render);
		}
	});

	return {
		pause(p: boolean) {
			setPaused(p);
		},
	};
};

export const createStaticShaderCanvas = (canvas: HTMLCanvasElement) => {
	const gl = canvas.getContext("webgl2")!;

	// automatically resize canvas
	const obs = new ResizeObserver((entries) => {
		for (const entry of entries) {
			const borderBoxSize = entry.borderBoxSize[0];
			if (
				canvas.width !== borderBoxSize.inlineSize ||
				canvas.height !== borderBoxSize.blockSize
			) {
				canvas.width = borderBoxSize.inlineSize;
				canvas.height = borderBoxSize.blockSize;
				gl.viewport(0, 0, canvas.width, canvas.height);
			}
		}
	});

	const ctl = createStaticShader(gl, () => [canvas.width, canvas.height]);

	obs.observe(canvas);
	onCleanup(() => obs.disconnect());

	return ctl;
};

export const Static = () => {
	const start = (canvas: HTMLCanvasElement) => {
		createStaticShaderCanvas(canvas);
	};

	return (
		<canvas
			ref={start}
			style="height: 300px; aspect-ratio: 16/9; display: block; border: solid blue 1px"
		/>
	);
};
