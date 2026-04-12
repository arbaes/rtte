// Scattered effect — faithful TTE reimplementation
//
// Characters start at random positions and converge to their final positions
// using in_out_back easing. Color transitions from gradient start to final
// positional color, synced to movement distance.

pub const NAME: &str = "scattered";
pub const DESCRIPTION: &str = "Text is scattered across the canvas and moves into position.";
pub const EXTRA_EFFECT: bool = false;

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::Rng;

struct CharMotion {
    // Final position (where the char belongs in the text)
    final_y: usize,
    final_x: usize,
    original_ch: char,
    // Random starting position (floating point for smooth motion)
    start_y: f64,
    start_x: f64,
    // Current position
    cur_y: f64,
    cur_x: f64,
    // Movement
    speed: f64,
    progress: f64, // 0.0 → 1.0
    done: bool,
    // Color: 10-step gradient from spectrum start to final color
    color_steps: Vec<Rgb>,
    final_color: Rgb,
}

pub struct ScatteredEffect {
    chars: Vec<CharMotion>,
    hold_frames: usize,
    hold_count: usize,
    width: usize,
    height: usize,
    started: bool,
}

impl ScatteredEffect {
    pub fn new(grid: &Grid) -> Self {
        let width = grid.width;
        let height = grid.height;
        let dm: usize = 2;

        let final_gradient = Gradient::new(
            &[
                Rgb::from_hex("ff9048"),
                Rgb::from_hex("ab9dff"),
                Rgb::from_hex("bdffea"),
            ],
            12,
        );

        let mut rng = rand::thread_rng();
        let movement_speed = 0.5; // TTE default

        // Per-character speed: TTE movement speed maps to progress increment per frame
        // At speed=0.5, character should take roughly 60-80 frames to travel across canvas
        // TTE speed is pixels/tick, so we compute distance-based progress
        let mut chars = Vec::with_capacity(width * height);

        for y in 0..height {
            for x in 0..width {
                let original_ch = grid.cells[y][x].ch;
                let final_color =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);

                // Random starting position
                let start_y = if height > 1 {
                    rng.gen_range(0..height) as f64
                } else {
                    0.0
                };
                let start_x = if width > 1 {
                    rng.gen_range(0..width) as f64
                } else {
                    0.0
                };

                // Distance from start to final
                let dy = y as f64 - start_y;
                let dx = x as f64 - start_x;
                let dist = (dy * dy + dx * dx).sqrt().max(1.0);

                // Speed as progress per frame: movement_speed pixels/frame → progress = speed/dist
                // Scale for dm (60fps)
                let speed = (movement_speed / dist) / dm as f64;

                // 10-step color gradient from spectrum[0] (#ff9048) to final color
                let start_color = Rgb::from_hex("ff9048");
                let steps = 10;
                let mut color_steps = Vec::with_capacity(steps);
                for i in 0..steps {
                    let t = (i + 1) as f64 / steps as f64;
                    color_steps.push(Rgb::lerp(start_color, final_color, t));
                }

                chars.push(CharMotion {
                    final_y: y,
                    final_x: x,
                    original_ch,
                    start_y,
                    start_x,
                    cur_y: start_y,
                    cur_x: start_x,
                    speed,
                    progress: 0.0,
                    done: false,
                    color_steps,
                    final_color,
                });
            }
        }

        ScatteredEffect {
            chars,
            hold_frames: 25 * dm,
            hold_count: 0,
            width,
            height,
            started: false,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        // Initial hold phase
        if self.hold_count < self.hold_frames {
            self.hold_count += 1;
            // Show all chars at their random starting positions
            // Clear grid first
            for row in &mut grid.cells {
                for cell in row {
                    cell.visible = false;
                }
            }
            for cm in &self.chars {
                let ry = cm.start_y.round() as usize;
                let rx = cm.start_x.round() as usize;
                if ry < self.height && rx < self.width {
                    let cell = &mut grid.cells[ry][rx];
                    cell.visible = true;
                    cell.ch = cm.original_ch;
                    cell.fg = Some(cm.color_steps[0].to_crossterm());
                }
            }
            return false;
        }

        // Movement phase: advance each character toward its target
        let mut all_done = true;
        for cm in &mut self.chars {
            if cm.done {
                continue;
            }
            cm.progress += cm.speed;
            if cm.progress >= 1.0 {
                cm.progress = 1.0;
                cm.done = true;
            }
            let eased = easing::in_out_back(cm.progress);
            cm.cur_y = cm.start_y + (cm.final_y as f64 - cm.start_y) * eased;
            cm.cur_x = cm.start_x + (cm.final_x as f64 - cm.start_x) * eased;
            if !cm.done {
                all_done = false;
            }
        }

        // Render: clear grid, then place chars at current positions
        for row in &mut grid.cells {
            for cell in row {
                cell.visible = false;
            }
        }

        for cm in &self.chars {
            let ry = cm.cur_y.round() as usize;
            let rx = cm.cur_x.round() as usize;
            if ry < self.height && rx < self.width {
                let cell = &mut grid.cells[ry][rx];
                cell.visible = true;
                cell.ch = cm.original_ch;

                // Color synced to distance progress
                let color_idx = (cm.progress * (cm.color_steps.len() - 1) as f64).round() as usize;
                let color_idx = color_idx.min(cm.color_steps.len() - 1);
                cell.fg = Some(cm.color_steps[color_idx].to_crossterm());
            }
        }

        if all_done {
            // Final frame: all chars at their correct positions with final colors
            for cm in &self.chars {
                if cm.final_y < grid.height && cm.final_x < grid.width {
                    let cell = &mut grid.cells[cm.final_y][cm.final_x];
                    cell.visible = true;
                    cell.ch = cm.original_ch;
                    cell.fg = Some(cm.final_color.to_crossterm());
                }
            }
            return true;
        }

        false
    }
}
