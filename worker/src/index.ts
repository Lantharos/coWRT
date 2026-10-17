import { connect } from 'cloudflare:sockets';

const TIMEOUT_MS = 4000;
const MAX_PORTS = 10;

type Probe = { port: number; open: boolean };

async function probe(host: string, port: number): Promise<Probe> {
	const socket = connect({ hostname: host, port });
	const timeout = new Promise<false>((resolve) => setTimeout(() => resolve(false), TIMEOUT_MS));
	const opened = socket.opened.then(() => true).catch(() => false);
	const open = await Promise.race([opened, timeout]);
	socket.close().catch(() => {});
	return { port, open };
}

function json(body: unknown, status = 200) {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'content-type': 'application/json', 'cache-control': 'no-store' }
	});
}

export default {
	async fetch(request): Promise<Response> {
		const url = new URL(request.url);
		const caller = request.headers.get('cf-connecting-ip');
		if (!caller) return json({ error: 'Unknown caller' }, 400);

		if (url.pathname === '/ip' && request.method === 'GET') {
			return json({ ip: caller });
		}

		if (url.pathname === '/check' && request.method === 'POST') {
			const body = (await request.json().catch(() => null)) as { ports?: unknown } | null;
			const ports = Array.isArray(body?.ports) ? body.ports : [];
			const valid = ports.filter(
				(port): port is number => Number.isInteger(port) && port > 0 && port < 65536
			);
			if (!valid.length || valid.length > MAX_PORTS) {
				return json({ error: `Send between 1 and ${MAX_PORTS} ports` }, 400);
			}
			const results = await Promise.all(valid.map((port) => probe(caller, port)));
			return json({ ip: caller, results });
		}

		return json({ error: 'Not found' }, 404);
	}
} satisfies ExportedHandler;
