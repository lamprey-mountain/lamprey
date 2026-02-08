export async function fetchWithRetry<T>(
	fn: () => Promise<{ data?: T; error?: any; response: Response }>,
	retries = 3,
	delay = 1000,
): Promise<T> {
	for (let i = 0; i < retries; i++) {
		let res;
		try {
			res = await fn();
		} catch (e) {
			if (i === retries - 1) throw e;
			await new Promise((r) => setTimeout(r, delay * Math.pow(2, i)));
			continue;
		}

		const { data, error, response } = res;
		if (!error) return data!;

		if (response.status < 500 && response.status !== 429) {
			throw error;
		}

		if (i === retries - 1) throw error;
		await new Promise((r) => setTimeout(r, delay * Math.pow(2, i)));
	}

	throw new Error("too many errors?");
}
