import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { load } from '../+layout.server';
import type { Track, PlayerState } from '$lib/types';
import { getApiBaseUrl } from '$lib/services/apiClient';

// Mock the API base URL function for tests
vi.mock('$lib/services/apiClient', () => ({
	getApiBaseUrl: () => 'http://localhost:8000/api/v1'
}));

// Get the mocked base URL to use in tests
const API_BASE_URL = getApiBaseUrl();

// Spy on console.error
let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

// Define the return type for the load function
interface LoadReturn {
	tracks: Track[];
	playerState: PlayerState | null;
}

describe('(app) layout server load function', () => {
	let mockSvelteKitFetch: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		vi.clearAllMocks();
		mockSvelteKitFetch = vi.fn();
		// Suppress console.error for these specific tests by default in this suite
		consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
	});

	afterEach(() => {
		// Restore console.error
		consoleErrorSpy.mockRestore();
	});

	it('should load tracks and player state successfully', async () => {
		const mockTracksData: Track[] = [{ id: 1, title: 'Test Track' } as Track];
		const mockPlayerStateData: PlayerState = { current_track_id: 1 } as PlayerState;

		mockSvelteKitFetch
			.mockResolvedValueOnce(
				Promise.resolve({
					ok: true,
					json: () => Promise.resolve(mockTracksData),
					text: () => Promise.resolve(JSON.stringify(mockTracksData)),
					headers: new Headers({ 'Content-Type': 'application/json' })
				} as unknown as Response)
			)
			.mockResolvedValueOnce(
				Promise.resolve({
					ok: true,
					json: () => Promise.resolve(mockPlayerStateData),
					text: () => Promise.resolve(JSON.stringify(mockPlayerStateData)),
					headers: new Headers({ 'Content-Type': 'application/json' })
				} as unknown as Response)
			);

		// Using unknown instead of any, then cast to the expected return type
		const result = (await load({ fetch: mockSvelteKitFetch } as unknown as Parameters<
			typeof load
		>[0])) as LoadReturn;

		expect(mockSvelteKitFetch).toHaveBeenCalledWith(`${API_BASE_URL}/tracks`);
		expect(mockSvelteKitFetch).toHaveBeenCalledWith(`${API_BASE_URL}/player/state`);
		expect(result.tracks).toEqual(mockTracksData);
		expect(result.playerState).toEqual(mockPlayerStateData);
		expect(consoleErrorSpy).not.toHaveBeenCalled();
	});

	it('should return empty tracks array if fetchTracks fails', async () => {
		mockSvelteKitFetch
			.mockResolvedValueOnce(
				// For tracks - error
				Promise.resolve({
					ok: false,
					status: 500,
					statusText: 'Server Error',
					text: () => Promise.resolve('Server Error Details for tracks'),
					headers: new Headers()
				} as unknown as Response)
			)
			.mockResolvedValueOnce(
				// For player state - success
				Promise.resolve({
					ok: true,
					json: () => Promise.resolve(null),
					text: () => Promise.resolve('null'),
					headers: new Headers({ 'Content-Type': 'application/json' })
				} as unknown as Response)
			);

		const result = (await load({ fetch: mockSvelteKitFetch } as unknown as Parameters<
			typeof load
		>[0])) as LoadReturn;
		expect(result.tracks).toEqual([]);
		expect(result.playerState).toBeNull();
		expect(consoleErrorSpy).toHaveBeenCalledWith(
			'Failed to fetch tracks: 500 Server Error. Response: Server Error Details for tracks'
		);
	});

	it('should return null for playerState if fetchPlayerState fails (non-404)', async () => {
		const mockTracksData: Track[] = [{ id: 1, title: 'Test Track' } as Track];
		mockSvelteKitFetch
			.mockResolvedValueOnce(
				// For tracks - success
				Promise.resolve({
					ok: true,
					json: () => Promise.resolve(mockTracksData),
					text: () => Promise.resolve(JSON.stringify(mockTracksData)),
					headers: new Headers({ 'Content-Type': 'application/json' })
				} as unknown as Response)
			)
			.mockResolvedValueOnce(
				// For player state - error
				Promise.resolve({
					ok: false,
					status: 500,
					statusText: 'Server Error',
					text: () => Promise.resolve('Server Error Details for player state'),
					headers: new Headers()
				} as unknown as Response)
			);

		const result = (await load({ fetch: mockSvelteKitFetch } as unknown as Parameters<
			typeof load
		>[0])) as LoadReturn;
		expect(result.tracks).toEqual(mockTracksData);
		expect(result.playerState).toBeNull();
		expect(consoleErrorSpy).toHaveBeenCalledWith(
			'Failed to fetch player state: 500 Server Error. Response: Server Error Details for player state'
		);
	});

	it('should return null for playerState if fetchPlayerState returns 404', async () => {
		const mockTracksData: Track[] = [{ id: 1, title: 'Test Track' } as Track];
		mockSvelteKitFetch
			.mockResolvedValueOnce(
				// For tracks - success
				Promise.resolve({
					ok: true,
					json: () => Promise.resolve(mockTracksData),
					text: () => Promise.resolve(JSON.stringify(mockTracksData)),
					headers: new Headers({ 'Content-Type': 'application/json' })
				} as unknown as Response)
			)
			.mockResolvedValueOnce(
				// For player state - 404
				Promise.resolve({
					ok: false,
					status: 404,
					statusText: 'Not Found',
					text: () => Promise.resolve('Not Found Details'),
					headers: new Headers()
				} as unknown as Response)
			);

		const result = (await load({ fetch: mockSvelteKitFetch } as unknown as Parameters<
			typeof load
		>[0])) as LoadReturn;
		expect(result.tracks).toEqual(mockTracksData);
		expect(result.playerState).toBeNull();
		// 404 for player state is handled gracefully within the load function and not logged as an error by console.error
		// because it's an expected "not found" scenario.
		expect(consoleErrorSpy).not.toHaveBeenCalled();
	});

	it('should throw SvelteKit error if Promise.all itself fails or unhandled rejection occurs', async () => {
		// This test simulates a scenario where an error occurs outside the individual .catch()
		// blocks of the fetch promises, for example, if Promise.all itself throws or an error
		// occurs in the logic after Promise.all but before returning.
		const originalPromiseAll = Promise.all;
		// Typescript safe way to mock Promise.all
		const mockPromiseAll = vi.fn().mockRejectedValue(new Error('Catastrophic Promise.all failure'));
		(Promise as unknown as { all: typeof mockPromiseAll }).all = mockPromiseAll;

		try {
			await load({ fetch: mockSvelteKitFetch } as unknown as Parameters<typeof load>[0]);
			// This line should not be reached if the error is thrown correctly
			expect(true, 'Test should have thrown an error').toBe(false);
		} catch (e) {
			// Type guard to check if it's a SvelteKit error object with the expected properties
			if (e && typeof e === 'object' && 'status' in e && 'body' in e) {
				const errorObj = e as { status: number; body: { message: string } };
				// Check if it's a SvelteKit error object
				expect(errorObj.status).toBe(500);
				expect(errorObj.body.message).toBe('Failed to load initial data');
				// The console.error inside load's main try-catch will be called
				expect(consoleErrorSpy).toHaveBeenCalledWith(
					'Error in load function (Promise.all or unexpected):',
					expect.any(Error) // The original 'Catastrophic Promise.all failure'
				);
			} else {
				// This should not happen - fail the test if we get a different error type
				expect(false, 'Unexpected error type thrown').toBe(true);
			}
		} finally {
			Promise.all = originalPromiseAll; // Restore original Promise.all
		}
	});

	it('should handle network error in fetchTracks and log it via console.error', async () => {
		mockSvelteKitFetch
			.mockImplementationOnce(() => Promise.reject(new Error('Tracks Network Error'))) // For tracks
			.mockResolvedValueOnce(
				// For player state - success
				Promise.resolve({
					ok: true,
					json: () => Promise.resolve(null),
					text: () => Promise.resolve('null'),
					headers: new Headers({ 'Content-Type': 'application/json' })
				} as unknown as Response)
			);

		const result = (await load({ fetch: mockSvelteKitFetch } as unknown as Parameters<
			typeof load
		>[0])) as LoadReturn;
		expect(result.tracks).toEqual([]);
		expect(result.playerState).toBeNull();
		expect(consoleErrorSpy).toHaveBeenCalledWith(
			'Network or other error fetching tracks:',
			expect.objectContaining({ message: 'Tracks Network Error' })
		);
	});

	it('should handle network error in fetchPlayerState and log it via console.error', async () => {
		const mockTracksData: Track[] = [{ id: 1, title: 'Test Track' } as Track];
		mockSvelteKitFetch
			.mockResolvedValueOnce(
				// For tracks - success
				Promise.resolve({
					ok: true,
					json: () => Promise.resolve(mockTracksData),
					text: () => Promise.resolve(JSON.stringify(mockTracksData)),
					headers: new Headers({ 'Content-Type': 'application/json' })
				} as unknown as Response)
			)
			.mockImplementationOnce(() => Promise.reject(new Error('Player State Network Error'))); // For player state

		const result = (await load({ fetch: mockSvelteKitFetch } as unknown as Parameters<
			typeof load
		>[0])) as LoadReturn;
		expect(result.tracks).toEqual(mockTracksData);
		expect(result.playerState).toBeNull();
		expect(consoleErrorSpy).toHaveBeenCalledWith(
			'Network or other error fetching player state:',
			expect.objectContaining({ message: 'Player State Network Error' })
		);
	});
});
