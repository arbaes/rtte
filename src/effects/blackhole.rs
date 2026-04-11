// Blackhole effect — faithful TTE reimplementation
// Characters pulled into center, consumed, then explode outward

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::Rng;

const STAR_COLORS: [Rgb; 6] = [
    Rgb {
        r: 0xff,
        g: 0xcc,
        b: 0x0d,
    },
    Rgb {
        r: 0xff,
        g: 0x73,
        b: 0x26,
    },
    Rgb {
        r: 0xff,
        g: 0x19,
        b: 0x4d,
    },
    Rgb {
        r: 0xbf,
        g: 0x26,
        b: 0x69,
    },
    Rgb {
        r: 0x70,
        g: 0x2a,
        b: 0x8c,
    },
    Rgb {
        r: 0x04,
        g: 0x9d,
        b: 0xbf,
    },
];

#[derive(Clone, Copy, PartialEq)]
enum BHPhase {
    Forming,
    Consuming,
    Exploding,
    Settling,
    Done,
}

struct BHChar {
    final_y: usize,
    final_x: usize,
    cur_y: f64,
    cur_x: f64,
    original_ch: char,
    final_color: Rgb,
    star_color: Rgb,
    phase: BHPhase,
    progress: f64,
    speed: f64,
    // Explosion target (random nearby before returning)
    explode_y: f64,
    explode_x: f64,
}

pub struct BlackholeEffect {
    chars: Vec<BHChar>,
    center_y: f64,
    center_x: f64,
    frame: usize,
    form_delay: usize,
    form_counter: usize,
    formed_up_to: usize,
    dm: usize,
    width: usize,
    height: usize,
}

impl BlackholeEffect {
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
            9,
        );

        let center_y = height as f64 / 2.0;
        let center_x = width as f64 / 2.0;
        let mut rng = rand::thread_rng();

        let total = width * height;
        let form_delay = (100usize / total.max(1)).max(6) * dm;

        let mut chars = Vec::with_capacity(total);

        // Sort by distance from center for formation order
        let mut positions: Vec<(usize, usize, f64)> = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let d = ((y as f64 - center_y).powi(2) + (x as f64 - center_x).powi(2)).sqrt();
                positions.push((y, x, d));
            }
        }
        positions.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

        for &(y, x, _) in &positions {
            let final_color =
                final_gradient.color_at_coord(y, x, height, width, GradientDirection::Diagonal);
            let star_color = STAR_COLORS[rng.gen_range(0..STAR_COLORS.len())];
            let dist_to_center = ((y as f64 - center_y).powi(2) + (x as f64 - center_x).powi(2))
                .sqrt()
                .max(1.0);
            let consume_speed = (rng.gen_range(0.17..0.30) / dist_to_center) / dm as f64;

            // Random explosion target
            let angle = rng.gen_range(0.0..std::f64::consts::TAU);
            let exp_dist = rng.gen_range(3.0..8.0);
            let explode_y = center_y + angle.sin() * exp_dist;
            let explode_x = center_x + angle.cos() * exp_dist;

            chars.push(BHChar {
                final_y: y,
                final_x: x,
                cur_y: y as f64,
                cur_x: x as f64,
                original_ch: grid.cells[y][x].ch,
                final_color,
                star_color,
                phase: BHPhase::Forming,
                progress: 0.0,
                speed: consume_speed,
                explode_y,
                explode_x,
            });
        }

        BlackholeEffect {
            chars,
            center_y,
            center_x,
            frame: 0,
            form_delay,
            form_counter: 0,
            formed_up_to: 0,
            dm,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        self.frame += 1;
        let dm = self.dm;

        // Form: gradually start consuming chars (closest first)
        if self.formed_up_to < self.chars.len() {
            self.form_counter += 1;
            if self.form_counter >= self.form_delay {
                self.form_counter = 0;
                let batch = (self.chars.len() / 20).max(1);
                let end = (self.formed_up_to + batch).min(self.chars.len());
                for i in self.formed_up_to..end {
                    self.chars[i].phase = BHPhase::Consuming;
                    self.chars[i].progress = 0.0;
                }
                self.formed_up_to = end;
            }
        }

        // Check if all consumed → trigger explosion
        let all_consumed = self
            .chars
            .iter()
            .all(|c| c.phase != BHPhase::Forming && c.phase != BHPhase::Consuming);

        let mut all_done = true;
        let mut rng = rand::thread_rng();

        for ch in &mut self.chars {
            match ch.phase {
                BHPhase::Forming => {
                    all_done = false;
                }
                BHPhase::Consuming => {
                    ch.progress += ch.speed;
                    if ch.progress >= 1.0 {
                        ch.progress = 0.0;
                        ch.phase = BHPhase::Exploding;
                        ch.cur_y = self.center_y;
                        ch.cur_x = self.center_x;
                        let dist = ((ch.explode_y - self.center_y).powi(2)
                            + (ch.explode_x - self.center_x).powi(2))
                        .sqrt()
                        .max(1.0);
                        ch.speed = (rng.gen_range(0.3..0.4) / dist) / dm as f64;
                    } else {
                        let eased = easing::in_expo(ch.progress);
                        ch.cur_y = ch.final_y as f64 + (self.center_y - ch.final_y as f64) * eased;
                        ch.cur_x = ch.final_x as f64 + (self.center_x - ch.final_x as f64) * eased;
                    }
                    all_done = false;
                }
                BHPhase::Exploding => {
                    ch.progress += ch.speed;
                    if ch.progress >= 1.0 {
                        ch.progress = 0.0;
                        ch.phase = BHPhase::Settling;
                        ch.cur_y = ch.explode_y;
                        ch.cur_x = ch.explode_x;
                        let dist = ((ch.final_y as f64 - ch.explode_y).powi(2)
                            + (ch.final_x as f64 - ch.explode_x).powi(2))
                        .sqrt()
                        .max(1.0);
                        ch.speed = (rng.gen_range(0.04..0.06) / dist.sqrt()) / dm as f64;
                    } else {
                        let eased = easing::out_expo(ch.progress);
                        ch.cur_y = self.center_y + (ch.explode_y - self.center_y) * eased;
                        ch.cur_x = self.center_x + (ch.explode_x - self.center_x) * eased;
                    }
                    all_done = false;
                }
                BHPhase::Settling => {
                    ch.progress += ch.speed;
                    if ch.progress >= 1.0 {
                        ch.progress = 1.0;
                        ch.phase = BHPhase::Done;
                        ch.cur_y = ch.final_y as f64;
                        ch.cur_x = ch.final_x as f64;
                    } else {
                        let eased = easing::in_cubic(ch.progress);
                        ch.cur_y = ch.explode_y + (ch.final_y as f64 - ch.explode_y) * eased;
                        ch.cur_x = ch.explode_x + (ch.final_x as f64 - ch.explode_x) * eased;
                    }
                    if ch.phase != BHPhase::Done {
                        all_done = false;
                    }
                }
                BHPhase::Done => {}
            }
        }

        // Render
        for row in &mut grid.cells {
            for cell in row {
                cell.visible = false;
            }
        }

        for ch in &self.chars {
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

            match ch.phase {
                BHPhase::Forming => {
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.final_color.to_crossterm());
                }
                BHPhase::Consuming => {
                    cell.ch = '*';
                    cell.fg = Some(ch.star_color.to_crossterm());
                }
                BHPhase::Exploding => {
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.star_color.to_crossterm());
                }
                BHPhase::Settling => {
                    cell.ch = ch.original_ch;
                    let t = ch.progress;
                    cell.fg = Some(Rgb::lerp(ch.star_color, ch.final_color, t).to_crossterm());
                }
                BHPhase::Done => {
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.final_color.to_crossterm());
                }
            }
        }

        all_done
    }
}
