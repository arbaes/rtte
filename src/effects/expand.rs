// Expand effect — faithful TTE reimplementation
// Characters expand from center to positions with gradient animation

use crate::engine::Grid;
use crate::easing;
use crate::gradient::{Gradient, Rgb, GradientDirection};

struct ExpandChar {
    final_y: usize,
    final_x: usize,
    cur_y: f64,
    cur_x: f64,
    original_ch: char,
    final_color: Rgb,
    progress: f64,
    speed: f64,
    done: bool,
}

pub struct ExpandEffect {
    chars: Vec<ExpandChar>,
    center_y: f64,
    center_x: f64,
    width: usize,
    height: usize,
}

impl ExpandEffect {
    pub fn new(grid: &Grid) -> Self {
        let width = grid.width;
        let height = grid.height;
        let dm: usize = 2;

        let final_gradient = Gradient::new(
            &[Rgb::from_hex("8A008A"), Rgb::from_hex("00D1FF"), Rgb::from_hex("FFFFFF")],
            12,
        );

        let center_y = height as f64 / 2.0;
        let center_x = width as f64 / 2.0;
        let movement_speed = 0.35;

        let mut chars = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let final_color = final_gradient.color_at_coord(
                    y, x, height, width, GradientDirection::Vertical,
                );
                let dist = ((y as f64 - center_y).powi(2) + (x as f64 - center_x).powi(2)).sqrt().max(1.0);
                let speed = (movement_speed / dist) / dm as f64;

                chars.push(ExpandChar {
                    final_y: y,
                    final_x: x,
                    cur_y: center_y,
                    cur_x: center_x,
                    original_ch: grid.cells[y][x].ch,
                    final_color,
                    progress: 0.0,
                    speed,
                    done: false,
                });
            }
        }

        ExpandEffect { chars, center_y, center_x, width, height }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        let mut all_done = true;

        for ch in &mut self.chars {
            if ch.done { continue; }
            ch.progress += ch.speed;
            if ch.progress >= 1.0 {
                ch.progress = 1.0;
                ch.done = true;
            }
            let eased = easing::in_out_quart(ch.progress);
            ch.cur_y = self.center_y + (ch.final_y as f64 - self.center_y) * eased;
            ch.cur_x = self.center_x + (ch.final_x as f64 - self.center_x) * eased;
            if !ch.done { all_done = false; }
        }

        // Render
        for row in &mut grid.cells {
            for cell in row { cell.visible = false; }
        }

        for ch in &self.chars {
            let ry = ch.cur_y.round() as isize;
            let rx = ch.cur_x.round() as isize;
            if ry < 0 || rx < 0 { continue; }
            let (ry, rx) = (ry as usize, rx as usize);
            if ry >= self.height || rx >= self.width { continue; }
            let cell = &mut grid.cells[ry][rx];
            cell.visible = true;
            cell.ch = ch.original_ch;
            let start = Rgb::from_hex("8A008A");
            cell.fg = Some(Rgb::lerp(start, ch.final_color, ch.progress).to_crossterm());
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
