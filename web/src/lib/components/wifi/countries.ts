const codes = [
	'AT',
	'AU',
	'BE',
	'BR',
	'CA',
	'CH',
	'CN',
	'CZ',
	'DE',
	'DK',
	'ES',
	'FI',
	'FR',
	'GB',
	'GR',
	'HR',
	'HU',
	'IE',
	'IL',
	'IN',
	'IT',
	'JP',
	'KR',
	'MX',
	'NL',
	'NO',
	'NZ',
	'PL',
	'PT',
	'RO',
	'RS',
	'SE',
	'SG',
	'SI',
	'SK',
	'TR',
	'TW',
	'UA',
	'US',
	'ZA'
];

const names = new Intl.DisplayNames(['en'], { type: 'region' });

export const countries = codes
	.map((code) => ({ value: code, label: names.of(code) ?? code }))
	.sort((a, b) => a.label.localeCompare(b.label));
