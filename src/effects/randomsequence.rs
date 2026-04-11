// RandomSequence effect — faithful TTE reimplementation
//
// Characters are revealed in random order. Each revealed character plays
// a 7-step gradient animation from starting_color (#000000) to its
// final positional color.

use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::seq::SliceRandom;

struct CharAnim {
    y: usize,
    x: usize,
    original_ch: char,
    // 7-step gradient: starting_color → final_color
    gradient_colors: Vec<Rgb>,
    step: usize,
    hold: usize,
    frames_per_step: usize,
    active: bool,
    done: bool,
    final_color: Rgb,
}

impl CharAnim {
    fn tick(&mut self) {
        if !self.active || self.done {
            return;
        }
        self.hold += 1;
        if self.hold >= self.frames_per_step {
            self.hold = 0;
            self.step += 1;
            if self.step >= self.gradient_colors.len() {
                self.step = self.gradient_colors.len() - 1;
                self.done = true;
            }
        }
    }

    fn current_color(&self) -> Rgb {
        if self.gradient_colors.is_empty() {
            return self.final_color;
        }
        self.gradient_colors[self.step]
    }
}

pub struct RandomSequenceEffect {
    chars: Vec<CharAnim>,     // flat list of all chars
    reveal_order: Vec<usize>, // indices into chars, shuffled
    reveal_pos: usize,        // how far through reveal_order we've gone
    chars_per_tick: usize,
    width: usize,
    height: usize,
}

impl RandomSequenceEffect {
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

        let starting_color = Rgb::new(0, 0, 0);
        let gradient_steps = 7;
        let frames_per_step = 8 * dm;

        let total_chars = width * height;

        let mut chars: Vec<CharAnim> = Vec::with_capacity(total_chars);
        for y in 0..height {
            for x in 0..width {
                let final_color =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);

                // Build 7-step gradient from starting_color to final_color
                let mut gradient_colors = Vec::with_capacity(gradient_steps);
                for i in 0..gradient_steps {
                    let t = (i + 1) as f64 / gradient_steps as f64;
                    gradient_colors.push(Rgb::lerp(starting_color, final_color, t));
                }

                chars.push(CharAnim {
                    y,
                    x,
                    original_ch: grid.cells[y][x].ch,
                    gradient_colors,
                    step: 0,
                    hold: 0,
                    frames_per_step,
                    active: false,
                    done: false,
                    final_color,
                });
            }
        }

        // Shuffle reveal order
        let mut reveal_order: Vec<usize> = (0..total_chars).collect();
        let mut rng = rand::thread_rng();
        reveal_order.shuffle(&mut rng);

        // speed=0.007 → chars_per_tick = max(0.007 * total, 1)
        let chars_per_tick = ((0.007 * total_chars as f64).round() as usize).max(1);

        RandomSequenceEffect {
            chars,
            reveal_order,
            reveal_pos: 0,
            chars_per_tick,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        // Reveal next batch of characters
        let end = (self.reveal_pos + self.chars_per_tick).min(self.reveal_order.len());
        for i in self.reveal_pos..end {
            let idx = self.reveal_order[i];
            self.chars[idx].active = true;
        }
        self.reveal_pos = end;

        // Tick all active animations
        let all_revealed = self.reveal_pos >= self.reveal_order.len();
        let mut all_done = all_revealed;
        for ca in &mut self.chars {
            ca.tick();
            if ca.active && !ca.done {
                all_done = false;
            }
        }

        // Render to grid
        for ca in &self.chars {
            if ca.y < grid.height && ca.x < grid.width {
                let cell = &mut grid.cells[ca.y][ca.x];
                if ca.active {
                    cell.visible = true;
                    cell.ch = ca.original_ch;
                    cell.fg = Some(ca.current_color().to_crossterm());
                }
            }
        }

        if all_done {
            // Set final colors
            for ca in &self.chars {
                if ca.y < grid.height && ca.x < grid.width {
                    let cell = &mut grid.cells[ca.y][ca.x];
                    cell.visible = true;
                    cell.ch = ca.original_ch;
                    cell.fg = Some(ca.final_color.to_crossterm());
                }
            }
            return true;
        }

        false
    }
}
