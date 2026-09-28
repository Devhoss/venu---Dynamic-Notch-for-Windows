//! System output volume, via Windows Core Audio.
//!
//! Deliberately free of threads, timers and caching: the media poller in
//! [`super::media`] calls [`read`] on its existing 900 ms cycle, and the UI
//! thread only ever sees the published snapshot. Every entry point re-resolves
//! the default endpoint from scratch.
//!
//! That is not fussiness. The default render endpoint changes identity when
//! headphones are plugged in, a Bluetooth device connects, or a display is
//! switched — a cached `IAudioEndpointVolume` would then point into a device
//! that no longer exists. Re-resolving costs two COM calls per poll, which is
//! cheaper than being wrong.
//!
//! Scope is the default **output** endpoint. `eRender` is requested
//! explicitly, so the microphone and any other input device are never touched.

use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};

/// What the painter needs to draw the control. `None` from [`read`] means there
/// is no usable output device, and the control should not be drawn at all.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VolumeState {
    /// Master output level, always within `0.0..=1.0`.
    pub level: f32,
    /// Mute is independent of level: a muted endpoint can still sit at 0.7, and
    /// showing that as 0% would misreport what the user will actually hear.
    pub muted: bool,
}

impl VolumeState {
    /// The level to display. A muted endpoint reads as silent whatever its
    /// scalar says, which is also what Windows' own flyout shows.
    pub fn effective_level(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.level
        }
    }

    /// Whether to *draw* the muted glyph.
    ///
    /// True when the endpoint is muted **or** the level has reached zero. This
    /// is a display decision only — it never sets the mute flag, for two
    /// reasons.
    ///
    /// The first is honesty: on a laptop's built-in speakers the volume curve
    /// is non-linear, so scalar 0 maps to a very quiet but non-silent floor. A
    /// speaker drawn at full weight next to a level of 0 would be claiming
    /// something untrue. Windows' own flyout draws the muted icon at zero for
    /// the same reason, and likewise leaves the flag alone.
    ///
    /// The second is that level and mute are independent states here: a muted
    /// endpoint can sit at any level, and auto-setting mute at zero would make
    /// the two collapse into one. Dragging back up would then have to decide
    /// whether to un-mute, either overriding a deliberate mute or leaving the
    /// slider reading 30% over a silent system — both worse than the gap this
    /// is closing. The mute button stays the only thing that changes mute.
    pub fn shows_muted(&self) -> bool {
        self.muted || self.level <= 0.0
    }
}

/// Constrain a level to the range Core Audio accepts. Guards the public entry
/// points so a bad value can never reach a `Set` call, and gives the UI a
/// single definition of "clamped" to share.
pub fn clamp(level: f32) -> f32 {
    if level.is_nan() {
        return 0.0;
    }
    level.clamp(0.0, 1.0)
}

/// Resolve the default render endpoint and activate its volume interface.
///
/// Both steps are ordinary fallible Windows calls: no output device yields
/// `E_NOTFOUND`, and stopped audio services yield something else. Every failure
/// collapses to `None` so the caller can hide the control rather than propagate
/// an error the user cannot act on.
fn endpoint_volume() -> Option<IAudioEndpointVolume> {
    // SAFETY: the media poller initializes COM on its own thread before it can
    // reach this, and every COM object produced here is dropped on that same
    // thread before returning.
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER).ok()?;
        // eRender, not eAll: this is the speakers/headphones, never the mic.
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .ok()?;
        Some(device.Activate(CLSCTX_INPROC_SERVER, None).ok()?)
    }
}

/// Current output volume and mute state, or `None` when there is no output
/// device to report on.
pub fn read() -> Option<VolumeState> {
    // SAFETY: see [`endpoint_volume`].
    unsafe {
        let volume = endpoint_volume()?;
        let level = volume.GetMasterVolumeLevelScalar().ok()?;
        let muted = volume.GetMute().ok()?.as_bool();
        Some(VolumeState {
            level: clamp(level),
            muted,
        })
    }
}

/// Set the master output level. Clamped by [`clamp`] before it reaches Windows.
pub fn set_level(level: f32) {
    // SAFETY: see [`endpoint_volume`]. The endpoint is resolved and released
    // here, so a device that vanished since the last poll is a plain `None`.
    unsafe {
        let Some(volume) = endpoint_volume() else {
            return;
        };
        let _ = volume.SetMasterVolumeLevelScalar(clamp(level), std::ptr::null());
    }
}

/// Set mute without disturbing the level, so unmuting restores what the user
/// had rather than dropping them back to a default.
pub fn set_muted(muted: bool) {
    // SAFETY: see [`endpoint_volume`].
    unsafe {
        let Some(volume) = endpoint_volume() else {
            return;
        };
        let _ = volume.SetMute(muted, std::ptr::null());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamping_keeps_values_inside_the_scalar_range() {
        assert_eq!(clamp(0.0), 0.0);
        assert_eq!(clamp(0.5), 0.5);
        assert_eq!(clamp(1.0), 1.0);
    }

    #[test]
    fn clamping_truncates_out_of_range_values() {
        assert_eq!(clamp(-0.5), 0.0);
        assert_eq!(clamp(-1.0), 0.0);
        assert_eq!(clamp(1.5), 1.0);
        assert_eq!(clamp(999.0), 1.0);
    }

    #[test]
    fn clamping_rejects_nan_rather_than_passing_it_through() {
        // A NaN reaching SetMasterVolumeLevelScalar would be undefined; this
        // clamp is the only thing standing between a bad slider value and it.
        assert_eq!(clamp(f32::NAN), 0.0);
    }

    #[test]
    fn mute_is_independent_of_the_stored_level() {
        // The case that matters: a muted endpoint whose scalar is well above
        // zero. It must not be reported as 0% volume, only as muted.
        let state = VolumeState {
            level: 0.7,
            muted: true,
        };
        assert_eq!(state.level, 0.7);
        assert!(state.muted);
        assert_eq!(state.effective_level(), 0.0);
    }

    #[test]
    fn an_unmuted_level_displays_as_itself() {
        let state = VolumeState {
            level: 0.42,
            muted: false,
        };
        assert_eq!(state.effective_level(), 0.42);
    }

    /// The reason this exists: a laptop speaker at scalar 0 is not actually
    /// silent, so the glyph must not claim it is audible.
    #[test]
    fn a_zero_level_shows_the_muted_glyph() {
        let state = VolumeState {
            level: 0.0,
            muted: false,
        };
        assert!(state.shows_muted());
    }

    /// The important counter-test: showing the muted glyph must not have set
    /// the mute flag. Level and mute stay independent states, or dragging back
    /// up would either override a deliberate mute or leave the slider reading
    /// a level the system is not playing at.
    #[test]
    fn a_zero_level_does_not_become_mute() {
        let state = VolumeState {
            level: 0.0,
            muted: false,
        };
        assert!(state.shows_muted());
        assert!(!state.muted, "the mute flag must stay false at zero level");
    }

    /// Any audible level draws the live glyph, including the smallest step the
    /// endpoint will actually accept.
    #[test]
    fn an_audible_level_shows_the_live_glyph() {
        for level in [0.01_f32, 0.2, 0.5, 1.0] {
            let state = VolumeState {
                level,
                muted: false,
            };
            assert!(!state.shows_muted(), "level {level} should read as live");
        }
    }

    /// Muting at an audible level shows the glyph and empties the track, and
    /// unmuting puts the level back.
    #[test]
    fn mute_shows_the_glyph_at_any_level_and_unmute_restores_it() {
        let muted = VolumeState {
            level: 0.6,
            muted: true,
        };
        assert!(muted.shows_muted());
        assert_eq!(muted.effective_level(), 0.0);

        let unmuted = VolumeState {
            level: 0.6,
            muted: false,
        };
        assert!(!unmuted.shows_muted());
        assert_eq!(unmuted.effective_level(), 0.6);
    }
}
