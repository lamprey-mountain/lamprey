export function compileShader(
	gl: WebGLRenderingContext | WebGL2RenderingContext,
	type: number,
	source: string,
): WebGLShader {
	const shader = gl.createShader(type);
	if (!shader) {
		throw new Error("Failed to create WebGL shader object.");
	}

	gl.shaderSource(shader, source);
	gl.compileShader(shader);

	const success = gl.getShaderParameter(shader, gl.COMPILE_STATUS);
	if (!success) {
		const infoLog = gl.getShaderInfoLog(shader);
		gl.deleteShader(shader);
		throw new Error(`Failed to compile WebGL shader: ${infoLog}`);
	}

	return shader;
}

export function createWebGLProgram(
	gl: WebGLRenderingContext | WebGL2RenderingContext,
	vertexShader: WebGLShader,
	fragmentShader: WebGLShader,
): WebGLProgram {
	const program = gl.createProgram();
	if (!program) {
		throw new Error("Failed to create WebGL program object.");
	}

	gl.attachShader(program, vertexShader);
	gl.attachShader(program, fragmentShader);
	gl.linkProgram(program);

	const success = gl.getProgramParameter(program, gl.LINK_STATUS);
	if (!success) {
		const infoLog = gl.getProgramInfoLog(program);
		gl.deleteProgram(program);
		throw new Error(`Failed to link WebGL program: ${infoLog}`);
	}

	return program;
}
