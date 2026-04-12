// OrbittingVolley — 4 launchers orbit, fire chars inward

pub const NAME: &str = "orbittingvolley";
pub const DESCRIPTION: &str = "Four launchers orbit the canvas firing volleys of characters inward to build the input text from the center out.";
pub const EXTRA_EFFECT: bool = false;

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};

struct OVChar {
    final_y: usize,
    final_x: usize,
    cur_y: f64,
    cur_x: f64,
    original_ch: char,
    final_color: Rgb,
    progress: f64,
    speed: f64,
    active: bool,
    done: bool,
    start_y: f64,
    start_x: f64,
}

pub struct OrbittingVolleyEffect {
    chars: Vec<OVChar>,
    pending: Vec<usize>,
    launchers: [(f64, f64); 4],
    launcher_angles: [f64; 4],
    volley_delay: usize,
    delay_count: usize,
    volley_size: usize,
    dm: usize,
    width: usize,
    height: usize,
}

impl OrbittingVolleyEffect {
    pub fn new(grid: &Grid) -> Self {
        let (width, height, dm) = (grid.width, grid.height, 2usize);
        let final_gradient = Gradient::new(&[Rgb::from_hex("FFA15C"), Rgb::from_hex("44D492")], 12);
        let cy = height as f64 / 2.0;
        let cx = width as f64 / 2.0;
        let launchers = [
            (0.0, cx),
            (cy, width as f64 - 1.0),
            (height as f64 - 1.0, cx),
            (cy, 0.0),
        ];

        // Sort chars center→outside for volley order
        let mut indexed: Vec<(usize, usize, f64)> = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let d = ((y as f64 - cy).powi(2) + (x as f64 - cx).powi(2)).sqrt();
                indexed.push((y, x, d));
            }
        }
        indexed.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

        let mut chars = Vec::with_capacity(width * height);
        let mut pending = Vec::new();
        for (i, &(y, x, _)) in indexed.iter().enumerate() {
            let fc =
                final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);
            pending.push(i);
            chars.push(OVChar {
                final_y: y,
                final_x: x,
                cur_y: 0.0,
                cur_x: 0.0,
                original_ch: grid.cells[y][x].ch,
                final_color: fc,
                progress: 0.0,
                speed: 0.0,
                active: false,
                done: false,
                start_y: 0.0,
                start_x: 0.0,
            });
        }

        let volley_size = ((width * height) as f64 * 0.03).max(1.0) as usize;
        OrbittingVolleyEffect {
            chars,
            pending,
            launchers,
            launcher_angles: [0.0; 4],
            volley_delay: 30 * dm,
            delay_count: 0,
            volley_size,
            dm,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        let dm = self.dm;
        // Orbit launchers
        for i in 0..4 {
            self.launcher_angles[i] += 0.8 * 0.01 / dm as f64;
        }

        // Fire volleys
        if !self.pending.is_empty() {
            self.delay_count += 1;
            if self.delay_count >= self.volley_delay {
                self.delay_count = 0;
                let _rng = rand::thread_rng();
                for li in 0..4 {
                    let (ly, lx) = self.launchers[li];
                    for _ in 0..self.volley_size.min(self.pending.len()) {
                        if self.pending.is_empty() {
                            break;
                        }
                        let idx = self.pending.remove(0);
                        let ch = &mut self.chars[idx];
                        ch.active = true;
                        ch.start_y = ly;
                        ch.start_x = lx;
                        ch.cur_y = ly;
                        ch.cur_x = lx;
                        let dist = ((ch.final_y as f64 - ly).powi(2)
                            + (ch.final_x as f64 - lx).powi(2))
                        .sqrt()
                        .max(1.0);
                        ch.speed = (1.5 / dist) / dm as f64;
                    }
                }
            }
        }

        let mut all_done = self.pending.is_empty();
        for ch in &mut self.chars {
            if !ch.active || ch.done {
                continue;
            }
            ch.progress += ch.speed;
            if ch.progress >= 1.0 {
                ch.progress = 1.0;
                ch.done = true;
            }
            let eased = easing::out_sine(ch.progress);
            ch.cur_y = ch.start_y + (ch.final_y as f64 - ch.start_y) * eased;
            ch.cur_x = ch.start_x + (ch.final_x as f64 - ch.start_x) * eased;
            if !ch.done {
                all_done = false;
            }
        }

        for row in &mut grid.cells {
            for cell in row {
                cell.visible = false;
            }
        }
        for ch in &self.chars {
            if !ch.active {
                continue;
            }
            let ry = ch.cur_y.round() as isize;
            let rx = ch.cur_x.round() as isize;
            if ry < 0 || rx < 0 {
                continue;
            }
            let (ry, rx) = (ry as usize, rx as usize);
            if ry >= self.height || rx >= self.width {
                continue;
            }
            let cell = &mut grid.cells[ry][rx];
            cell.visible = true;
            cell.ch = ch.original_ch;
            cell.fg = Some(
                Rgb::lerp(Rgb::from_hex("FFA15C"), ch.final_color, ch.progress).to_crossterm(),
            );
        }

        // Render launchers
        for i in 0..4 {
            let (ly, lx) = self.launchers[i];
            let ry = ly.round() as usize;
            let rx = lx.round() as usize;
            if ry < self.height && rx < self.width {
                let cell = &mut grid.cells[ry][rx];
                cell.visible = true;
                cell.ch = '█';
                cell.fg = Some(Rgb::new(255, 255, 255).to_crossterm());
            }
        }

        if all_done {
            for ch in &self.chars {
                if ch.final_y < self.height && ch.final_x < self.width {
                    let cell = &mut grid.cells[ch.final_y][ch.final_x];
                    cell.visible = true;
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.final_color.to_crossterm());
                }
            }
        }
        all_done
    }
}
