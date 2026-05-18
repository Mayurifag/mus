# Roadmap

Primary goal: keep mus as a small, single-container music player/downloader while product work stays focused on real daily-use gaps.

## Product Backlog

### Player And UI

- Make idle sliders less visually saturated while keeping hover/play states prominent.
- Add hotkeys for player controls and show hotkey hints on hover.
- Add marquee/overflow behavior for long text where truncation hurts usability.
- Render a play button under album cover in the track list.
- Make shuffled history tracks playable from the playback timeline.
- Consider a `Play next` action.

### Library Management

- Add sorting by different track fields and directions.
- Add fast search that can later search local library, YouTube, VK, and other sources.
- Add optional track download/export from the edit window if there is a real use case.
- Define Artist, Album, and Playlist entities later.
- Support many-to-many track/artist/album/playlist relationships.
- Promote parsed artists into unique artist records and keep aliases such as `Тату` / `t.A.T.u.`.

### Download Sources

- Add VK search/download support.
- Add zaycev.net search/download support for uncensored Russian music variants.
