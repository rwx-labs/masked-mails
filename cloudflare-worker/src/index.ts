export default {
	async email(message: ForwardableEmailMessage, env: Env, _ctx: ExecutionContext) {
		const headers = {
			"Content-Type": "application/json",
			"Authorization": `Token ${env.API_TOKEN}`
		};
		const process_start = new Date();
		const payload = {
			from: message.from,
			to: message.to,
			raw: message.raw,
			raw_size: message.rawSize,
			headers: message.headers,
			process_start
		};

		fetch(`${env.SERVICE_URL}/api/v1/ingress`, {
			headers,
			method: "POST",
			body: JSON.stringify(payload),
		});
	}
}
