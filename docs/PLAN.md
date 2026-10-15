# Circles: project plan

## Goal

Make Circles, a desktop video call app that works like Around:

- Small round video bubbles that float over other apps.
- Compact controls that do not take space from your work.
- Fast to join: one click from a link.

## Technology

| Part | Choice | Why |
| --- | --- | --- |
| Desktop UI | GPUI 0.2 (Rust) | GPU-rendered, native windows, low memory. Zed uses it for its own calls. |
| Media server | LiveKit (Cloud or self-hosted) | Open source SFU. Has a Rust SDK. Zed uses it too. |
| Media client | `livekit` Rust crate (libwebrtc) | Same language as the UI. No web view. |
| Camera capture | `nokhwa` crate, or platform APIs | The LiveKit Rust SDK does not capture devices. |
| Audio in and out | `cpal` crate into LiveKit `NativeAudioSource` | Cross-platform. |
| Token service | Small Rust service (`axum` + `livekit-api`) | Gives each client a signed room token. |

## Architecture

```
 ┌──────────── desktop app ─────────────┐
 │  ui/ (GPUI views)                    │
 │    lobby ── call_window ── bubble    │
 │           │ RoomEvent / commands     │
 │  call.rs (state, no GPUI)            │
 │           │                          │
 │  media.rs  RoomBackend trait         │
 │    ├─ SimulatedRoom  (now)           │
 │    └─ LiveKitRoom    (phase 2)       │
 └───────────┼──────────────────────────┘
             │ WebRTC
       LiveKit server ◄── token service
```

The UI only reads `CallState` and sends commands to a `RoomBackend`. The
backend sends `RoomEvent`s back. This keeps media code out of the views, and
lets us test the UI with the simulated room.

## Phases

### Phase 1: UI scaffold (done)

- Start window and floating call window.
- Bubbles, speaking ring, badges, control bar, key bindings.
- Simulated room with fake guests.
- Unit tests for call state and the simulated room.

### Phase 2: Real calls with LiveKit

1. Add the token service (`server/`). It signs a token for a room and a name.
2. Add `LiveKitRoom: RoomBackend`. Connect, then map LiveKit room events to `RoomEvent`.
3. Publish the microphone (`cpal` capture into `NativeAudioSource`) and play remote audio.
4. Publish the camera (`nokhwa` capture into `NativeVideoSource`).
5. Show remote video. Convert each I420 frame to BGRA, make a GPUI `RenderImage`,
   and draw it with `img()` clipped to a circle. On macOS, use GPUI `surface()`
   with the `CVPixelBuffer` for less copying.
6. Use LiveKit audio levels for the speaking ring.
7. Room links: `circles://join/<room>` and a web page that opens the app.

### Phase 3: Features from Around

- Screen share in its own window, with a picker for screen or window.
- Auto-framing: crop the camera to the face so it fits the circle.
- Background blur and noise suppression.
- Emoji reactions that float over the bubbles.
- Text chat and a shared notes panel.
- Hover to show controls, so the bar hides when you do not need it.

### Phase 4: Ship

- Settings: devices, display name, start at login.
- Sign in and saved rooms.
- Packages: macOS `.app` (signed and notarized), Windows installer, Linux AppImage or Flatpak.
- Auto-update.

## Risks

- GPUI is young and its API changes. We pin the version and update on purpose.
- GPUI has no API to move a window from code. We may need a small patch upstream
  to keep the window anchored to the screen corner when it grows.
- libwebrtc makes the build large and slow. Cache it in CI.
- On Linux, a transparent window needs a compositor.
