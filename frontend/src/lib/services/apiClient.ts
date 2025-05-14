import type { Track, PlayerState } from '$lib/types';

const API_BASE_URL = '/api/v1';

export async function fetchTracks(): Promise<Track[]> {
	try {
		const response = await fetch(`${API_BASE_URL}/tracks`);

		if (!response.ok) {
			throw new Error(`Failed to fetch tracks: ${response.status} ${response.statusText}`);
		}

		return await response.json();
	} catch (error) {
		console.error('Error fetching tracks:', error);
		return [];
	}
}

export async function fetchPlayerState(): Promise<PlayerState | null> {
	try {
		const response = await fetch(`${API_BASE_URL}/player/state`);

		if (!response.ok) {
			if (response.status === 404) {
				// No state exists yet, return null
				return null;
			}
			throw new Error(`Failed to fetch player state: ${response.status} ${response.statusText}`);
		}

		return await response.json();
	} catch (error) {
		console.error('Error fetching player state:', error);
		return null;
	}
}

export async function savePlayerState(state: PlayerState): Promise<PlayerState | null> {
	try {
		const response = await fetch(`${API_BASE_URL}/player/state`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify(state)
		});

		if (!response.ok) {
			throw new Error(`Failed to save player state: ${response.status} ${response.statusText}`);
		}

		return await response.json();
	} catch (error) {
		console.error('Error saving player state:', error);
		return null;
	}
}

export async function triggerScan(): Promise<{ success: boolean; message: string }> {
	try {
		const response = await fetch(`${API_BASE_URL}/scan`, {
			method: 'POST'
		});

		if (!response.ok) {
			throw new Error(`Failed to trigger scan: ${response.status} ${response.statusText}`);
		}

		return await response.json();
	} catch (error) {
		console.error('Error triggering scan:', error);
		return {
			success: false,
			message: error instanceof Error ? error.message : 'Unknown error'
		};
	}
}
