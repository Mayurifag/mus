import type { Handle } from '@sveltejs/kit';

// Mock data for API responses
const mockTracks = [
	{
		id: 1,
		title: 'Test Track',
		artist: 'Test Artist',
		duration: 180,
		file_path: '/path/to/track.mp3',
		added_at: Date.now(),
		has_cover: true,
		cover_small_url: '/api/v1/tracks/1/covers/small.webp',
		cover_original_url: '/api/v1/tracks/1/covers/original.webp'
	}
];

const mockPlayerState = {
	current_track_id: 1,
	progress_seconds: 0,
	volume_level: 0.8,
	is_muted: false
};

export const handle: Handle = async ({ event, resolve }) => {
	// Only mock API responses during testing
	if (import.meta.env.MODE === 'test' || import.meta.env.VITE_MOCK_API === 'true') {
		const url = new URL(event.request.url);

		// Handle API routes for mocking
		if (url.pathname.startsWith('/api/v1/')) {
			// Mock tracks endpoint
			if (url.pathname === '/api/v1/tracks') {
				return new Response(JSON.stringify(mockTracks), {
					headers: { 'Content-Type': 'application/json' }
				});
			}

			// Mock player state endpoint
			if (url.pathname === '/api/v1/player/state') {
				return new Response(JSON.stringify(mockPlayerState), {
					headers: { 'Content-Type': 'application/json' }
				});
			}

			// Mock scan endpoint
			if (url.pathname === '/api/v1/scan' && event.request.method === 'POST') {
				return new Response(
					JSON.stringify({ success: true, message: 'Scan started successfully' }),
					{
						headers: { 'Content-Type': 'application/json' }
					}
				);
			}
		}
	}

	// For all other requests, proceed normally
	return await resolve(event);
};
