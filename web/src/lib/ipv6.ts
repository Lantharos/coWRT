const isGlobal = (prefix: string) => /^[23]/.test(prefix);

export function mainPrefix(prefixes: string[]) {
	return prefixes.find(isGlobal) ?? prefixes[0] ?? null;
}
