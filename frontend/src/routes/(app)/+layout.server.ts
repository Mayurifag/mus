import { error } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';
import type { Track, PlayerState } from '$lib/types';

const API_BASE_URL = '/api/v1';

export const load: LayoutServerLoad = async ({ fetch: svelteKitFetch }) => {
	try {
		const tracksPromise = svelteKitFetch(`${API_BASE_URL}/tracks`)
			.then(async (res) => {
				if (!res.ok) {
					// Log the error response text for better debugging
					const errorText = await res.text();
					console.error(
						`Failed to fetch tracks: ${res.status} ${res.statusText}. Response: ${errorText}`
					);
					return [] as Track[]; // Return empty on error
				}
				return res.json() as Promise<Track[]>;
			})
			.catch((err) => {
				console.error('Network or other error fetching tracks:', err);
				return [] as Track[]; // Ensure return type matches, return empty on error
			});

		const playerStatePromise = svelteKitFetch(`${API_BASE_URL}/player/state`)
			.then(async (res) => {
				if (!res.ok) {
					if (res.status === 404) {
						// This is an expected case if player state doesn't exist yet
						return null;
					}
					// Log the error response text for better debugging
					const errorText = await res.text();
					console.error(
						`Failed to fetch player state: ${res.status} ${res.statusText}. Response: ${errorText}`
					);
					return null; // Return null on other errors
				}
				const contentType = res.headers.get('content-type');
				if (contentType && contentType.includes('application/json')) {
					return res.json() as Promise<PlayerState | null>;
				}
				// Handle cases where response might not be JSON (e.g. empty 204 or unexpected format)
				console.warn(
					`Received non-JSON response for player state: ${res.status} ${res.statusText}`
				);
				return null;
			})
			.catch((err) => {
				console.error('Network or other error fetching player state:', err);
				return null as PlayerState | null; // Ensure return type matches, return null on error
			});

		const [tracks, playerState] = await Promise.all([tracksPromise, playerStatePromise]);

		return {
			tracks,
			playerState
		};
	} catch (err: unknown) {
		// Catching unknown and then checking instance if needed
		console.error('Error in load function (Promise.all or unexpected):', err);
		// SvelteKit's error function expects a specific structure for the second argument if it's an object
		throw error(500, { message: 'Failed to load initial data' });
	}
};
