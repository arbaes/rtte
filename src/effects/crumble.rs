// Crumble effect — weaken, dust fall, vacuum up, reset to position

pub const NAME: &str = "crumble";
pub const DESCRIPTION: &str =
    "Characters lose color and crumble into dust, vacuumed up, and reformed.";
pub const EXTRA_EFFECT: bool = false;

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::Rng;

#[derive(Clone, Copy, PartialEq)]
enum CrumblePhase {
    Weakening,
    Falling,
    Vacuuming,
    Resetting,
    Done,
}

struct CrumbleChar {
    final_y: usize,
    final_x: usize,
    cur_y: f64,
    original_ch: char,
    final_color: Rgb,
    phase: CrumblePhase,
    frame: usize,
    progress: f64,
    speed: f64,
    fall_delay: usize,
    delay_count: usize,
}

pub struct CrumbleEffect {
    chars: Vec<CrumbleChar>,
    dm: usize,
    width: usize,
    height: usize,
}

impl CrumbleEffect {
    pub fn new(grid: &Grid) -> Self {
        let (width, height, dm) = (grid.width, grid.height, 2usize);
        let final_gradient = Gradient::new(&[Rgb::from_hex("5CE1FF"), Rgb::from_hex("FF8C00")], 12);
        let mut rng = rand::thread_rng();
        let mut chars = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let fc =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Diagonal);
                chars.push(CrumbleChar {
                    final_y: y,
                    final_x: x,
                    cur_y: y as f64,
                    original_ch: grid.cells[y][x].ch,
                    final_color: fc,
                    phase: CrumblePhase::Weakening,
                    frame: 0,
                    progress: 0.0,
                    speed: 0.65 / dm as f64,
                    fall_delay: rng.gen_range(9..=12) * dm,
                    delay_count: 0,
                });
            }
        }
        CrumbleEffect {
            chars,
            dm,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        let dm = self.dm;
        let mut all_done = true;
        for ch in &mut self.chars {
            match ch.phase {
                CrumblePhase::Weakening => {
                    ch.frame += 1;
                    if ch.frame >= 9 * dm {
                        ch.delay_count += 1;
                        if ch.delay_count >= ch.fall_delay {
                            ch.phase = CrumblePhase::Falling;
                            ch.progress = 0.0;
                            ch.cur_y = ch.final_y as f64;
                        }
                    }
                    all_done = false;
                }
                CrumblePhase::Falling => {
                    ch.progress += ch.speed / (self.height as f64).max(1.0);
                    if ch.progress >= 1.0 {
                        ch.phase = CrumblePhase::Vacuuming;
                        ch.progress = 0.0;
                        ch.cur_y = self.height as f64;
                    } else {
                        let eased = easing::out_bounce(ch.progress);
                        ch.cur_y =
                            ch.final_y as f64 + (self.height as f64 - ch.final_y as f64) * eased;
                    }
                    all_done = false;
                }
                CrumblePhase::Vacuuming => {
                    ch.progress += 1.0 / (self.height as f64 * dm as f64).max(1.0);
                    if ch.progress >= 1.0 {
                        ch.phase = CrumblePhase::Resetting;
                        ch.progress = 0.0;
                        ch.cur_y = -1.0;
                    } else {
                        let eased = easing::out_quint(ch.progress);
                        ch.cur_y = self.height as f64 + (-1.0 - self.height as f64) * eased;
                    }
                    all_done = false;
                }
                CrumblePhase::Resetting => {
                    ch.progress += 0.5 / (self.height as f64 * dm as f64).max(1.0);
                    if ch.progress >= 1.0 {
                        ch.phase = CrumblePhase::Done;
                        ch.cur_y = ch.final_y as f64;
                    } else {
                        ch.cur_y = -1.0 + (ch.final_y as f64 + 1.0) * ch.progress;
                    }
                    if ch.phase != CrumblePhase::Done {
                        all_done = false;
                    }
                }
                CrumblePhase::Done => {}
            }
        }

        for row in &mut grid.cells {
            for cell in row {
                cell.visible = false;
            }
        }
        let dust_symbols = ['*', '.', ','];
        let mut rng = rand::thread_rng();

        for ch in &self.chars {
            let ry = ch.cur_y.round() as isize;
            if ry < 0 {
                continue;
            }
            let ry = ry as usize;
            if ry >= self.height || ch.final_x >= self.width {
                continue;
            }
            let cell = &mut grid.cells[ry][ch.final_x];
            cell.visible = true;
            match ch.phase {
                CrumblePhase::Weakening => {
                    cell.ch = ch.original_ch;
                    let t = ch.frame as f64 / (9.0 * dm as f64);
                    let weak = ch.final_color.adjust_brightness(0.65 - 0.1 * t.min(1.0));
                    cell.fg = Some(weak.to_crossterm());
                }
                CrumblePhase::Falling => {
                    cell.ch = dust_symbols[rng.gen_range(0..dust_symbols.len())];
                    cell.fg = Some(ch.final_color.adjust_brightness(0.55).to_crossterm());
                }
                CrumblePhase::Vacuuming => {
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.final_color.adjust_brightness(0.55).to_crossterm());
                }
                CrumblePhase::Resetting => {
                    cell.ch = ch.original_ch;
                    let t = ch.progress;
                    let c = Rgb::lerp(Rgb::new(255, 255, 255), ch.final_color, t);
                    cell.fg = Some(c.to_crossterm());
                }
                CrumblePhase::Done => {
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.final_color.to_crossterm());
                }
            }
        }
        all_done
    }
}
