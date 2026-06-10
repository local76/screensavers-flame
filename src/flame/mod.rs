//! Consolidated flame screensaver effect module.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).

use std::time::Duration;
use library::core::{LcgRng, TerminalCell};
use library::core::screensaver::Screensaver;
use library::core::logo_block::render_logo_block;
use library::toolkit::sys_info::get_system_info;
use library::toolkit::sys_info::query_current_palette;

mod types;
mod physics;

// Re-export or use internal types
pub use types::{Spark, LogoCell, Star, VolcanicGlob};

pub struct Flame {
    pub(crate) rng: LcgRng,
    pub(crate) fire_grid: Vec<u8>,
    pub(crate) sparks: Vec<Spark>,
    pub(crate) logo_cells: Vec<LogoCell>,
    pub(crate) stars: Vec<Star>,
    pub(crate) volcanic_globs: Vec<VolcanicGlob>,
    pub(crate) time_elapsed: f32,
    pub(crate) physics_accumulator: f32,
    pub(crate) last_cols: usize,
    pub(crate) last_rows: usize,
    pub(crate) palette: [(u8, u8, u8); 36],
    pub(crate) flame_height_opt: u32,
    pub(crate) spark_count_opt: u32,

    // Live system dynamics
    pub(crate) sys_refresh_timer: f32,
    pub(crate) mem_pressure: f32,
    pub(crate) cpu_load: f32,
    pub(crate) host_bias: f32,
}

impl Default for Flame {
    fn default() -> Self {
        Self::new()
    }
}

impl Flame {
    pub fn new() -> Self {
        // Pre-4.1 HKEY_CURRENT_USER registry reads (FlameHeight, SparkCount)
        // collapsed to defaults for the inline migration. Re-added in 4.2.
        let flame_height_opt: u32 = 1;
        let spark_count_opt: u32 = 1;

        // library 4.0: pull the accent + the fire heat ramp from the canonical
        // ScreenPalette. The local `get_palette(accent)` helper is a
        // fire-specific heat ramp (not accent-derived) so we still call
        // it directly, but we pass the library-routed accent through so
        // a future palette change propagates.
        let accent = query_current_palette().accent;
        let palette = physics::get_palette(accent);

        let sys = get_system_info();
        let host_bias = sys.hostname.chars().map(|c| c as u32).sum::<u32>() as f32 / 1000.0 % 1.0;
        let mem_pressure = sys.mem_used_pct / 100.0;
        let cpu_load = 0.4;

        Self {
            rng: LcgRng::new(9999),
            fire_grid: Vec::new(),
            sparks: Vec::new(),
            logo_cells: Vec::new(),
            stars: Vec::new(),
            volcanic_globs: Vec::new(),
            time_elapsed: 0.0,
            physics_accumulator: 0.0,
            last_cols: 0,
            last_rows: 0,
            palette,
            flame_height_opt,
            spark_count_opt,
            sys_refresh_timer: 0.0,
            mem_pressure,
            cpu_load,
            host_bias,
        }
    }
}

impl Screensaver for Flame {
    fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        let delta = dt.as_secs_f32();
        self.time_elapsed += delta;
        self.physics_accumulator += delta;

        // Live system refresh ~every sec
        self.sys_refresh_timer += delta;
        if self.sys_refresh_timer >= 1.0 {
            let sys = get_system_info();
            self.mem_pressure = sys.mem_used_pct / 100.0;
            self.cpu_load = (self.mem_pressure * 0.7 + 0.25).min(0.95);
            if self.host_bias > 0.7 { self.cpu_load = (self.cpu_load + 0.1).min(0.98); }
            self.sys_refresh_timer = 0.0;
        }

        // 1. Check for resize and initialize logo
        if cols != self.last_cols || rows != self.last_rows {
            self.fire_grid = vec![0; cols * rows];
            self.sparks.clear();
            self.logo_cells.clear();
            self.volcanic_globs.clear();
            // library 4.1: render the centered system-logo overlay from
            // the live system info (replaces the pre-4.1
            // `trance_core::logo_lines()` + `logo_dimensions()` Windows-only
            // file read).
            let logo_text = get_system_info().logo_text;
            let lines = render_logo_block(&logo_text, None);
            let logo_h = lines.len();
            let logo_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let logo_x = cols.saturating_sub(logo_w) / 2;
            let logo_y = rows.saturating_sub(logo_h) / 2;

            for (r_offset, line) in lines.iter().enumerate().take(logo_h) {
                for (c_offset, ch) in line.chars().enumerate() {
                    if ch != ' ' {
                        self.logo_cells.push(LogoCell {
                            x: logo_x + c_offset,
                            y: logo_y + r_offset,
                            ch,
                            temp: 0.0,
                        });
                    }
                }
            }

            self.last_cols = cols;
            self.last_rows = rows;

            // Create background stars
            let target_stars = (cols * rows / 20).clamp(10, 80);
            let mut stars = Vec::new();
            for i in 0..target_stars {
                stars.push(Star {
                    x: self.rng.next_f32(),
                    y: self.rng.next_f32(),
                    phase: self.rng.next_f32() * std::f32::consts::TAU,
                    ch: if i % 7 == 0 { '✦' } else if i % 3 == 0 { '•' } else { '.' },
                    excitation: 0.0,
                    excited_color: (255, 255, 255),
                });
            }
            self.stars = stars;

            // library 4.0: refresh the fire heat ramp from the canonical
            // ScreenPalette. The pre-4.0 cached `self.theme_accent` field
            // is gone; the per-frame accent is now pulled from
            // `query_current_palette()` in `drawing.rs` directly.
            let accent = query_current_palette().accent;
            self.palette = physics::get_palette(accent);
        }

        // 2. Fixed timestep step for fire cellular automata at 32 Hz
        let physics_step = 0.031;
        while self.physics_accumulator >= physics_step {
            self.physics_accumulator -= physics_step;
            physics::step_fire(self, cols, rows);
        }

        // 3. Update logo temperature
        for cell in &mut self.logo_cells {
            if cell.x >= cols || cell.y >= rows {
                continue;
            }
            let mut column_heat = 0.0;
            let check_depth = 12;
            for dy in 1..=check_depth {
                let check_y = cell.y + dy;
                if check_y < rows {
                    column_heat += self.fire_grid[check_y * cols + cell.x] as f32;
                }
            }
            let average_heat = column_heat / (check_depth as f32 * 35.0);
            cell.temp = cell.temp * 0.86 + average_heat * 0.14;

            let spark_logo_prob = match self.spark_count_opt {
                0 => 0.0135,
                2 => 0.1125,
                _ => 0.045,
            };
            if cell.temp > 0.15 && self.rng.next_bool(spark_logo_prob) {
                self.sparks.push(Spark {
                    x: cell.x as f32,
                    y: cell.y as f32,
                    vx: self.rng.next_range(-1.8, 1.8),
                    vy: -self.rng.next_range(4.5, 9.5),
                    life: self.rng.next_range(0.8, 2.0),
                    max_life: 2.0,
                });
            }
        }

        // Spawn sparks from top of the fire grid columns
        let spark_top_prob = match self.spark_count_opt {
            0 => 0.072,
            2 => 0.60,
            _ => 0.24,
        };
        if self.rng.next_bool(spark_top_prob) {
            let x = self.rng.next_usize(cols);
            for y in (rows / 2..rows - 2).rev() {
                let val = self.fire_grid[y * cols + x];
                if (6..=24).contains(&val) {
                    self.sparks.push(Spark {
                        x: x as f32,
                        y: y as f32,
                        vx: self.rng.next_range(-2.0, 2.0),
                        vy: -self.rng.next_range(5.5, 12.0),
                        life: self.rng.next_range(0.9, 2.3),
                        max_life: 2.3,
                    });
                    break;
                }
            }
        }

        // 4. Update spark velocities
        for spark in &mut self.sparks {
            let wind = (self.time_elapsed * 2.3 + spark.y * 0.08).sin() * 4.5;
            spark.vx += wind * delta;
            spark.vx = spark.vx.clamp(-8.0, 8.0);

            spark.x += spark.vx * delta;
            spark.y += spark.vy * delta;
            spark.life -= delta;
        }

        self.sparks.retain(|s| s.life > 0.0 && s.x >= 0.0 && s.x < cols as f32 && s.y >= 0.0 && s.y < rows as f32);

        // Decay star excitations
        for star in &mut self.stars {
            if star.excitation > 0.0 {
                star.excitation -= delta * 1.8;
                if star.excitation < 0.0 { star.excitation = 0.0; }
            }
        }

        // Excite background stars near sparks
        for spark in &self.sparks {
            for star in &mut self.stars {
                let sx = star.x * cols as f32;
                let sy = star.y * rows as f32;
                let dx = spark.x - sx;
                let dy = (spark.y - sy) * 2.0;
                let dist_sq = dx*dx + dy*dy;
                if dist_sq < 9.0 {
                    let dist = dist_sq.sqrt();
                    let force = (1.0 - dist / 3.0) * 1.5;
                    if force > star.excitation {
                        star.excitation = force;
                        let t = self.rng.next_f32();
                        star.excited_color = (255, (160.0 + t * 90.0) as u8, 0);
                    }
                }
            }
        }

        // Launch a new volcanic glob randomly
        let launch_chance = 0.015 * (1.0 + self.cpu_load);
        if self.volcanic_globs.len() < 3 && self.rng.next_bool(launch_chance) {
            let launch_left = self.rng.next_bool(0.5);
            let start_x = if launch_left {
                self.rng.next_range(2.0, (cols as f32 * 0.25).max(4.0))
            } else {
                self.rng.next_range((cols as f32 * 0.75).min(cols as f32 - 4.0), cols as f32 - 2.0)
            };
            let start_y = rows as f32 - 1.0;

            let speed_scale = (cols as f32 / 80.0).clamp(0.5, 2.5);
            let vx = if launch_left {
                self.rng.next_range(14.0, 26.0) * speed_scale
            } else {
                -self.rng.next_range(14.0, 26.0) * speed_scale
            };

            let gravity = 12.0f32;
            let target_height = rows as f32 * self.rng.next_range(0.5, 0.75);
            let vy = -(2.0 * gravity * target_height).sqrt();

            self.volcanic_globs.push(VolcanicGlob {
                x: start_x,
                y: start_y,
                vx,
                vy,
                life: 4.5,
            });
        }

        // Update volcanic globs
        let mut exploded_globs = Vec::new();
        let gravity = 12.0f32;

        for (idx, glob) in self.volcanic_globs.iter_mut().enumerate() {
            glob.vy += gravity * delta;
            glob.x += glob.vx * delta;
            glob.y += glob.vy * delta;
            glob.life -= delta;

            if self.rng.next_bool(0.35) {
                self.sparks.push(Spark {
                    x: glob.x,
                    y: glob.y,
                    vx: -glob.vx * 0.15 + self.rng.next_range(-0.5, 0.5),
                    vy: -glob.vy * 0.15 + self.rng.next_range(-0.5, 0.5),
                    life: 0.8,
                    max_life: 0.8,
                });
            }

            let mut hit = false;
            for cell in &mut self.logo_cells {
                let dx = glob.x - cell.x as f32;
                let dy = (glob.y - cell.y as f32) * 2.0;
                let dist = (dx*dx + dy*dy).sqrt();
                if dist < 1.6 {
                    hit = true;
                    cell.temp = 3.0; 
                }
            }

            if hit {
                exploded_globs.push((idx, glob.x, glob.y));
            }
        }

        // Handle glob explosions
        for (idx, x, y) in exploded_globs.into_iter().rev() {
            self.volcanic_globs.remove(idx);

            for _ in 0..25 {
                let angle = self.rng.next_range(0.0, std::f32::consts::TAU);
                let speed = self.rng.next_range(7.0, 16.0);
                self.sparks.push(Spark {
                    x,
                    y,
                    vx: angle.cos() * speed,
                    vy: angle.sin() * speed * 0.5 - 2.0,
                    life: self.rng.next_range(0.6, 1.6),
                    max_life: 1.6,
                });
            }

            let ex = x.round() as i32;
            let ey = y.round() as i32;
            let r_int = 4;
            for dy in -r_int..=r_int {
                for dx in -r_int..=r_int {
                    let px = ex + dx;
                    let py = ey + dy;
                    if px >= 0 && px < cols as i32 && py >= 0 && py < rows as i32
                        && (dx*dx + dy*dy) as f32 <= 16.0 {
                        let grid_idx = py as usize * cols + px as usize;
                        self.fire_grid[grid_idx] = 35;
                    }
                }
            }

}
        self.volcanic_globs.retain(|g| g.life > 0.0 && g.x >= 0.0 && g.x < cols as f32 && g.y < rows as f32);
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        physics::draw_fire(self, grid, cols, rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flame_new() {
        let flame = Flame::new();
        assert_eq!(flame.last_cols, 0);
        assert_eq!(flame.last_rows, 0);
        assert_eq!(flame.sparks.len(), 0);
    }

    #[test]
    fn test_flame_update_and_draw() {
        let mut flame = Flame::new();
        flame.update(Duration::from_millis(16), 80, 24);
        let mut grid = vec![TerminalCell::default(); 80 * 24];
        flame.draw(&mut grid, 80, 24);
        // Ensure state variables get initialized
        assert_eq!(flame.last_cols, 80);
        assert_eq!(flame.last_rows, 24);
        assert!(!flame.stars.is_empty());
    }
}

