export type Level = 'off' | 'threats' | 'ads';

export const levelLabel: Record<Level, string> = {
	off: 'Off',
	threats: 'Threats',
	ads: 'Ads and threats'
};
