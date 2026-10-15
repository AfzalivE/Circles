//! Colors and shared styles. Circles uses a dark, glassy look on top of whatever
//! is behind the floating window.

use gpui::{BoxShadow, Hsla, hsla, point, px};

pub fn surface() -> Hsla {
    hsla(240. / 360., 0.12, 0.10, 0.92)
}

pub fn surface_hover() -> Hsla {
    hsla(240. / 360., 0.10, 0.22, 1.0)
}

pub fn text() -> Hsla {
    hsla(0., 0., 0.97, 1.0)
}

pub fn text_muted() -> Hsla {
    hsla(240. / 360., 0.06, 0.68, 1.0)
}

pub fn speaking() -> Hsla {
    hsla(146. / 360., 0.72, 0.52, 1.0)
}

pub fn danger() -> Hsla {
    hsla(356. / 360., 0.78, 0.56, 1.0)
}

pub fn accent() -> Hsla {
    hsla(250. / 360., 0.86, 0.66, 1.0)
}

pub fn soft_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hsla(0., 0., 0., 0.35),
        offset: point(px(0.), px(4.)),
        blur_radius: px(14.),
        spread_radius: px(0.),
    }]
}

/// Glow around the bubble of whoever is speaking. Stronger when louder.
pub fn speaking_glow(level: f32) -> Vec<BoxShadow> {
    let mut glow = speaking();
    glow.a = 0.35 + 0.45 * level.clamp(0.0, 1.0);
    vec![BoxShadow {
        color: glow,
        offset: point(px(0.), px(0.)),
        blur_radius: px(10. + 14. * level),
        spread_radius: px(1. + 3. * level),
    }]
}
