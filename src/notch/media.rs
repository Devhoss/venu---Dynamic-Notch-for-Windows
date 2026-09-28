//! Reads the system's "Now Playing" session — whichever app currently owns
//! Windows' System Media Transport Controls — on a dedicated background
//! thread, and publishes a snapshot the painter can read without blocking.
//!
//! The WinRT calls here block on `.get()` rather than awaiting. That is safe
//! only because this thread does nothing else: no message pump, no UI, so a
//! stalled wait starves nothing but itself.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use windows::core::Interface;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Storage::Streams::{DataReader, IInputStream, IRandomAccessStreamReference};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

use super::volume::{self, VolumeState};

const POLL_INTERVAL: Duration = Duration::from_millis(900);
/// Album art is a thumbnail, never anything close to this; a stream this big
/// is a sign to bail rather than block on reading it.
const MAX_ART_BYTES: u64 = 8 * 1024 * 1024;

/// What the painter reads each frame. Cheap to clone — art bytes live behind
/// an `Arc` so an unchanged frame costs nothing to hand back.
#[derive(Clone, Default)]
pub struct NowPlaying {
    pub has_session: bool,
    pub playing: bool,
    pub title: String,
    pub artist: String,
    pub art: Option<Arc<[u8]>>,
    /// Bumped whenever `art` changes, so the painter's decode cache knows to
    /// re-decode without hashing or comparing the byte slice itself.
    pub art_generation: u64,
    /// Master output volume for the default render endpoint. Read on the same
    /// 900 ms cycle as everything else above, on the same thread — volume adds
    /// no thread, no timer and no wakeups of its own.
    pub volume: VolumeState,
    /// False when there is no usable output device (unplugged, or audio
    /// services stopped). The control hides itself rather than showing a
    /// stale number from a device that is gone.
    pub has_volume: bool,
    /// True while the user is dragging the volume slider. The poller keeps
    /// reading, but the published value stops overriding what the drag is
    /// doing — otherwise the 900 ms read would yank the handle backwards
    /// under the cursor mid-gesture.
    pub volume_dragging: bool,
    /// True from the moment a volume write is queued until the poller has
    /// actually applied it.
    ///
    /// This is a separate flag from `volume_dragging` because the two guards
    /// cover different windows of time. The drag ends when the button comes
    /// up, but the write is still sitting in the command queue at that point:
    /// `poll_once` reads Windows *before* draining it, so it can observe the
    /// pre-write value and mistake it for the user having changed the volume
    /// elsewhere. Without this flag that stale read overwrites the handle,
    /// which then springs back once the next poll sees the truth — while the
    /// sound, having been set correctly all along, never moved.
    pub volume_pending: bool,
}

enum Command {
    PlayPause,
    Next,
    Previous,
    SetVolume(f32),
    SetMuted(bool),
}

/// Owns the background poller. Dropping this stops the thread — the notch
/// feature can be toggled off and on freely without leaking one.
pub struct MediaWatcher {
    snapshot: Arc<RwLock<NowPlaying>>,
    /// Bumped only when a poll actually found something different. The notch
    /// reads it every tick to decide whether the frame on screen is still
    /// current, so a poll that finds the same track playing must not look
    /// like a change.
    revision: Arc<AtomicU64>,
    tx: Sender<Command>,
    running: Arc<AtomicBool>,
}

impl MediaWatcher {
    pub fn spawn() -> Self {
        let snapshot = Arc::new(RwLock::new(NowPlaying::default()));
        let revision = Arc::new(AtomicU64::new(0));
        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::channel();

        let worker_snapshot = snapshot.clone();
        let worker_revision = revision.clone();
        let worker_running = running.clone();
        std::thread::spawn(move || run(worker_snapshot, worker_revision, worker_running, rx));

        Self {
            snapshot,
            revision,
            tx,
            running,
        }
    }

    pub fn snapshot(&self) -> NowPlaying {
        self.snapshot.read().clone()
    }

    /// Monotonic counter over every change to what is playing. Equal
    /// revisions mean the notch would draw the same Now Playing it drew last
    /// time, without having to clone and compare the snapshot.
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    pub fn play_pause(&self) {
        let _ = self.tx.send(Command::PlayPause);
    }

    pub fn next(&self) {
        let _ = self.tx.send(Command::Next);
    }

    pub fn previous(&self) {
        let _ = self.tx.send(Command::Previous);
    }

    /// Ask Windows for a new output level. Queued rather than called directly:
    /// Core Audio needs a COM apartment, and this thread is the one that has
    /// one. The UI thread must never make this call itself.
    pub fn set_volume(&self, level: f32) {
        {
            let mut current = self.snapshot.write();
            current.volume_pending = true;
        }
        let _ = self.tx.send(Command::SetVolume(volume::clamp(level)));
    }

    /// Queue a mute change, with the level left alone so unmuting restores
    /// what the user had.
    pub fn set_muted(&self, muted: bool) {
        let _ = self.tx.send(Command::SetMuted(muted));
    }

    /// Record an in-progress drag: publish the level the user is pointing at
    /// straight away, so the handle tracks the cursor instead of waiting out
    /// the poll, and mark the snapshot so the next poll does not fight the
    /// gesture.
    ///
    /// Only the snapshot is written here. The value still reaches Windows
    /// through [`MediaWatcher::set_volume`], which runs on the poller thread
    /// where COM is initialized — this method must not call into Core Audio.
    pub fn set_dragging(&self, level: f32) {
        {
            let mut current = self.snapshot.write();
            current.volume_dragging = true;
            current.volume_pending = true;
            if current.has_volume {
                current.volume.level = volume::clamp(level);
            }
        }
        self.revision.fetch_add(1, Ordering::Release);
    }

    /// End the gesture. The next poll re-reads the true value from Windows and
    /// takes over again.
    pub fn end_drag(&self) {
        {
            let mut current = self.snapshot.write();
            if !current.volume_dragging {
                return;
            }
            current.volume_dragging = false;
            // `volume_pending` is deliberately left alone: the write queued by
            // the last drag frame has not necessarily been applied yet, and
            // clearing it here is what let a stale read undo the drag.
        }
        self.revision.fetch_add(1, Ordering::Release);
    }

    /// Toggle mute, and report what it is now so the caller can draw it without
    /// waiting for the poll to come round.
    pub fn toggle_mute(&self) {
        let next = !self.snapshot.read().volume.muted;
        {
            let mut current = self.snapshot.write();
            current.volume.muted = next;
        }
        self.revision.fetch_add(1, Ordering::Release);
        self.set_muted(next);
    }
}

impl Drop for MediaWatcher {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn run(
    snapshot: Arc<RwLock<NowPlaying>>,
    revision: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
    rx: Receiver<Command>,
) {
    // COM apartment for this thread only; the overlay's own STA thread is
    // initialized separately and neither knows about the other.
    if unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_err() {
        return;
    }

    let mut art_generation: u64 = 0;
    let mut last_track_key: Option<String> = None;

    while running.load(Ordering::Relaxed) {
        poll_once(
            &snapshot,
            &revision,
            &mut art_generation,
            &mut last_track_key,
        );

        // Drain the whole queue each cycle, collapsing volume writes to the
        // last one.
        //
        // Handling a single command per cycle is what made the slider snap
        // back: a drag queues one write per mouse-move, so the poller spent
        // seconds replaying the drag's history one value per 900 ms, walking
        // the handle backwards through positions the user had already passed.
        // Only the newest value matters — every intermediate one is a value the
        // user has already moved past.
        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(first) => {
                let mut last_volume: Option<f32> = None;
                let mut last_mute: Option<bool> = None;
                let mut transport: Vec<Command> = Vec::new();
                let mut consider = |cmd: Command| match cmd {
                    Command::SetVolume(level) => last_volume = Some(level),
                    Command::SetMuted(muted) => last_mute = Some(muted),
                    other => transport.push(other),
                };
                consider(first);
                while let Ok(next) = rx.try_recv() {
                    consider(next);
                }

                // Newest write first, so a drag's final position lands before
                // anything else queued behind it.
                if let Some(level) = last_volume {
                    apply_volume(level, &snapshot, &revision);
                }
                if let Some(muted) = last_mute {
                    apply_mute(muted, &snapshot, &revision);
                }
                for cmd in transport {
                    handle_transport(cmd);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    unsafe { CoUninitialize() };
}

fn current_session() -> Option<windows::Media::Control::GlobalSystemMediaTransportControlsSession> {
    let manager = SessionManager::RequestAsync().ok()?.get().ok()?;
    manager.GetCurrentSession().ok()
}

/// Apply a volume write and settle the snapshot on what Windows actually
/// accepted — a requested level is not always the granted one.
fn apply_volume(level: f32, snapshot: &Arc<RwLock<NowPlaying>>, revision: &Arc<AtomicU64>) {
    volume::set_level(level);
    let confirmed = volume::read();
    {
        let mut current = snapshot.write();
        current.volume_pending = false;
        if let Some(state) = confirmed {
            current.volume = state;
        }
    }
    revision.fetch_add(1, Ordering::Release);
}

fn apply_mute(muted: bool, snapshot: &Arc<RwLock<NowPlaying>>, revision: &Arc<AtomicU64>) {
    volume::set_muted(muted);
    let confirmed = volume::read();
    {
        let mut current = snapshot.write();
        if let Some(state) = confirmed {
            current.volume = state;
        }
    }
    revision.fetch_add(1, Ordering::Release);
}

/// Play/pause/next/previous still go through the media session, unchanged.
fn handle_transport(cmd: Command) {
    let Some(session) = current_session() else {
        return;
    };
    // Each result is discarded individually: the three WinRT calls return
    // different `Result` shapes, and all of them are fire-and-forget — a
    // refused command is not something the notch can report or retry.
    match cmd {
        Command::PlayPause => {
            let _ = session.TryTogglePlayPauseAsync().and_then(|op| op.get());
        }
        Command::Next => {
            let _ = session.TrySkipNextAsync().and_then(|op| op.get());
        }
        Command::Previous => {
            let _ = session.TrySkipPreviousAsync().and_then(|op| op.get());
        }
        Command::SetVolume(_) | Command::SetMuted(_) => {}
    }
}

/// Publish `next` and bump the revision, but only if it differs from what is
/// already there. Most polls find the same track in the same state, and the
/// notch treats a bumped revision as a reason to repaint.
fn publish(snapshot: &Arc<RwLock<NowPlaying>>, revision: &Arc<AtomicU64>, next: NowPlaying) {
    {
        let current = snapshot.read();
        // `art` is compared through `art_generation`, which the poller bumps
        // whenever it reads new bytes — cheaper than comparing the bytes.
        if current.has_session == next.has_session
            && current.playing == next.playing
            && current.title == next.title
            && current.artist == next.artist
            && current.art_generation == next.art_generation
            && !volume_changed(&current, &next)
        {
            return;
        }
    }

    *snapshot.write() = next;
    revision.fetch_add(1, Ordering::Release);
}

/// Whether the volume half of the snapshot differs enough to be worth a repaint.
///
/// While the user is dragging, the freshly-read value is deliberately ignored:
/// the drag is the authority, and letting a 900 ms read win would snap the
/// handle back to where Windows last confirmed it. A device that appears or
/// disappears still counts, because the control has to show or hide itself.
fn volume_changed(current: &NowPlaying, next: &NowPlaying) -> bool {
    if current.has_volume != next.has_volume || current.volume.muted != next.volume.muted {
        return true;
    }
    // A queued write outranks a polled read: until the poller has applied it,
    // Windows is reporting the value the user is replacing, not a new one.
    if next.volume_pending {
        return false;
    }
    !next.volume_dragging && (current.volume.level - next.volume.level).abs() > f32::EPSILON
}

fn poll_once(
    snapshot: &Arc<RwLock<NowPlaying>>,
    revision: &Arc<AtomicU64>,
    art_generation: &mut u64,
    last_track_key: &mut Option<String>,
) {
    // Volume is independent of any media session, so it is read first and
    // carried through the no-session path. Reading it only after the session
    // check would wipe the control every time playback stopped, which has
    // nothing to do with the user's audio hardware.
    let volume = volume::read();
    let (volume, has_volume) = match volume {
        Some(v) => (v, true),
        None => (VolumeState::default(), false),
    };
    // Both flags are carried over from whatever the UI last set. A poll is a
    // read of the world, not a claim on what the user is in the middle of doing.
    let (dragging, pending) = {
        let current = snapshot.read();
        (current.volume_dragging, current.volume_pending)
    };

    let Some(session) = current_session() else {
        *last_track_key = None;
        publish(
            snapshot,
            revision,
            NowPlaying {
                volume,
                has_volume,
                volume_dragging: dragging,
                volume_pending: pending,
                ..NowPlaying::default()
            },
        );
        return;
    };

    let playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|s| s == PlaybackStatus::Playing)
        .unwrap_or(false);

    let props = session.TryGetMediaPropertiesAsync().and_then(|op| op.get());
    let (title, artist, thumb_ref) = match &props {
        Ok(p) => (
            p.Title().map(|h| h.to_string_lossy()).unwrap_or_default(),
            p.Artist().map(|h| h.to_string_lossy()).unwrap_or_default(),
            p.Thumbnail().ok(),
        ),
        Err(_) => (String::new(), String::new(), None),
    };

    // The thumbnail read is the expensive part of a poll; only redo it when
    // the track has actually changed.
    let key = format!("{title}\u{0}{artist}");
    let track_changed = Some(&key) != last_track_key.as_ref();
    *last_track_key = Some(key);

    let art = if track_changed {
        let bytes = thumb_ref.and_then(|r| read_thumbnail(&r));
        if bytes.is_some() {
            *art_generation += 1;
        }
        bytes.map(Arc::from)
    } else {
        snapshot.read().art.clone()
    };

    publish(
        snapshot,
        revision,
        NowPlaying {
            has_session: true,
            playing,
            title,
            artist,
            art,
            art_generation: *art_generation,
            volume,
            has_volume,
            volume_dragging: dragging,
            volume_pending: pending,
        },
    );
}

fn read_thumbnail(reference: &IRandomAccessStreamReference) -> Option<Vec<u8>> {
    let stream = reference.OpenReadAsync().ok()?.get().ok()?;
    let size = stream.Size().ok()?;
    if size == 0 || size > MAX_ART_BYTES {
        return None;
    }

    let input: IInputStream = stream.cast().ok()?;
    let reader = DataReader::CreateDataReader(&input).ok()?;
    reader.LoadAsync(size as u32).ok()?.get().ok()?;

    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf).ok()?;
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing_at(level: f32, dragging: bool, pending: bool) -> NowPlaying {
        NowPlaying {
            has_session: true,
            playing: true,
            title: "t".into(),
            artist: "a".into(),
            volume: VolumeState {
                level,
                muted: false,
            },
            has_volume: true,
            volume_dragging: dragging,
            volume_pending: pending,
            ..NowPlaying::default()
        }
    }

    /// The bug this exists for: a poll landing between "button released" and
    /// "queued write applied" used to read the pre-write level from Windows and
    /// drag the handle back to it, even though the sound had changed
    /// correctly. A pending write has to win over a polled read.
    #[test]
    fn a_pending_write_is_not_overwritten_by_a_stale_read() {
        // Handle is where the user put it; Windows has not caught up yet.
        let current = playing_at(0.3, false, true);
        // The poll still sees the old level, because the write is queued.
        let next = playing_at(1.0, false, true);
        assert!(!volume_changed(&current, &next));
    }

    /// Once the write lands, the poll is authoritative again — otherwise a
    /// rejected or clamped write would stick on screen forever.
    #[test]
    fn once_the_write_has_landed_a_poll_can_correct_the_handle() {
        let current = playing_at(0.3, false, false);
        let next = playing_at(0.5, false, false);
        assert!(volume_changed(&current, &next));
    }

    /// A drag in progress is the user's, not the poller's.
    #[test]
    fn an_in_progress_drag_ignores_a_poll_that_looks_like_a_change() {
        let current = playing_at(0.4, true, false);
        let next = playing_at(1.0, true, false);
        assert!(!volume_changed(&current, &next));
    }

    /// Losing the output device has to reach the screen even mid-gesture,
    /// otherwise the control sits there showing a device that is gone.
    #[test]
    fn losing_the_device_still_repaints_while_dragging() {
        let current = playing_at(0.4, true, true);
        let mut next = playing_at(0.4, true, true);
        next.has_volume = false;
        assert!(volume_changed(&current, &next));
    }

    /// Mute is a separate axis from level, and must repaint independently.
    #[test]
    fn a_mute_toggle_repaints_even_at_an_unchanged_level() {
        let current = playing_at(0.6, false, false);
        let mut next = playing_at(0.6, false, false);
        next.volume.muted = true;
        assert!(volume_changed(&current, &next));
    }

    /// An unchanged poll is not a change. This is what keeps the notch from
    /// repainting four times a second for no reason.
    #[test]
    fn an_unchanged_poll_is_not_a_change() {
        let current = playing_at(0.42, false, false);
        let next = playing_at(0.42, false, false);
        assert!(!volume_changed(&current, &next));
    }
}
