//! Media backends. A backend connects to a room and reports what happens in it as
//! [`RoomEvent`]s. The UI never talks to the network directly.
//!
//! Only [`SimulatedRoom`] exists for now. A LiveKit backend will implement the same
//! trait (see `docs/PLAN.md`).

use std::time::Duration;

use crate::call::{Participant, ParticipantId, RoomEvent};

pub trait RoomBackend {
    /// Advances the backend by `dt` and returns the events since the last call.
    fn poll(&mut self, dt: Duration) -> Vec<RoomEvent>;
    fn set_mic(&mut self, on: bool);
    fn set_camera(&mut self, on: bool);
    fn leave(&mut self);
}

/// A fake room with a few guests who join, talk in turns, and mute themselves.
/// It lets us build and demo the UI without a media server.
pub struct SimulatedRoom {
    rng: Lcg,
    clock: Duration,
    pending_guests: Vec<(Duration, Participant)>,
    guests: Vec<ParticipantId>,
    speaker: Option<ParticipantId>,
    next_turn: Duration,
    local_mic_on: bool,
    left: bool,
}

const GUESTS: &[(&str, f32)] = &[
    ("Ada Lovelace", 0.02),
    ("Grace Hopper", 0.33),
    ("Alan Turing", 0.80),
];

impl SimulatedRoom {
    pub fn new(seed: u64) -> Self {
        let pending_guests = GUESTS
            .iter()
            .enumerate()
            .map(|(i, (name, hue))| {
                let id = ParticipantId(i as u64 + 1);
                let mut p = Participant::new(id, *name, *hue);
                // One guest joins with the camera off, to show the initials bubble.
                p.camera_on = i != 1;
                (Duration::from_millis(400 + 900 * i as u64), p)
            })
            .rev()
            .collect();
        Self {
            rng: Lcg(seed | 1),
            clock: Duration::ZERO,
            pending_guests,
            guests: Vec::new(),
            speaker: None,
            next_turn: Duration::from_secs(2),
            local_mic_on: true,
            left: false,
        }
    }
}

impl RoomBackend for SimulatedRoom {
    fn poll(&mut self, dt: Duration) -> Vec<RoomEvent> {
        if self.left {
            return Vec::new();
        }
        self.clock += dt;
        let mut events = Vec::new();

        while let Some((at, _)) = self.pending_guests.last() {
            if *at > self.clock {
                break;
            }
            let (_, guest) = self.pending_guests.pop().unwrap();
            self.guests.push(guest.id);
            events.push(RoomEvent::ParticipantJoined(guest));
        }

        if self.clock >= self.next_turn && !self.guests.is_empty() {
            // Pick the next speaker. Index `guests.len()` means "you", or silence if muted.
            let pick = self.rng.below(self.guests.len() as u64 + 1) as usize;
            self.speaker = match self.guests.get(pick) {
                Some(id) => Some(*id),
                None if self.local_mic_on => Some(ParticipantId(0)),
                None => None,
            };
            self.next_turn = self.clock + Duration::from_millis(1500 + self.rng.below(2500));

            // Now and then a guest mutes or unmutes.
            if self.rng.below(5) == 0 {
                let id = self.guests[self.rng.below(self.guests.len() as u64) as usize];
                if Some(id) != self.speaker {
                    events.push(RoomEvent::MicChanged(id, self.rng.below(2) == 0));
                }
            }

            // Less often, a guest turns the camera on or off.
            if self.rng.below(9) == 0 {
                let id = self.guests[self.rng.below(self.guests.len() as u64) as usize];
                events.push(RoomEvent::CameraChanged(id, self.rng.below(2) == 0));
            }
        }

        let t = self.clock.as_secs_f32();
        let ids = std::iter::once(ParticipantId(0)).chain(self.guests.iter().copied());
        for id in ids.collect::<Vec<_>>() {
            let level = if Some(id) == self.speaker {
                // Speech-like envelope: syllables on top of a slower phrase rhythm.
                let syllables = (t * 9.0 + id.0 as f32).sin().abs();
                let phrase = 0.6 + 0.4 * (t * 1.7 + id.0 as f32 * 2.0).sin();
                0.2 + 0.6 * syllables * phrase
            } else {
                0.02 * self.rng.unit()
            };
            events.push(RoomEvent::AudioLevel(id, level));
        }
        events
    }

    fn set_mic(&mut self, on: bool) {
        self.local_mic_on = on;
        if !on && self.speaker == Some(ParticipantId(0)) {
            self.speaker = None;
        }
    }

    fn set_camera(&mut self, _on: bool) {}

    fn leave(&mut self) {
        self.left = true;
    }
}

/// Small deterministic random number generator, so the demo needs no extra crates.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }

    fn unit(&mut self) -> f32 {
        (self.next() % 10_000) as f32 / 10_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call::CallState;

    #[test]
    fn guests_join_and_someone_speaks() {
        let mut room = SimulatedRoom::new(7);
        let mut call = CallState::new("demo", "Me");
        for _ in 0..200 {
            for event in room.poll(Duration::from_millis(50)) {
                call.apply(event);
            }
        }
        assert_eq!(call.participants.len(), 1 + GUESTS.len());
        let mut heard_speaker = false;
        for _ in 0..200 {
            for event in room.poll(Duration::from_millis(50)) {
                call.apply(event);
            }
            heard_speaker |= call.active_speaker().is_some();
        }
        assert!(heard_speaker);
    }

    #[test]
    fn leaving_stops_events() {
        let mut room = SimulatedRoom::new(1);
        room.leave();
        assert!(room.poll(Duration::from_secs(10)).is_empty());
    }
}
