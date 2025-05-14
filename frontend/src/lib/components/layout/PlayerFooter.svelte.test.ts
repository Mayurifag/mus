import { vi } from 'vitest';

// Mock playerStore
vi.mock('$lib/stores/playerStore', () => {
	const mockStore = {
		subscribe: vi.fn(),
		togglePlayPause: vi.fn(),
		setCurrentTime: vi.fn(),
		setVolume: vi.fn(),
		toggleMute: vi.fn()
	};

	return {
		playerStore: mockStore
	};
});

// Mock trackStore
vi.mock('$lib/stores/trackStore', () => {
	const mockStore = {
		subscribe: vi.fn(),
		nextTrack: vi.fn(),
		previousTrack: vi.fn()
	};

	return {
		trackStore: mockStore
	};
});

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import PlayerFooter from './PlayerFooter.svelte';
import { playerStore } from '$lib/stores/playerStore';
import { trackStore } from '$lib/stores/trackStore';
import type { PlayerStoreState } from '$lib/stores/playerStore';

interface MockSubscriberCallback {
	(state: PlayerStoreState): void;
}

// Helper to create a mock store state
function createMockStore() {
	let subscribers: MockSubscriberCallback[] = [];

	const subscribe = vi.fn((callback: MockSubscriberCallback) => {
		subscribers.push(callback);
		callback(currentState);

		return () => {
			subscribers = subscribers.filter((cb) => cb !== callback);
		};
	});

	// Default state
	let currentState: PlayerStoreState = {
		currentTrack: {
			id: 1,
			title: 'Test Track',
			artist: 'Test Artist',
			duration: 180,
			file_path: '/path/to/file.mp3',
			added_at: 1615478400,
			has_cover: true,
			cover_small_url: '/api/v1/tracks/1/covers/small.webp',
			cover_original_url: '/api/v1/tracks/1/covers/original.webp'
		},
		isPlaying: false,
		currentTime: 30,
		duration: 180,
		volume: 0.5,
		isMuted: false
	};

	// Update all subscribers
	const updateSubscribers = () => {
		subscribers.forEach((callback) => callback(currentState));
	};

	// Method to update state for tests
	const updateState = (newState: Partial<PlayerStoreState>) => {
		currentState = { ...currentState, ...newState };
		updateSubscribers();
		return currentState; // Return for easier testing
	};

	return { subscribe, updateState };
}

describe('PlayerFooter component', () => {
	let mockPlayerStore: ReturnType<typeof createMockStore>;
	let mockTrackStore: {
		subscribe: ReturnType<typeof vi.fn>;
		nextTrack: ReturnType<typeof vi.fn>;
		previousTrack: ReturnType<typeof vi.fn>;
	};

	beforeEach(() => {
		// Set up mocks
		mockPlayerStore = createMockStore();
		mockTrackStore = {
			subscribe: vi.fn(),
			nextTrack: vi.fn(),
			previousTrack: vi.fn()
		};

		// Reset mocks
		vi.mocked(playerStore.togglePlayPause).mockClear();
		vi.mocked(playerStore.setCurrentTime).mockClear();
		vi.mocked(playerStore.setVolume).mockClear();
		vi.mocked(playerStore.toggleMute).mockClear();
		vi.mocked(trackStore.nextTrack).mockClear();
		vi.mocked(trackStore.previousTrack).mockClear();

		// Override mock implementations
		vi.mocked(playerStore.subscribe).mockImplementation(mockPlayerStore.subscribe);
		vi.mocked(trackStore.subscribe).mockImplementation(mockTrackStore.subscribe);
	});

	afterEach(() => {
		cleanup();
	});

	it('renders track info when a track is loaded', () => {
		render(PlayerFooter);

		expect(screen.getByText('Test Track')).toBeInTheDocument();
		expect(screen.getByText('Test Artist')).toBeInTheDocument();
		expect(screen.getByText('0:30')).toBeInTheDocument(); // Current time
		expect(screen.getByText('3:00')).toBeInTheDocument(); // Duration
	});

	it('shows Play button when paused', () => {
		render(PlayerFooter);

		// Find the button with the Play icon by its aria-label
		const playButton = screen.getByRole('button', { name: 'Play' });
		expect(playButton).toBeInTheDocument();
		expect(playButton.innerHTML).toContain('lucide-play');
	});

	it('shows Pause button when playing', async () => {
		render(PlayerFooter);

		// Update state to playing
		mockPlayerStore.updateState({ isPlaying: true });

		// Re-render component with new state
		cleanup();
		render(PlayerFooter);

		// Find the button with the Pause icon by its aria-label
		await waitFor(() => {
			const pauseButton = screen.getByRole('button', { name: 'Pause' });
			expect(pauseButton).toBeInTheDocument();
			expect(pauseButton.innerHTML).toContain('lucide-pause');
		});
	});

	it('calls togglePlayPause when play/pause button is clicked', async () => {
		render(PlayerFooter);

		// Find play button by its aria-label
		const playButton = screen.getByRole('button', { name: 'Play' });
		await fireEvent.click(playButton);

		expect(playerStore.togglePlayPause).toHaveBeenCalled();
	});

	it('calls previousTrack when previous button is clicked', async () => {
		render(PlayerFooter);

		// Find the button with the aria-label for previous track
		const previousButton = screen.getByRole('button', { name: 'Previous Track' });
		await fireEvent.click(previousButton);

		expect(trackStore.previousTrack).toHaveBeenCalled();
	});

	it('calls nextTrack when next button is clicked', async () => {
		render(PlayerFooter);

		// Find the button with the aria-label for next track
		const nextButton = screen.getByRole('button', { name: 'Next Track' });
		await fireEvent.click(nextButton);

		expect(trackStore.nextTrack).toHaveBeenCalled();
	});

	it('calls toggleMute when mute button is clicked', async () => {
		render(PlayerFooter);

		// Find the button with the mute aria-label
		const muteButton = screen.getByRole('button', { name: 'Mute' });
		await fireEvent.click(muteButton);

		expect(playerStore.toggleMute).toHaveBeenCalled();
	});

	it('displays VolumeX icon when muted', async () => {
		render(PlayerFooter);

		// Update the state to muted
		mockPlayerStore.updateState({ isMuted: true });

		// Re-render component with new state
		cleanup();
		render(PlayerFooter);

		// We should check for the Unmute button label when muted
		await waitFor(() => {
			const unmuteButton = screen.getByRole('button', { name: 'Unmute' });
			expect(unmuteButton).toBeInTheDocument();
			expect(unmuteButton.innerHTML).toContain('lucide-volume-x');
		});
	});

	// Fix tests that were previously skipped
	it('displays "No Track" when no track is loaded', async () => {
		render(PlayerFooter);

		// Update state with null track - don't cleanup and rerender
		mockPlayerStore.updateState({ currentTrack: null });

		// Wait for the component to update
		await waitFor(() => {
			const noTrackElements = screen.getAllByText('No Track');
			expect(noTrackElements.length).toBe(1);
		});
	});

	it('displays "Not Playing" message when no track is loaded', async () => {
		render(PlayerFooter);

		// Update the state with null track - don't cleanup and rerender
		mockPlayerStore.updateState({ currentTrack: null });

		// Wait for the component to update
		await waitFor(() => {
			const notPlayingElement = screen.getByText('Not Playing');
			expect(notPlayingElement).toBeInTheDocument();
		});
	});
});
