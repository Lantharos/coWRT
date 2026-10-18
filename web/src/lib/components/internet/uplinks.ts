import { Cable, Globe, Smartphone } from '@lucide/svelte';
import type { Uplink, UplinkKind } from '#lib/api/types.js';

export const uplinkIcon: Record<UplinkKind, typeof Globe> = {
	cable: Cable,
	phone: Smartphone,
	other: Globe
};

export function uplinkState(uplink: Uplink) {
	if (uplink.inUse && uplink.online) return 'In use';
	if (uplink.inUse) return 'In use, no internet';
	if (!uplink.connected) return uplink.kind === 'phone' ? 'Not plugged in' : 'Not connected';
	if (!uplink.online) return 'No internet';
	return 'Standing by';
}
