//! Consolidated flame screensaver effect module.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).


use library::core::{LcgRng, TerminalCell};
use std::time::Duration;
use library::core::screensaver::Screensaver;
use library::core::logo_block::render_logo_block;

use library::platform::native::sys_info::get_system_info;
use library::toolkit::sys_info::query_current_palette;

use library::toolkit::rgb_controller::{RgbController, is_openrgb_enabled};

use library::toolkit::rgb_protocol::RgbColor;

pub struct Spark {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
}

pub struct LogoCell {
    pub x: usize,
    pub y: usize,
    pub ch: char,
    pub temp: f32,
}

pub struct Star {
    pub x: f32,
    pub y: f32,
    pub phase: f32,
    pub ch: char,
    pub excitation: f32,
    pub excited_color: (u8, u8, u8),
}

pub struct VolcanicGlob {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
}

pub fn get_palette(_accent: (u8, u8, u8)) -> [(u8, u8, u8); 36] {
    let mut palette = [(0, 0, 0); 36];
    palette[0] = (0, 0, 0);
    for (i, color) in palette.iter_mut().enumerate().skip(1) {
        if i < 12 {
            // Dark red to bright red
            let t = i as f32 / 12.0;
            *color = (
                (200.0 * t) as u8,
                0,
                0,
            );
        } else if i < 24 {
            // Bright red to vibrant orange/gold
            let t = (i - 12) as f32 / 12.0;
            *color = (
                (200.0 + 55.0 * t) as u8,
                (140.0 * t) as u8,
                0,
            );
        } else if i < 32 {
            // Orange/gold to bright yellow
            let t = (i - 24) as f32 / 8.0;
            *color = (
                255,
                (140.0 + 90.0 * t) as u8,
                (50.0 * t) as u8,
            );
        } else {
            // Bright yellow to white-hot
            let t = (i - 32) as f32 / 3.0;
            *color = (
                255,
                (230.0 + 25.0 * t) as u8,
                (50.0 + 190.0 * t) as u8,
            );
        }
    }
    palette
}



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
    pub(crate) rgb: Option<RgbController>,
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
        let palette = get_palette(accent);

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
            rgb: if is_openrgb_enabled() { Some(RgbController::new()) } else { None },
        }
    }

    fn step_fire(&mut self, cols: usize, rows: usize) {
        // 1. Maintain bottom row (fire source) with dynamic flicker
        let bottom_row_start = (rows - 1) * cols;
        let heat_base = 26.0 + self.cpu_load * 9.0 + self.mem_pressure * 7.0;
        for x in 0..cols {
            let idx = bottom_row_start + x;
            self.fire_grid[idx] = (self.rng.next_range(heat_base, heat_base + 13.0) as u8).min(35);
        }

        // Slightly seed the second row from the bottom to keep the fire thick
        if rows > 2 {
            let second_bottom_start = (rows - 2) * cols;
            for x in 0..cols {
                let idx = second_bottom_start + x;
                if self.rng.next_bool(0.7) {
                    self.fire_grid[idx] = (self.rng.next_range(26.0, 36.0) as u8).min(35);
                }
            }
        }

        // Occasional large fire plumes
        if self.rng.next_bool(0.12) {
            let flare_width = self.rng.next_range(3.0, 8.0) as usize;
            let flare_x = self.rng.next_usize(cols.saturating_sub(flare_width));
            let bottom_row = rows - 1;
            for dx in 0..flare_width {
                let x = flare_x + dx;
                if x < cols {
                    for dy in 0..3 {
                        let y = bottom_row - dy;
                        let idx = y * cols + x;
                        self.fire_grid[idx] = 35;
                    }
                }
            }
        }

        // 2. Propagate fire upwards
        for y in 1..rows {
            for x in 0..cols {
                let src_idx = y * cols + x;
                let fire_val = self.fire_grid[src_idx];

                if fire_val == 0 {
                    let rand_x = self.rng.next_range(-1.0, 2.0) as i32;
                    let dst_x = (x as i32 + rand_x).clamp(0, cols as i32 - 1) as usize;
                    let dst_y = y - 1;
                    self.fire_grid[dst_y * cols + dst_x] = 0;
                } else {
                    let height_ratio = (rows - 1 - y) as f32 / rows as f32;
                    let min_decay = if height_ratio > 0.65 { 1.6 } else if height_ratio > 0.4 { 1.0 } else { 0.4 };
                    let max_decay = if height_ratio > 0.65 { 3.4 } else if height_ratio > 0.4 { 2.4 } else { 1.7 };
                    let decay_mult = match self.flame_height_opt {
                        0 => 3.5f32,
                        2 => 1.3f32,
                        _ => 2.2f32,
                    };
                    let decay = ((self.rng.next_range(min_decay, max_decay) * decay_mult) as u8).max(1);

                    let rand_x = self.rng.next_range(-1.0, 2.0) as i32;
                    let dst_x = (x as i32 + rand_x).clamp(0, cols as i32 - 1) as usize;
                    let dst_y = y - 1;

                    self.fire_grid[dst_y * cols + dst_x] = fire_val.saturating_sub(decay);
                }
            }
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
            self.palette = get_palette(accent);
        }

        // 2. Fixed timestep step for fire cellular automata at 32 Hz
        let physics_step = 0.031;
        while self.physics_accumulator >= physics_step {
            self.physics_accumulator -= physics_step;
            self.step_fire(cols, rows);
        }

        // 3. Update logo temperature
        for cell in &mut self.logo_cells {
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

            if let Some(ref r) = self.rgb {
                r.flash(RgbColor::new(255, 80, 20), std::time::Duration::from_millis(120));
            }
        }
        self.volcanic_globs.retain(|g| g.life > 0.0 && g.x >= 0.0 && g.x < cols as f32 && g.y < rows as f32);
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        draw_fire(self, grid, cols, rows);
    }
}

pub fn draw_fire(effect: &Flame, grid: &mut [TerminalCell], cols: usize, rows: usize) {
    const CHARS: &[char] = &[
        ' ', '.', ':', '-', '=', '+', '*', 'o', 's', 'x', 'z', '#', 'A', '@', '█'
    ];

    // Clear the grid to blank black cells first
    for cell in grid.iter_mut() {
        *cell = TerminalCell {
            ch: ' ',
            fg: (0, 0, 0),
            bg: (0, 0, 0),
            bold: false,
        };
    }

    // Find top candidates for lens flares (only highly excited stars, max 4)
    let mut flare_candidates: Vec<(usize, f32)> = effect.stars.iter()
        .enumerate()
        .filter(|(_, star)| star.excitation > 0.8)
        .map(|(idx, star)| (idx, star.excitation))
        .collect();
    flare_candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
    let allowed_flares: Vec<usize> = flare_candidates.iter()
        .take(4)
        .map(|&(idx, _)| idx)
        .collect();

    // 1. Draw background stars & lens flares (illuminated and excited by sparks)
    for (i, star) in effect.stars.iter().enumerate() {
        let sx = (star.x * cols as f32) as usize;
        let sy = (star.y * rows as f32) as usize;
        if sx < cols && sy < rows {
            // Only draw if there is no fire at this location
            if effect.fire_grid[sy * cols + sx] == 0 {
                // Base twinkle brightness
                let sparkle_base = ((effect.time_elapsed * 2.0 + star.phase).sin() + 1.0) * 0.5;
                let sparkle = (sparkle_base + star.excitation).min(2.0);
                let base_brightness = (sparkle_base * 120.0 + 40.0) as u8;

                let mut r = base_brightness;
                let mut g = base_brightness;
                let mut b = base_brightness.saturating_add(25);

                if star.excitation > 0.05 {
                    let blend = (star.excitation * 0.7).min(1.0);
                    r = (r as f32 * (1.0 - blend) + star.excited_color.0 as f32 * blend).min(255.0) as u8;
                    g = (g as f32 * (1.0 - blend) + star.excited_color.1 as f32 * blend).min(255.0) as u8;
                    b = (b as f32 * (1.0 - blend) + star.excited_color.2 as f32 * blend).min(255.0) as u8;
                }

                let final_brightness = sparkle * 0.4;

                let ch = if final_brightness > 0.8 {
                    '✹'
                } else if final_brightness > 0.5 {
                    '✦'
                } else {
                    star.ch
                };

                grid[sy * cols + sx] = TerminalCell {
                    ch,
                    fg: (r, g, b),
                    bg: (0, 0, 0),
                    bold: final_brightness > 0.6 || star.excitation > 0.3,
                };

                // Draw lens flares and starbursts on highly excited stars
                let is_excited = allowed_flares.contains(&i);
                if is_excited {
                    let flare_intensity = ((star.excitation - 0.8) / 0.7 + 0.5).min(1.5);
                    let flare_color = star.excited_color;

                    // Draw horizontal flare (cinematic anamorphic streak, longer)
                    let h_len = 12;
                    for dx in 1..h_len {
                        let alpha = (120.0f32 * flare_intensity).max(30.0f32) as u8;
                        let fade = alpha.saturating_sub((dx * (110 / h_len)) as u8);
                        if fade > 10 {
                            if sx + dx < cols {
                                let cell = &mut grid[sy * cols + (sx + dx)];
                                if effect.fire_grid[sy * cols + (sx + dx)] == 0 && (cell.ch == ' ' || cell.ch == '─') {
                                    cell.ch = '─';
                                    let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
                                    let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
                                    let fg_b = (fade.saturating_add(45)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
                                    cell.fg = (fg_r, fg_g, fg_b);
                                }
                            }
                            if sx >= dx {
                                let cell = &mut grid[sy * cols + (sx - dx)];
                                if effect.fire_grid[sy * cols + (sx - dx)] == 0 && (cell.ch == ' ' || cell.ch == '─') {
                                    cell.ch = '─';
                                    let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
                                    let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
                                    let fg_b = (fade.saturating_add(45)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
                                    cell.fg = (fg_r, fg_g, fg_b);
                                }
                            }
                        }
                    }

                    // Draw vertical flare
                    let v_len = 5;
                    for dy in 1..v_len {
                        let alpha = (90.0f32 * flare_intensity).max(20.0f32) as u8;
                        let fade = alpha.saturating_sub((dy * (80 / v_len)) as u8);
                        if fade > 10 {
                            if sy + dy < rows {
                                let cell = &mut grid[(sy + dy) * cols + sx];
                                if effect.fire_grid[(sy + dy) * cols + sx] == 0 && (cell.ch == ' ' || cell.ch == '│') {
                                    cell.ch = '│';
                                    let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
                                    let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
                                    let fg_b = (fade.saturating_add(30)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
                                    cell.fg = (fg_r, fg_g, fg_b);
                                }
                            }
                            if sy >= dy {
                                let cell = &mut grid[(sy - dy) * cols + sx];
                                if effect.fire_grid[(sy - dy) * cols + sx] == 0 && (cell.ch == ' ' || cell.ch == '│') {
                                    cell.ch = '│';
                                    let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
                                    let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
                                    let fg_b = (fade.saturating_add(30)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
                                    cell.fg = (fg_r, fg_g, fg_b);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Render fire grid (overlays stars/flares where fire_val > 0)
    for y in 0..rows {
        for x in 0..cols {
            let mut fire_val = effect.fire_grid[y * cols + x] as usize;
            if fire_val > 0 {
                fire_val = fire_val.min(35);
                let char_idx = (fire_val * (CHARS.len() - 1)) / 35;
                let ch = CHARS[char_idx];
                let fg = effect.palette[fire_val];

                grid[y * cols + x] = TerminalCell {
                    ch,
                    fg,
                    bg: (0, 0, 0),
                    bold: fire_val > 14,
                };
            }
        }
    }

    // 3. Overlay rising sparks
    for spark in &effect.sparks {
        let sx = spark.x.round() as i32;
        let sy = spark.y.round() as i32;
        if sx >= 0 && sx < cols as i32 && sy >= 0 && sy < rows as i32 {
            let ux = sx as usize;
            let uy = sy as usize;
            let grid_idx = uy * cols + ux;

            let life_pct = spark.life / spark.max_life;
            let ch = if life_pct > 0.72 {
                '*'
            } else if life_pct > 0.32 {
                '+'
            } else {
                '.'
            };

            let color = if life_pct > 0.75 {
                let t = (life_pct - 0.75) / 0.25;
                (
                    255,
                    (180.0 + 75.0 * t) as u8,
                    (120.0 * t) as u8,
                )
            } else if life_pct > 0.35 {
                let t = (life_pct - 0.35) / 0.40;
                (
                    (180.0 + 75.0 * t) as u8,
                    (t * 180.0) as u8,
                    0,
                )
            } else {
                let t = life_pct / 0.35;
                (
                    (180.0 * t) as u8,
                    0,
                    0,
                )
            };

            let current = &mut grid[grid_idx];
            let current_fire_val = effect.fire_grid[grid_idx];
            if current_fire_val < 10 {
                current.ch = ch;
                current.fg = color;
                current.bold = life_pct > 0.45;
            }
        }
    }

    // 3.5. Overlay active volcanic globs (100% larger with core and envelope)
    for glob in &effect.volcanic_globs {
        let gx = glob.x.round() as i32;
        let gy = glob.y.round() as i32;
        
        let cells = [
            (gx, gy, '●', (255, 255, 200), true),      // Core
            (gx - 1, gy, 'o', (255, 130, 0), true),     // Left
            (gx + 1, gy, 'o', (255, 130, 0), true),     // Right
            (gx, gy - 1, 'o', (255, 130, 0), true),     // Top
            (gx, gy + 1, 'o', (255, 130, 0), true),     // Bottom
        ];

        for &(px, py, ch, fg, bold) in &cells {
            if px >= 0 && px < cols as i32 && py >= 0 && py < rows as i32 {
                let grid_idx = py as usize * cols + px as usize;
                grid[grid_idx] = TerminalCell {
                    ch,
                    fg,
                    bg: (0, 0, 0),
                    bold,
                };
            }
        }
    }

    // 4. Draw logo cells (styled with Windows Theme Accent color)
    for cell in &effect.logo_cells {
        let grid_idx = cell.y * cols + cell.x;
        let temp = cell.temp.min(1.0);

        // library 4.0: pull the accent per-frame from the canonical
        // ScreenPalette. Replaces the pre-4.0 `effect.theme_accent` field
        // so OS theme changes propagate without restarting the saver.
        let mut fg = query_current_palette().accent;
        if temp > 0.1 {
            let t = (temp - 0.1) / 0.9;
            fg.0 = (fg.0 as f32 * (1.0 - t) + 255.0 * t) as u8;
            fg.1 = (fg.1 as f32 * (1.0 - t) + 255.0 * t) as u8;
            fg.2 = (fg.2 as f32 * (1.0 - t) + 180.0 * t) as u8;
        }

        grid[grid_idx] = TerminalCell {
            ch: cell.ch,
            fg,
            bg: (0, 0, 0),
            bold: temp > 0.15,
        };
    }
}
