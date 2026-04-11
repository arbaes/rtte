// MiddleOut effect — faithful TTE reimplementation
// Characters expand from center: first to center line, then to final positions

pub const NAME: &str = "middleout";
pub const DESCRIPTION: &str =
    "Text expands in a single row or column in the middle of the canvas then out.";

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};

#[derive(PartialEq)]
enum Phase {
    Center,
    Full,
    Complete,
}

struct MiddleChar {
    final_y: usize,
    final_x: usize,
    // Phase 1: center → center line position
    mid_y: f64,
    mid_x: f64,
    // Phase 2: center line → final
    original_ch: char,
    cur_y: f64,
    cur_x: f64,
    final_color: Rgb,
    progress: f64,
    speed: f64,
    done_p1: bool,
    done_p2: bool,
}

pub struct MiddleOutEffect {
    chars: Vec<MiddleChar>,
    center_y: f64,
    center_x: f64,
    phase: Phase,
    dm: usize,
    width: usize,
    height: usize,
}

impl MiddleOutEffect {
    pub fn new(grid: &Grid) -> Self {
        let width = grid.width;
        let height = grid.height;
        let dm: usize = 2;

        let final_gradient = Gradient::new(
            &[
                Rgb::from_hex("8A008A"),
                Rgb::from_hex("00D1FF"),
                Rgb::from_hex("FFFFFF"),
            ],
            12,
        );

        let center_y = height as f64 / 2.0;
        let center_x = width as f64 / 2.0;
        let center_speed = 0.6;

        let mut chars = Vec::with_capacity(width * height);

        for y in 0..height {
            for x in 0..width {
                let final_color =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);
                // Phase 1 target: same column, center row (vertical expand)
                let mid_y = center_y;
                let mid_x = x as f64;

                // Speed based on distance from center to mid position
                let d1 = ((center_y - mid_y).powi(2) + (center_x - mid_x).powi(2))
                    .sqrt()
                    .max(1.0);
                let speed1 = (center_speed / d1) / dm as f64;

                chars.push(MiddleChar {
                    final_y: y,
                    final_x: x,
                    mid_y,
                    mid_x,
                    original_ch: grid.cells[y][x].ch,
                    cur_y: center_y,
                    cur_x: center_x,
                    final_color,
                    progress: 0.0,
                    speed: speed1,
                    done_p1: false,
                    done_p2: false,
                });
            }
        }

        MiddleOutEffect {
            chars,
            center_y,
            center_x,
            phase: Phase::Center,
            dm,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        match self.phase {
            Phase::Center => {
                let mut all_done = true;
                for ch in &mut self.chars {
                    if ch.done_p1 {
                        continue;
                    }
                    ch.progress += ch.speed;
                    if ch.progress >= 1.0 {
                        ch.progress = 1.0;
                        ch.done_p1 = true;
                    }
                    let eased = easing::in_out_sine(ch.progress);
                    ch.cur_y = self.center_y + (ch.mid_y - self.center_y) * eased;
                    ch.cur_x = self.center_x + (ch.mid_x - self.center_x) * eased;
                    if !ch.done_p1 {
                        all_done = false;
                    }
                }
                if all_done {
                    self.phase = Phase::Full;
                    let full_speed = 0.6;
                    for ch in &mut self.chars {
                        ch.progress = 0.0;
                        let d = ((ch.final_y as f64 - ch.mid_y).powi(2)
                            + (ch.final_x as f64 - ch.mid_x).powi(2))
                        .sqrt()
                        .max(1.0);
                        ch.speed = (full_speed / d) / self.dm as f64;
                    }
                }
            }
            Phase::Full => {
                let mut all_done = true;
                for ch in &mut self.chars {
                    if ch.done_p2 {
                        continue;
                    }
                    ch.progress += ch.speed;
                    if ch.progress >= 1.0 {
                        ch.progress = 1.0;
                        ch.done_p2 = true;
                    }
                    let eased = easing::in_out_sine(ch.progress);
                    ch.cur_y = ch.mid_y + (ch.final_y as f64 - ch.mid_y) * eased;
                    ch.cur_x = ch.mid_x + (ch.final_x as f64 - ch.mid_x) * eased;
                    if !ch.done_p2 {
                        all_done = false;
                    }
                }
                if all_done {
                    self.phase = Phase::Complete;
                }
            }
            Phase::Complete => {}
        }

        // Render
        for row in &mut grid.cells {
            for cell in row {
                cell.visible = false;
            }
        }

        for ch in &self.chars {
            let ry = ch.cur_y.round() as usize;
            let rx = ch.cur_x.round() as usize;
            if ry < self.height && rx < self.width {
                let cell = &mut grid.cells[ry][rx];
                cell.visible = true;
                cell.ch = ch.original_ch;
                let white = Rgb::new(255, 255, 255);
                let total_progress = if self.phase == Phase::Center {
                    ch.progress * 0.5
                } else {
                    0.5 + ch.progress * 0.5
                };
                cell.fg = Some(Rgb::lerp(white, ch.final_color, total_progress).to_crossterm());
            }
        }

        if self.phase == Phase::Complete {
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
