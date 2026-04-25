# Task 10: Audio Feedback Implementation

## Objective
Implement audio feedback (success/error sounds) when a gesture is matched or fails.

## Implementation Details

### Win32 Integration
- **WAV Support**: Used `PlaySoundW` from `windows` crate for standard waveform files.
- **MP3 & Other Formats**: Expanded support to MP3, MIDI, etc., using the Media Control Interface (MCI) via `mciSendStringW`.
- **Cargo Features**: Enabled `Win32_Media`, `Win32_Media_Audio`, and `Win32_Media_Multimedia`.
- **Performance**: Audio is played asynchronously (SND_ASYNC for WAV, non-blocking MCI commands for others).

### Components
1. **AudioPlayer (`src/audio/mod.rs`)**:
   - Manages audio configuration.
   - Provides `play_success(override_path: Option<&str>)` and `play_error()`.
   - Resolves relative paths against the QuickDraw config directory.
   - Handles different file extensions:
     - `.wav`: Uses `PlaySoundW`.
     - `.mp3`, etc.: Uses MCI commands (`open`, `play`, `close`).
   - Gracefully handles missing files by logging a debug message instead of erroring.

2. **Config Update (`src/config.rs`)**:
   - Made `get_config_dir()` public to allow path resolution in `AudioPlayer`.

3. **Pipeline Integration (`src/pipeline.rs`)**:
   - Added `AudioPlayer` to `Pipeline`.
   - Successful match: Plays success sound (respects per-gesture `sound` override).
   - Below threshold or No match: Plays error sound.

### Configuration
- `[audio].enabled`: Global toggle for audio feedback.
- `[audio].success`: Default success sound path.
- `[audio].error`: Default error sound path.
- Per-gesture `sound` property in gesture profiles can override the success sound.

## Verification
- `cargo check` passes.
- Audio plays asynchronously.
- Missing files do not cause crashes.
- `audio.enabled = false` correctly mutes all feedback.

---

## Addendum — Architect Review (2026-04-25)

All decisions accepted. One notable scope expansion beyond the original spec.

### Confirmed: PlaySoundW for WAV
Correct implementation. `SND_ASYNC` ensures it doesn't block the pipeline loop during gesture processing.

### Beyond spec: MCI for MP3/other formats
The agent added MCI (`mciSendStringW`) support for non-WAV formats (MP3, MIDI, etc.) — this was not in the original spec. It's a reasonable addition: MCI is a standard Win32 API and adds meaningful flexibility. No concerns with the approach, but note that MCI commands are more complex to manage (open/play/close sequence) compared to `PlaySoundW`. If audio ever becomes a source of bugs, this is where to look first.

### Confirmed: Path resolution via get_config_dir()
Resolving relative sound paths against `get_config_dir()` means portable mode works correctly — sounds placed next to the exe are found automatically.
