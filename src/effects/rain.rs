// Rain effect — faithful TTE reimplementation
// Characters fall from top with rain symbols, fade to final color on landing

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::Rng;

const RAIN_SYMBOLS: [char; 5] = ['o', '.', ',', '*', '|'];

struct RainChar {
    final_y: usize,
    final_x: usize,
    start_y: f64,
    cur_y: f64,
    original_ch: char,
    final_color: Rgb,
    rain_color: Rgb,
    rain_symbol: char,
    progress: f64,
    speed: f64,
    active: bool,
    landed: bool,
    // Fade scene after landing
    fade_frame: usize,
    fade_total: usize,
    fade_done: bool,
}

pub struct RainEffect {
    chars: Vec<RainChar>,
    groups: Vec<Vec<usize>>, // grouped by row
    gap_counter: usize,
    gap: usize,
    activated_up_to: usize,
    width: usize,
    height: usize,
}

impl RainEffect {
    pub fn new(grid: &Grid) -> Self {
        let width = grid.width;
        let height = grid.height;
        let dm: usize = 2;

        let final_gradient = Gradient::new(
            &[
                Rgb::from_hex("488bff"),
                Rgb::from_hex("b2e7de"),
                Rgb::from_hex("57eaf7"),
            ],
            12,
        );
        let rain_gradient = Gradient::new(&[Rgb::from_hex("00315C"), Rgb::from_hex("E3EFFC")], 8);

        let mut rng = rand::thread_rng();
        let mut chars = Vec::with_capacity(width * height);
        let mut groups: Vec<Vec<usize>> = vec![Vec::new(); height];

        for (y, group) in groups.iter_mut().enumerate() {
            for x in 0..width {
                let final_color =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Diagonal);
                let rain_color =
                    rain_gradient.spectrum()[rng.gen_range(0..rain_gradient.spectrum().len())];
                let rain_symbol = RAIN_SYMBOLS[rng.gen_range(0..RAIN_SYMBOLS.len())];
                let speed_val: f64 = rng.gen_range(0.33..0.57);
                let start_y = -1.0;
                let dist = (y as f64 - start_y).abs().max(1.0);
                let speed = (speed_val / dist) / dm as f64;

                let idx = chars.len();
                group.push(idx);

                chars.push(RainChar {
                    final_y: y,
                    final_x: x,
                    start_y,
                    cur_y: start_y,
                    original_ch: grid.cells[y][x].ch,
                    final_color,
                    rain_color,
                    rain_symbol,
                    progress: 0.0,
                    speed,
                    active: false,
                    landed: false,
                    fade_frame: 0,
                    fade_total: 7 * 3 * dm,
                    fade_done: false,
                });
            }
        }

        RainEffect {
            chars,
            groups,
            gap_counter: 0,
            gap: 2 * dm,
            activated_up_to: 0,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        // Activate row groups with gap
        if self.activated_up_to < self.groups.len() {
            if self.gap_counter == 0 {
                for &idx in &self.groups[self.activated_up_to] {
                    self.chars[idx].active = true;
                }
                self.activated_up_to += 1;
                self.gap_counter = self.gap;
            } else {
                self.gap_counter -= 1;
            }
        }

        // Tick
        let mut all_done = self.activated_up_to >= self.groups.len();
        for ch in &mut self.chars {
            if !ch.active {
                continue;
            }
            if !ch.landed {
                ch.progress += ch.speed;
                if ch.progress >= 1.0 {
                    ch.progress = 1.0;
                    ch.landed = true;
                }
                let eased = easing::in_quart(ch.progress);
                ch.cur_y = ch.start_y + (ch.final_y as f64 - ch.start_y) * eased;
            } else if !ch.fade_done {
                ch.fade_frame += 1;
                if ch.fade_frame >= ch.fade_total {
                    ch.fade_done = true;
                }
            }
            if !ch.fade_done {
                all_done = false;
            }
        }

        // Render
        for row in &mut grid.cells {
            for cell in row {
                cell.visible = false;
            }
        }

        for ch in &self.chars {
            if !ch.active {
                continue;
            }
            if ch.landed {
                if ch.final_y < self.height && ch.final_x < self.width {
                    let cell = &mut grid.cells[ch.final_y][ch.final_x];
                    cell.visible = true;
                    cell.ch = ch.original_ch;
                    let t = if ch.fade_done {
                        1.0
                    } else {
                        ch.fade_frame as f64 / ch.fade_total as f64
                    };
                    cell.fg = Some(Rgb::lerp(ch.rain_color, ch.final_color, t).to_crossterm());
                }
            } else {
                let ry = ch.cur_y.round() as isize;
                if ry >= 0 && (ry as usize) < self.height && ch.final_x < self.width {
                    let cell = &mut grid.cells[ry as usize][ch.final_x];
                    cell.visible = true;
                    cell.ch = ch.rain_symbol;
                    cell.fg = Some(ch.rain_color.to_crossterm());
                }
            }
        }

        all_done
    }
}
