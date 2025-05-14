import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { fetchTracks, fetchPlayerState, savePlayerState, triggerScan } from './apiClient';
import type { Track, PlayerState } from '$lib/types';

// Define constant for the base URL used in tests
const API_URL = 'http://localhost:8000/api/v1';

// Mock global fetch
const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

// Spy on console.error
let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

describe('apiClient', () => {
	beforeEach(() => {
		mockFetch.mockClear();
		// Suppress console.error for these specific tests by default in this suite
		consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
	});

	afterEach(() => {
		// Restore console.error
		consoleErrorSpy.mockRestore();
		vi.resetAllMocks();
	});

	describe('fetchTracks', () => {
		it('should fetch tracks successfully', async () => {
			const mockTracks: Track[] = [
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

			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () => Promise.resolve(mockTracks)
			});

			const result = await fetchTracks();

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/tracks`);
			expect(result).toEqual(mockTracks);
			expect(consoleErrorSpy).not.toHaveBeenCalled();
		});

		it('should handle fetch errors and return empty array', async () => {
			mockFetch.mockRejectedValueOnce(new Error('Network error'));

			const result = await fetchTracks();

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/tracks`);
			expect(result).toEqual([]);
			expect(consoleErrorSpy).toHaveBeenCalledWith('Error fetching tracks:', expect.any(Error));
		});

		it('should handle non-200 responses and return empty array', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: false,
				status: 404,
				statusText: 'Not Found'
			});

			const result = await fetchTracks();

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/tracks`);
			expect(result).toEqual([]);
			expect(consoleErrorSpy).toHaveBeenCalledWith(
				'Error fetching tracks:',
				expect.objectContaining({ message: 'Failed to fetch tracks: 404 Not Found' })
			);
		});
	});

	describe('fetchPlayerState', () => {
		it('should fetch player state successfully', async () => {
			const mockPlayerState: PlayerState = {
				current_track_id: 1,
				progress_seconds: 30,
				volume_level: 0.8,
				is_muted: false
			};

			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () => Promise.resolve(mockPlayerState)
			});

			const result = await fetchPlayerState();

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/player/state`);
			expect(result).toEqual(mockPlayerState);
			expect(consoleErrorSpy).not.toHaveBeenCalled();
		});

		it('should return null for 404 responses', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: false,
				status: 404,
				statusText: 'Not Found'
			});

			const result = await fetchPlayerState();

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/player/state`);
			expect(result).toBeNull();
			// No console.error expected for 404 in fetchPlayerState as it's a handled case
			expect(consoleErrorSpy).not.toHaveBeenCalled();
		});

		it('should handle other non-200 errors and return null', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: false,
				status: 500,
				statusText: 'Server Error'
			});
			const result = await fetchPlayerState();
			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/player/state`);
			expect(result).toBeNull();
			expect(consoleErrorSpy).toHaveBeenCalledWith(
				'Error fetching player state:',
				expect.objectContaining({ message: 'Failed to fetch player state: 500 Server Error' })
			);
		});

		it('should handle network errors and return null', async () => {
			mockFetch.mockRejectedValueOnce(new Error('Network error'));

			const result = await fetchPlayerState();

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/player/state`);
			expect(result).toBeNull();
			expect(consoleErrorSpy).toHaveBeenCalledWith(
				'Error fetching player state:',
				expect.any(Error)
			);
		});
	});

	describe('savePlayerState', () => {
		it('should save player state successfully', async () => {
			const mockPlayerState: PlayerState = {
				current_track_id: 1,
				progress_seconds: 30,
				volume_level: 0.8,
				is_muted: false
			};

			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () => Promise.resolve(mockPlayerState)
			});

			const result = await savePlayerState(mockPlayerState);

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/player/state`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify(mockPlayerState)
			});
			expect(result).toEqual(mockPlayerState);
			expect(consoleErrorSpy).not.toHaveBeenCalled();
		});

		it('should handle errors and return null', async () => {
			const mockPlayerState: PlayerState = {
				current_track_id: 1,
				progress_seconds: 30,
				volume_level: 0.8,
				is_muted: false
			};

			mockFetch.mockRejectedValueOnce(new Error('Network error'));

			const result = await savePlayerState(mockPlayerState);

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/player/state`, expect.any(Object));
			expect(result).toBeNull();
			expect(consoleErrorSpy).toHaveBeenCalledWith('Error saving player state:', expect.any(Error));
		});
	});

	describe('triggerScan', () => {
		it('should trigger scan successfully', async () => {
			const mockResponse = { success: true, message: 'Scan started' };

			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () => Promise.resolve(mockResponse)
			});

			const result = await triggerScan();

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/scan`, {
				method: 'POST'
			});
			expect(result).toEqual(mockResponse);
			expect(consoleErrorSpy).not.toHaveBeenCalled();
		});

		it('should handle errors with appropriate response', async () => {
			const networkError = new Error('Network error');
			mockFetch.mockRejectedValueOnce(networkError);

			const result = await triggerScan();

			expect(mockFetch).toHaveBeenCalledWith(`${API_URL}/scan`, {
				method: 'POST'
			});
			expect(result).toEqual({
				success: false,
				message: 'Network error' // The message from the error object
			});
			expect(consoleErrorSpy).toHaveBeenCalledWith('Error triggering scan:', networkError);
		});
	});
});
