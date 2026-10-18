const ALPHABET = 'abcdefghjkmnpqrstuvwxyz23456789';
const GROUPS = 3;
const GROUP_SIZE = 4;

export function generatePassword() {
	const values = crypto.getRandomValues(new Uint32Array(GROUPS * GROUP_SIZE));
	const characters = Array.from(values, (value) => ALPHABET[value % ALPHABET.length]);
	return Array.from({ length: GROUPS }, (_, group) =>
		characters.slice(group * GROUP_SIZE, (group + 1) * GROUP_SIZE).join('')
	).join('-');
}

const escape = (value: string) => value.replace(/([\;,:"])/g, '\\$1');

export function joinCode(ssid: string, password: string | null, hidden: boolean) {
	const type = password ? 'WPA' : 'nopass';
	const secret = password ? `P:${escape(password)};` : '';
	return `WIFI:T:${type};S:${escape(ssid)};${secret}${hidden ? 'H:true;' : ''};`;
}
