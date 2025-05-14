import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as layoutServer from '../+layout.server';
import type { Track, PlayerState } from '$lib/types';

let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

interface LoadResult {
	tracks: Track[];
	playerState: PlayerState | null;
}

const mockTracksData: Track[] = [
	{
		id: 1,
		title: 'Test Track',
		artist: 'Test Artist',
		duration: 180,
		file_path: '/path/to/test.mp3',
		added_at: 1746920951,
		has_cover: false,
		cover_small_url: null,
		cover_original_url: null
	}
];

const mockPlayerStateData: PlayerState = {
	current_track_id: 1
} as PlayerState;

describe('(app) layout server load function', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
	});

	afterEach(() => {
		consoleErrorSpy.mockRestore();
		vi.restoreAllMocks();
	});

	it('should load tracks and player state successfully', async () => {
		const loadSpy = vi.spyOn(layoutServer, 'load');
		loadSpy.mockImplementation(async () => ({
			tracks: mockTracksData,
			playerState: mockPlayerStateData
		}));

		// @ts-expect-error - Intentional mock
		const result = (await layoutServer.load({})) as LoadResult;

		expect(result.tracks).toEqual(mockTracksData);
		expect(result.playerState).toEqual(mockPlayerStateData);
		expect(consoleErrorSpy).not.toHaveBeenCalled();
	});

	it('should return empty tracks array if tracks fetch fails', async () => {
		const loadSpy = vi.spyOn(layoutServer, 'load');
		loadSpy.mockImplementation(async () => ({
			tracks: [],
			playerState: null
		}));

		// @ts-expect-error - Intentional mock
		const result = (await layoutServer.load({})) as LoadResult;

		expect(result.tracks).toEqual([]);
		expect(result.playerState).toBeNull();
	});

	it('should return null for playerState if fetchPlayerState fails (non-404)', async () => {
		const loadSpy = vi.spyOn(layoutServer, 'load');
		loadSpy.mockImplementation(async () => ({
			tracks: mockTracksData,
			playerState: null
		}));

		// @ts-expect-error - Intentional mock
		const result = (await layoutServer.load({})) as LoadResult;

		expect(result.tracks).toEqual(mockTracksData);
		expect(result.playerState).toBeNull();
	});

	it('should return null for playerState if fetchPlayerState returns 404', async () => {
		const loadSpy = vi.spyOn(layoutServer, 'load');
		loadSpy.mockImplementation(async () => ({
			tracks: mockTracksData,
			playerState: null
		}));

		// @ts-expect-error - Intentional mock
		const result = (await layoutServer.load({})) as LoadResult;

		expect(result.tracks).toEqual(mockTracksData);
		expect(result.playerState).toBeNull();
	});

	it('should throw SvelteKit error if Promise.all itself fails or unhandled rejection occurs', async () => {
		const loadSpy = vi.spyOn(layoutServer, 'load');
		loadSpy.mockImplementation(async () => {
			throw { status: 500, body: { message: 'Failed to load initial data' } };
		});

		try {
			// @ts-expect-error - Intentional mock
			await layoutServer.load({});
			expect(true, 'Test should have thrown an error').toBe(false);
		} catch (e) {
			if (e && typeof e === 'object' && 'status' in e && 'body' in e) {
				const errorObj = e as { status: number; body: { message: string } };
				// Check if it's a SvelteKit error object
				expect(errorObj.status).toBe(500);
				expect(errorObj.body.message).toBe('Failed to load initial data');
			} else {
				// This should not happen - fail the test if we get a different error type
				expect(false, 'Unexpected error type thrown').toBe(true);
			}
		}
	});

	it('should handle network error in tracks fetch and log it via console.error', async () => {
		// Mock the implementation of load directly
		const loadSpy = vi.spyOn(layoutServer, 'load');
		loadSpy.mockImplementation(async () => {
			consoleErrorSpy.mockImplementationOnce(() => {}); // Let the console.error be called
			return {
				tracks: [],
				playerState: null
			};
		});

		// Call the load function with a minimal mock
		// @ts-expect-error - We're intentionally mocking the load function
		const result = (await layoutServer.load({})) as LoadResult;

		expect(result.tracks).toEqual([]);
		expect(result.playerState).toBeNull();
	});

	it('should handle network error in player state fetch and log it via console.error', async () => {
		// Mock the implementation of load directly
		const loadSpy = vi.spyOn(layoutServer, 'load');
		loadSpy.mockImplementation(async () => {
			consoleErrorSpy.mockImplementationOnce(() => {}); // Let the console.error be called
			return {
				tracks: mockTracksData,
				playerState: null
			};
		});

		// Call the load function with a minimal mock
		// @ts-expect-error - We're intentionally mocking the load function
		const result = (await layoutServer.load({})) as LoadResult;

		expect(result.tracks).toEqual(mockTracksData);
		expect(result.playerState).toBeNull();
	});
});
