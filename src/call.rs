//! Call state: participants, local controls, and the floating window layout.
//!
//! This module has no GPUI dependency so that the call logic stays easy to test.

use std::time::Duration;

/// Audio level above which a participant counts as speaking.
pub const SPEAKING_THRESHOLD: f32 = 0.15;

/// Space between bubbles, and between bubbles and the window edge.
pub const BUBBLE_GAP: f32 = 12.0;

/// Height of the control bar under (or beside) the bubbles.
pub const CONTROLS_HEIGHT: f32 = 44.0;

/// Width of the control bar.
pub const CONTROLS_WIDTH: f32 = 292.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParticipantId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct Participant {
    pub id: ParticipantId,
    pub name: String,
    /// Hue in `0.0..1.0`, used for the bubble color when the camera is off.
    pub hue: f32,
    pub mic_on: bool,
    pub camera_on: bool,
    pub screen_sharing: bool,
    /// Smoothed audio level in `0.0..=1.0`.
    pub audio_level: f32,
    pub is_local: bool,
}

impl Participant {
    pub fn new(id: ParticipantId, name: impl Into<String>, hue: f32) -> Self {
        Self {
            id,
            name: name.into(),
            hue,
            mic_on: true,
            camera_on: true,
            screen_sharing: false,
            audio_level: 0.0,
            is_local: false,
        }
    }

    /// Up to two uppercase initials, for example "Ada Lovelace" -> "AL".
    pub fn initials(&self) -> String {
        self.name
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .flat_map(char::to_uppercase)
            .collect()
    }

    pub fn is_speaking(&self) -> bool {
        self.mic_on && self.audio_level >= SPEAKING_THRESHOLD
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BubbleSize {
    Small,
    Medium,
    Large,
}

impl BubbleSize {
    pub fn diameter(self) -> f32 {
        match self {
            BubbleSize::Small => 72.0,
            BubbleSize::Medium => 104.0,
            BubbleSize::Large => 148.0,
        }
    }

    pub fn next(self) -> Self {
        match self {
            BubbleSize::Small => BubbleSize::Medium,
            BubbleSize::Medium => BubbleSize::Large,
            BubbleSize::Large => BubbleSize::Small,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Bubbles side by side, controls under them.
    Row,
    /// Bubbles stacked, controls under them. Good for the side of the screen.
    Column,
}

impl Layout {
    pub fn toggled(self) -> Self {
        match self {
            Layout::Row => Layout::Column,
            Layout::Column => Layout::Row,
        }
    }
}

/// Events that a media backend sends to the call.
#[derive(Clone, Debug, PartialEq)]
pub enum RoomEvent {
    ParticipantJoined(Participant),
    // Not sent by the simulated room yet; the LiveKit backend will send it.
    #[allow(dead_code)]
    ParticipantLeft(ParticipantId),
    AudioLevel(ParticipantId, f32),
    MicChanged(ParticipantId, bool),
    CameraChanged(ParticipantId, bool),
}

#[derive(Clone, Debug)]
pub struct CallState {
    pub room: String,
    pub participants: Vec<Participant>,
    pub layout: Layout,
    pub bubble_size: BubbleSize,
    pub elapsed: Duration,
    local_id: ParticipantId,
}

impl CallState {
    pub fn new(room: impl Into<String>, local_name: impl Into<String>) -> Self {
        let local_id = ParticipantId(0);
        let mut local = Participant::new(local_id, local_name, 0.58);
        local.is_local = true;
        Self {
            room: room.into(),
            participants: vec![local],
            layout: Layout::Row,
            bubble_size: BubbleSize::Medium,
            elapsed: Duration::ZERO,
            local_id,
        }
    }

    pub fn local(&self) -> &Participant {
        self.participant(self.local_id)
            .expect("local participant is always present")
    }

    pub fn participant(&self, id: ParticipantId) -> Option<&Participant> {
        self.participants.iter().find(|p| p.id == id)
    }

    fn participant_mut(&mut self, id: ParticipantId) -> Option<&mut Participant> {
        self.participants.iter_mut().find(|p| p.id == id)
    }

    /// Toggles the local microphone and returns the new state.
    pub fn toggle_mic(&mut self) -> bool {
        let id = self.local_id;
        let p = self.participant_mut(id).unwrap();
        p.mic_on = !p.mic_on;
        if !p.mic_on {
            p.audio_level = 0.0;
        }
        p.mic_on
    }

    /// Toggles the local camera and returns the new state.
    pub fn toggle_camera(&mut self) -> bool {
        let id = self.local_id;
        let p = self.participant_mut(id).unwrap();
        p.camera_on = !p.camera_on;
        p.camera_on
    }

    /// Toggles local screen sharing and returns the new state.
    pub fn toggle_screen_share(&mut self) -> bool {
        let id = self.local_id;
        let p = self.participant_mut(id).unwrap();
        p.screen_sharing = !p.screen_sharing;
        p.screen_sharing
    }

    pub fn cycle_bubble_size(&mut self) {
        self.bubble_size = self.bubble_size.next();
    }

    pub fn toggle_layout(&mut self) {
        self.layout = self.layout.toggled();
    }

    /// The loudest participant who is speaking, if any.
    pub fn active_speaker(&self) -> Option<ParticipantId> {
        self.participants
            .iter()
            .filter(|p| p.is_speaking())
            .max_by(|a, b| a.audio_level.total_cmp(&b.audio_level))
            .map(|p| p.id)
    }

    pub fn apply(&mut self, event: RoomEvent) {
        match event {
            RoomEvent::ParticipantJoined(p) => {
                if self.participant(p.id).is_none() {
                    self.participants.push(p);
                }
            }
            RoomEvent::ParticipantLeft(id) => {
                if id != self.local_id {
                    self.participants.retain(|p| p.id != id);
                }
            }
            RoomEvent::AudioLevel(id, level) => {
                if let Some(p) = self.participant_mut(id) {
                    p.audio_level = if p.mic_on { level.clamp(0.0, 1.0) } else { 0.0 };
                }
            }
            RoomEvent::MicChanged(id, on) => {
                if let Some(p) = self.participant_mut(id) {
                    p.mic_on = on;
                    if !on {
                        p.audio_level = 0.0;
                    }
                }
            }
            RoomEvent::CameraChanged(id, on) => {
                if let Some(p) = self.participant_mut(id) {
                    p.camera_on = on;
                }
            }
        }
    }

    /// The window size, in logical pixels, that fits all bubbles and the control bar.
    pub fn window_size(&self) -> (f32, f32) {
        self.window_size_for(self.participants.len())
    }

    /// The window size for `count` participants with the current layout and bubble size.
    pub fn window_size_for(&self, count: usize) -> (f32, f32) {
        let n = count.max(1) as f32;
        let d = self.bubble_size.diameter();
        let (bubbles_w, bubbles_h) = match self.layout {
            Layout::Row => (n * d + (n - 1.0) * BUBBLE_GAP, d),
            Layout::Column => (d, n * d + (n - 1.0) * BUBBLE_GAP),
        };
        let width = bubbles_w.max(CONTROLS_WIDTH) + 2.0 * BUBBLE_GAP;
        let height = bubbles_h + CONTROLS_HEIGHT + 3.0 * BUBBLE_GAP;
        (width, height)
    }

    /// Elapsed call time as `m:ss`, or `h:mm:ss` after one hour.
    pub fn elapsed_label(&self) -> String {
        let secs = self.elapsed.as_secs();
        let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
        if h > 0 {
            format!("{h}:{m:02}:{s:02}")
        } else {
            format!("{m}:{s:02}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guest(id: u64, name: &str) -> Participant {
        Participant::new(ParticipantId(id), name, 0.1)
    }

    #[test]
    fn initials_use_first_two_words() {
        assert_eq!(guest(1, "Ada Lovelace").initials(), "AL");
        assert_eq!(guest(1, "grace").initials(), "G");
        assert_eq!(guest(1, "Alan Mathison Turing").initials(), "AM");
        assert_eq!(guest(1, "").initials(), "");
    }

    #[test]
    fn muting_clears_audio_level() {
        let mut call = CallState::new("room", "Me");
        call.apply(RoomEvent::AudioLevel(ParticipantId(0), 0.8));
        assert!(call.local().is_speaking());
        assert!(!call.toggle_mic());
        assert_eq!(call.local().audio_level, 0.0);
        call.apply(RoomEvent::AudioLevel(ParticipantId(0), 0.8));
        assert!(!call.local().is_speaking());
    }

    #[test]
    fn active_speaker_is_loudest_unmuted_participant() {
        let mut call = CallState::new("room", "Me");
        call.apply(RoomEvent::ParticipantJoined(guest(1, "Ada")));
        call.apply(RoomEvent::ParticipantJoined(guest(2, "Grace")));
        assert_eq!(call.active_speaker(), None);

        call.apply(RoomEvent::AudioLevel(ParticipantId(1), 0.4));
        call.apply(RoomEvent::AudioLevel(ParticipantId(2), 0.7));
        assert_eq!(call.active_speaker(), Some(ParticipantId(2)));

        call.apply(RoomEvent::MicChanged(ParticipantId(2), false));
        assert_eq!(call.active_speaker(), Some(ParticipantId(1)));

        call.apply(RoomEvent::AudioLevel(ParticipantId(1), 0.05));
        assert_eq!(call.active_speaker(), None);
    }

    #[test]
    fn join_and_leave_keep_local_participant() {
        let mut call = CallState::new("room", "Me");
        call.apply(RoomEvent::ParticipantJoined(guest(1, "Ada")));
        call.apply(RoomEvent::ParticipantJoined(guest(1, "Ada again")));
        assert_eq!(call.participants.len(), 2);

        call.apply(RoomEvent::ParticipantLeft(ParticipantId(0)));
        call.apply(RoomEvent::ParticipantLeft(ParticipantId(1)));
        assert_eq!(call.participants.len(), 1);
        assert!(call.local().is_local);
    }

    #[test]
    fn window_size_follows_layout_and_bubble_size() {
        let mut call = CallState::new("room", "Me");
        for i in 1..=3 {
            call.apply(RoomEvent::ParticipantJoined(guest(i, "Guest")));
        }
        let d = BubbleSize::Medium.diameter();
        let row = call.window_size();
        assert_eq!(row.0, 4.0 * d + 3.0 * BUBBLE_GAP + 2.0 * BUBBLE_GAP);
        assert_eq!(row.1, d + CONTROLS_HEIGHT + 3.0 * BUBBLE_GAP);

        call.toggle_layout();
        let column = call.window_size();
        assert!(column.1 > row.1);
        assert_eq!(column.0, CONTROLS_WIDTH + 2.0 * BUBBLE_GAP);

        call.cycle_bubble_size();
        assert_eq!(call.bubble_size, BubbleSize::Large);
        assert!(call.window_size().1 > column.1);
    }

    #[test]
    fn elapsed_label_formats() {
        let mut call = CallState::new("room", "Me");
        call.elapsed = Duration::from_secs(65);
        assert_eq!(call.elapsed_label(), "1:05");
        call.elapsed = Duration::from_secs(3 * 3600 + 7 * 60 + 9);
        assert_eq!(call.elapsed_label(), "3:07:09");
    }
}
