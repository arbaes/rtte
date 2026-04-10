// Pour effect — faithful TTE reimplementation
// Characters pour from top, falling to their positions with varied speeds

use crate::engine::Grid;
use crate::easing;
use crate::gradient::{Gradient, Rgb, GradientDirection};
use rand::Rng;

struct PourChar {
    final_y: usize,
    final_x: usize,
    start_y: f64,
    cur_y: f64,
    original_ch: char,
    final_color: Rgb,
    progress: f64,
    speed: f64,
    active: bool,
    done: bool,
}

pub struct PourEffect {
    chars: Vec<PourChar>,
    // Groups: columns of chars sorted by row, alternating direction
    pending: Vec<usize>,
    pour_speed: usize,
    gap: usize,
    gap_counter: usize,
    width: usize,
    height: usize,
}

impl PourEffect {
    pub fn new(grid: &Grid) -> Self {
        let width = grid.width;
        let height = grid.height;
        let dm: usize = 2;

        let final_gradient = Gradient::new(
            &[Rgb::from_hex("8A008A"), Rgb::from_hex("00D1FF"), Rgb::from_hex("FFFFFF")],
            12,
        );

        let mut rng = rand::thread_rng();

        let mut chars = Vec::with_capacity(width * height);

        // Pour down: start from top, group by column
        for x in 0..width {
            for y in 0..height {
                let final_color = final_gradient.color_at_coord(
                    y, x, height, width, GradientDirection::Vertical,
                );
                let speed_val: f64 = rng.gen_range(0.4..0.6);
                let start_y = -1.0;
                let dist = (y as f64 - start_y).abs().max(1.0);
                let speed = (speed_val / dist) / dm as f64;

                chars.push(PourChar {
                    final_y: y,
                    final_x: x,
                    start_y,
                    cur_y: start_y,
                    original_ch: grid.cells[y][x].ch,
                    final_color,
                    progress: 0.0,
                    speed,
                    active: false,
                    done: false,
                });
            }
        }

        // Build pending list: alternating column order (even cols forward, odd reversed)
        let mut pending = Vec::new();
        for x in 0..width {
            let base = x * height;
            if x % 2 == 0 {
                for y in 0..height {
                    pending.push(base + y);
                }
            } else {
                for y in (0..height).rev() {
                    pending.push(base + y);
                }
            }
        }

        PourEffect {
            chars,
            pending,
            pour_speed: 2,
            gap: 1 * dm,
            gap_counter: 0,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        // Activate characters
        if !self.pending.is_empty() {
            if self.gap_counter == 0 {
                for _ in 0..self.pour_speed {
                    if let Some(idx) = self.pending.first().copied() {
                        self.pending.remove(0);
                        self.chars[idx].active = true;
                    }
                }
                self.gap_counter = self.gap;
            } else {
                self.gap_counter -= 1;
            }
        }

        // Tick movement
        let mut all_done = self.pending.is_empty();
        for ch in &mut self.chars {
            if !ch.active || ch.done { continue; }
            ch.progress += ch.speed;
            if ch.progress >= 1.0 {
                ch.progress = 1.0;
                ch.done = true;
            }
            let eased = easing::in_quad(ch.progress);
            ch.cur_y = ch.start_y + (ch.final_y as f64 - ch.start_y) * eased;
            if !ch.done { all_done = false; }
        }

        // Render
        for row in &mut grid.cells {
            for cell in row {
                cell.visible = false;
            }
        }

        for ch in &self.chars {
            if !ch.active { continue; }
            let ry = ch.cur_y.round() as isize;
            if ry >= 0 && (ry as usize) < self.height && ch.final_x < self.width {
                let cell = &mut grid.cells[ry as usize][ch.final_x];
                cell.visible = true;
                cell.ch = ch.original_ch;
                let start_c = Rgb::new(255, 255, 255);
                let t = ch.progress;
                cell.fg = Some(Rgb::lerp(start_c, ch.final_color, t).to_crossterm());
            }
        }

        if all_done {
            for ch in &self.chars {
                if ch.final_y < grid.height && ch.final_x < grid.width {
                    let cell = &mut grid.cells[ch.final_y][ch.final_x];
                    cell.visible = true;
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.final_color.to_crossterm());
                }
            }
            return true;
        }
        false
    }
}
