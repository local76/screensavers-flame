//! Auxiliary types and structs for the flame screensaver.

/// A rising ember/spark particle.
pub struct Spark {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
}

/// A cell representing a character in the overlay logo.
pub struct LogoCell {
    pub x: usize,
    pub y: usize,
    pub ch: char,
    pub temp: f32,
}

/// A background star particle.
pub struct Star {
    pub x: f32,
    pub y: f32,
    pub phase: f32,
    pub ch: char,
    pub excitation: f32,
    pub excited_color: (u8, u8, u8),
}

/// A volcanic glob projectile.
pub struct VolcanicGlob {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
}
