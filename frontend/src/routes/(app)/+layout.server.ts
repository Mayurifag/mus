import { error } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';
import { fetchTracks, fetchPlayerState } from '$lib/services/apiClient';

export const load: LayoutServerLoad = async () => {
	try {
		const [tracks, playerState] = await Promise.all([fetchTracks(), fetchPlayerState()]);

		return {
			tracks,
			playerState
		};
	} catch (err: unknown) {
		console.error('Error in load function (Promise.all or unexpected):', err);
		throw error(500, { message: 'Failed to load initial data' });
	}
};
