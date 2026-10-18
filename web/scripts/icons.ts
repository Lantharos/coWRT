// Renders the app icons from logo.svg: bun scripts/icons.ts
import { chromium } from 'playwright';

const root = new URL('../../', import.meta.url).pathname;
const logo = await Bun.file(`${root}logo.svg`).text();
const out = `${root}web/static/`;

const icons = [
	{ file: 'icon-192.png', size: 192, scale: 0.62 },
	{ file: 'icon-512.png', size: 512, scale: 0.62 },
	{ file: 'icon-maskable.png', size: 512, scale: 0.46 },
	{ file: 'apple-touch-icon.png', size: 180, scale: 0.6 }
];

const browser = await chromium.launch();
const page = await browser.newPage();
for (const { file, size, scale } of icons) {
	await page.setViewportSize({ width: size, height: size });
	const inner = Math.round(size * scale);
	await page.setContent(
		`<body style="margin:0;display:grid;place-items:center;width:${size}px;height:${size}px;background:#111111">
			<div style="width:${inner}px;height:${inner}px">${logo.replace('<svg ', '<svg style="width:100%;height:100%" ')}</div>
		</body>`
	);
	await page.screenshot({ path: `${out}${file}` });
}
await browser.close();
await Bun.write(`${out}favicon.svg`, logo);
console.log(`Saved to ${out}`);
