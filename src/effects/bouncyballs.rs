// BouncyBalls effect — faithful TTE reimplementation
// Characters drop from above with bounce physics, settle into positions

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::Rng;

const BALL_COLORS: [Rgb; 3] = [
    Rgb {
        r: 0xd1,
        g: 0xf4,
        b: 0xa5,
    },
    Rgb {
        r: 0x96,
        g: 0xe2,
        b: 0xa4,
    },
    Rgb {
        r: 0x5a,
        g: 0xcd,
        b: 0xa9,
    },
];
const BALL_SYMBOLS: [char; 5] = ['*', 'o', 'O', '0', '.'];

struct BallChar {
    final_y: usize,
    final_x: usize,
    start_y: f64,
    cur_y: f64,
    original_ch: char,
    final_color: Rgb,
    ball_color: Rgb,
    ball_symbol: char,
    progress: f64,
    speed: f64,
    active: bool,
    landed: bool,
    // Fade after landing
    fade_frame: usize,
    fade_total: usize,
    done: bool,
}

pub struct BouncyBallsEffect {
    chars: Vec<BallChar>,
    // Groups by row (bottom to top)
    groups: Vec<Vec<usize>>,
    ball_delay: usize,
    delay_counter: usize,
    activated_up_to: usize,
    dm: usize,
    width: usize,
    height: usize,
}

impl BouncyBallsEffect {
    pub fn new(grid: &Grid) -> Self {
        let width = grid.width;
        let height = grid.height;
        let dm: usize = 2;

        let final_gradient = Gradient::new(&[Rgb::from_hex("f8ffae"), Rgb::from_hex("43c6ac")], 12);

        let mut rng = rand::thread_rng();
        let mut chars = Vec::with_capacity(width * height);

        // Group by row, bottom to top
        let mut row_chars: Vec<Vec<usize>> = vec![Vec::new(); height];

        for y in 0..height {
            for x in 0..width {
                let final_color =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Diagonal);
                let start_y = -(rng.gen_range(1.0..1.5) * height as f64);
                let dist = (y as f64 - start_y).abs().max(1.0);
                let speed = (0.45 / dist) / dm as f64;

                let idx = chars.len();
                row_chars[y].push(idx);

                chars.push(BallChar {
                    final_y: y,
                    final_x: x,
                    start_y,
                    cur_y: start_y,
                    original_ch: grid.cells[y][x].ch,
                    final_color,
                    ball_color: BALL_COLORS[rng.gen_range(0..BALL_COLORS.len())],
                    ball_symbol: BALL_SYMBOLS[rng.gen_range(0..BALL_SYMBOLS.len())],
                    progress: 0.0,
                    speed,
                    active: false,
                    landed: false,
                    fade_frame: 0,
                    fade_total: 6 * dm,
                    done: false,
                });
            }
        }

        // Reverse to get bottom rows first
        let mut groups: Vec<Vec<usize>> = Vec::new();
        for y in (0..height).rev() {
            if !row_chars[y].is_empty() {
                groups.push(row_chars[y].clone());
            }
        }

        BouncyBallsEffect {
            chars,
            groups,
            ball_delay: 4 * dm,
            delay_counter: 0,
            activated_up_to: 0,
            dm,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        let mut rng = rand::thread_rng();

        // Activate groups with delay
        if self.activated_up_to < self.groups.len() {
            if self.delay_counter == 0 {
                // Activate 2-6 random from next group
                let group = &self.groups[self.activated_up_to];
                let count = rng.gen_range(2..=6).min(group.len());
                let mut indices: Vec<usize> = group.clone();
                use rand::seq::SliceRandom;
                indices.shuffle(&mut rng);
                for &idx in indices.iter().take(count) {
                    self.chars[idx].active = true;
                }
                // Activate remaining too (they all need to go)
                for &idx in &indices[count..] {
                    self.chars[idx].active = true;
                }
                self.activated_up_to += 1;
                self.delay_counter = self.ball_delay;
            } else {
                self.delay_counter -= 1;
            }
        }

        // Tick
        let mut all_done = self.activated_up_to >= self.groups.len();
        for ch in &mut self.chars {
            if !ch.active || ch.done {
                continue;
            }
            if !ch.landed {
                ch.progress += ch.speed;
                if ch.progress >= 1.0 {
                    ch.progress = 1.0;
                    ch.landed = true;
                }
                let eased = easing::out_bounce(ch.progress);
                ch.cur_y = ch.start_y + (ch.final_y as f64 - ch.start_y) * eased;
            } else {
                ch.fade_frame += 1;
                if ch.fade_frame >= ch.fade_total {
                    ch.done = true;
                }
            }
            if !ch.done {
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
                    let t = if ch.done {
                        1.0
                    } else {
                        ch.fade_frame as f64 / ch.fade_total as f64
                    };
                    cell.fg = Some(Rgb::lerp(ch.ball_color, ch.final_color, t).to_crossterm());
                }
            } else {
                let ry = ch.cur_y.round() as isize;
                if ry >= 0 && (ry as usize) < self.height && ch.final_x < self.width {
                    let cell = &mut grid.cells[ry as usize][ch.final_x];
                    cell.visible = true;
                    cell.ch = ch.ball_symbol;
                    cell.fg = Some(ch.ball_color.to_crossterm());
                }
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
