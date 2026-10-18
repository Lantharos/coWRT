// Captures pages of a running router: bun scripts/screenshots.ts [page | page>row title ...]
import { chromium } from 'playwright';

const base = process.env.COWRT_ROUTER ?? 'http://127.0.0.1:18080';
const password = process.env.COWRT_PASSWORD ?? 'cowrt';
const out = new URL('../../.screenshots/', import.meta.url).pathname;

const pages = ['overview', 'devices', 'wifi', 'networks', 'access', 'security', 'system'];
const wanted = process.argv.slice(2).length ? process.argv.slice(2) : pages;
const viewports = {
	desktop: { width: 1440, height: 900 },
	phone: { width: 390, height: 844 }
};

const browser = await chromium.launch();
for (const [label, viewport] of Object.entries(viewports)) {
	const context = await browser.newContext({ viewport, deviceScaleFactor: 2, colorScheme: 'dark' });
	const page = await context.newPage();
	await page.goto(`${base}/login`);
	await page.fill('input[type=password]', password);
	await page.press('input[type=password]', 'Enter');
	await page.waitForURL(`${base}/`);
	for (const target of wanted) {
		const [name, row] = target.split('>');
		await page.goto(`${base}/${name === 'overview' ? '' : name}`);
		await page.waitForLoadState('networkidle').catch(() => {});
		await page.waitForTimeout(2500);
		if (row) {
			await page.getByRole('button', { name: row }).first().click();
			await page.waitForTimeout(800);
		}
		const file = target
			.replace('>', '-')
			.replace(/[^a-z0-9-]+/gi, '-')
			.toLowerCase();
		await page.screenshot({ path: `${out}${file}-${label}.png`, fullPage: !row });
	}
	await context.close();
}
await browser.close();
console.log(`Saved to ${out}`);
