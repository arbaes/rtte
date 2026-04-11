// ErrorCorrect effect — faithful TTE reimplementation
// Swap pairs of characters, show error state, block wipe, correct with movement

use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Clone, Copy, PartialEq)]
enum PairPhase {
    Waiting,
    ErrorDisplay,
    ErrorGlitch,
    BlockWipeIn,
    Moving,
    BlockWipeOut,
    FinalFade,
    Done,
}

struct SwapChar {
    orig_y: usize,
    orig_x: usize,
    wrong_y: usize,
    wrong_x: usize,
    cur_y: f64,
    cur_x: f64,
    original_ch: char,
    phase: PairPhase,
    frame_count: usize,
    scene_idx: usize,
    final_color: Rgb,
}

const BLOCK_WIPE_IN: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const BLOCK_WIPE_OUT: [char; 7] = ['▇', '▆', '▅', '▄', '▃', '▂', '▁'];

pub struct ErrorCorrectEffect {
    swaps: Vec<(SwapChar, SwapChar)>,
    non_swapped: Vec<(usize, usize, char, Rgb)>,
    swap_delay: usize,
    delay_counter: usize,
    activated_up_to: usize,
    error_color: Rgb,
    correct_color: Rgb,
    move_speed: f64,
    width: usize,
    height: usize,
    dm: usize,
}

impl ErrorCorrectEffect {
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
        let error_color = Rgb::from_hex("e74c3c");
        let correct_color = Rgb::from_hex("45bf55");

        let total_chars = width * height;
        let error_pairs_pct = 0.1_f64;
        let num_pairs = ((total_chars as f64 * error_pairs_pct) as usize / 2).max(1);

        // Collect all positions
        let mut positions: Vec<(usize, usize)> = Vec::new();
        for y in 0..height {
            for x in 0..width {
                positions.push((y, x));
            }
        }
        let mut rng = rand::thread_rng();
        positions.shuffle(&mut rng);

        let mut swapped_set = std::collections::HashSet::new();
        let mut swaps = Vec::new();
        let mut pair_count = 0;

        let mut i = 0;
        while pair_count < num_pairs && i + 1 < positions.len() {
            let (y1, x1) = positions[i];
            let (y2, x2) = positions[i + 1];
            i += 2;

            swapped_set.insert((y1, x1));
            swapped_set.insert((y2, x2));

            let fc1 =
                final_gradient.color_at_coord(y1, x1, height, width, GradientDirection::Vertical);
            let fc2 =
                final_gradient.color_at_coord(y2, x2, height, width, GradientDirection::Vertical);

            let s1 = SwapChar {
                orig_y: y1,
                orig_x: x1,
                wrong_y: y2,
                wrong_x: x2,
                cur_y: y2 as f64,
                cur_x: x2 as f64,
                original_ch: grid.cells[y1][x1].ch,
                phase: PairPhase::Waiting,
                frame_count: 0,
                scene_idx: 0,
                final_color: fc1,
            };
            let s2 = SwapChar {
                orig_y: y2,
                orig_x: x2,
                wrong_y: y1,
                wrong_x: x1,
                cur_y: y1 as f64,
                cur_x: x1 as f64,
                original_ch: grid.cells[y2][x2].ch,
                phase: PairPhase::Waiting,
                frame_count: 0,
                scene_idx: 0,
                final_color: fc2,
            };
            swaps.push((s1, s2));
            pair_count += 1;
        }

        // Non-swapped chars
        let mut non_swapped = Vec::new();
        for y in 0..height {
            for x in 0..width {
                if !swapped_set.contains(&(y, x)) {
                    let fc = final_gradient.color_at_coord(
                        y,
                        x,
                        height,
                        width,
                        GradientDirection::Vertical,
                    );
                    non_swapped.push((y, x, grid.cells[y][x].ch, fc));
                }
            }
        }

        ErrorCorrectEffect {
            swaps,
            non_swapped,
            swap_delay: 6 * dm,
            delay_counter: 0,
            activated_up_to: 0,
            error_color,
            correct_color,
            move_speed: 0.9 / dm as f64,
            width,
            height,
            dm,
        }
    }

    fn tick_swap_char(sc: &mut SwapChar, error_color: Rgb, correct_color: Rgb, dm: usize) {
        sc.frame_count += 1;
        match sc.phase {
            PairPhase::Waiting => {}
            PairPhase::ErrorDisplay => {
                if sc.frame_count >= 1 * dm {
                    sc.phase = PairPhase::ErrorGlitch;
                    sc.frame_count = 0;
                    sc.scene_idx = 0;
                }
            }
            PairPhase::ErrorGlitch => {
                // 20 frames total: alternating ▓/symbol
                if sc.frame_count >= 3 * dm {
                    sc.frame_count = 0;
                    sc.scene_idx += 1;
                    if sc.scene_idx >= 20 {
                        sc.phase = PairPhase::BlockWipeIn;
                        sc.frame_count = 0;
                        sc.scene_idx = 0;
                    }
                }
            }
            PairPhase::BlockWipeIn => {
                if sc.frame_count >= 3 * dm {
                    sc.frame_count = 0;
                    sc.scene_idx += 1;
                    if sc.scene_idx >= BLOCK_WIPE_IN.len() {
                        sc.phase = PairPhase::Moving;
                        sc.frame_count = 0;
                        sc.scene_idx = 0;
                    }
                }
            }
            PairPhase::Moving => {
                // Move from wrong position toward correct position
                let dy = sc.orig_y as f64 - sc.cur_y;
                let dx = sc.orig_x as f64 - sc.cur_x;
                let dist = (dy * dy + dx * dx).sqrt();
                if dist < 0.5 {
                    sc.cur_y = sc.orig_y as f64;
                    sc.cur_x = sc.orig_x as f64;
                    sc.phase = PairPhase::BlockWipeOut;
                    sc.frame_count = 0;
                    sc.scene_idx = 0;
                } else {
                    let speed = 0.9;
                    sc.cur_y += dy / dist * speed;
                    sc.cur_x += dx / dist * speed;
                }
            }
            PairPhase::BlockWipeOut => {
                if sc.frame_count >= 3 * dm {
                    sc.frame_count = 0;
                    sc.scene_idx += 1;
                    if sc.scene_idx >= BLOCK_WIPE_OUT.len() {
                        sc.phase = PairPhase::FinalFade;
                        sc.frame_count = 0;
                        sc.scene_idx = 0;
                    }
                }
            }
            PairPhase::FinalFade => {
                if sc.frame_count >= 3 * dm {
                    sc.frame_count = 0;
                    sc.scene_idx += 1;
                    if sc.scene_idx >= 10 {
                        sc.phase = PairPhase::Done;
                    }
                }
            }
            PairPhase::Done => {}
        }
    }

    fn render_swap_char(sc: &SwapChar, grid: &mut Grid, error_color: Rgb, correct_color: Rgb) {
        let (ry, rx) = match sc.phase {
            PairPhase::Waiting
            | PairPhase::ErrorDisplay
            | PairPhase::ErrorGlitch
            | PairPhase::BlockWipeIn => (sc.wrong_y, sc.wrong_x),
            PairPhase::Moving => (sc.cur_y.round() as usize, sc.cur_x.round() as usize),
            _ => (sc.orig_y, sc.orig_x),
        };

        if ry >= grid.height || rx >= grid.width {
            return;
        }
        let cell = &mut grid.cells[ry][rx];
        cell.visible = true;

        match sc.phase {
            PairPhase::Waiting => {
                cell.ch = sc.original_ch;
                cell.fg = Some(sc.final_color.to_crossterm());
            }
            PairPhase::ErrorDisplay => {
                cell.ch = sc.original_ch;
                cell.fg = Some(error_color.to_crossterm());
            }
            PairPhase::ErrorGlitch => {
                if sc.scene_idx % 2 == 0 {
                    cell.ch = '▓';
                    cell.fg = Some(error_color.to_crossterm());
                } else {
                    cell.ch = sc.original_ch;
                    cell.fg = Some(Rgb::new(255, 255, 255).to_crossterm());
                }
            }
            PairPhase::BlockWipeIn => {
                let idx = sc.scene_idx.min(BLOCK_WIPE_IN.len() - 1);
                cell.ch = BLOCK_WIPE_IN[idx];
                cell.fg = Some(error_color.to_crossterm());
            }
            PairPhase::Moving => {
                let t = {
                    let dy = sc.orig_y as f64 - sc.wrong_y as f64;
                    let dx = sc.orig_x as f64 - sc.wrong_x as f64;
                    let total = (dy * dy + dx * dx).sqrt().max(1.0);
                    let dy2 = sc.orig_y as f64 - sc.cur_y;
                    let dx2 = sc.orig_x as f64 - sc.cur_x;
                    let remaining = (dy2 * dy2 + dx2 * dx2).sqrt();
                    1.0 - (remaining / total)
                };
                cell.ch = '█';
                cell.fg = Some(Rgb::lerp(error_color, correct_color, t).to_crossterm());
            }
            PairPhase::BlockWipeOut => {
                let idx = sc.scene_idx.min(BLOCK_WIPE_OUT.len() - 1);
                cell.ch = BLOCK_WIPE_OUT[idx];
                let color = if idx == BLOCK_WIPE_OUT.len() - 1 {
                    sc.final_color
                } else {
                    correct_color
                };
                cell.fg = Some(color.to_crossterm());
            }
            PairPhase::FinalFade => {
                cell.ch = sc.original_ch;
                let t = sc.scene_idx as f64 / 10.0;
                cell.fg = Some(Rgb::lerp(correct_color, sc.final_color, t).to_crossterm());
            }
            PairPhase::Done => {
                cell.ch = sc.original_ch;
                cell.fg = Some(sc.final_color.to_crossterm());
            }
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        // Activate swap pairs with delay
        if self.activated_up_to < self.swaps.len() {
            if self.delay_counter == 0 {
                let (ref mut s1, ref mut s2) = self.swaps[self.activated_up_to];
                s1.phase = PairPhase::ErrorDisplay;
                s1.frame_count = 0;
                s2.phase = PairPhase::ErrorDisplay;
                s2.frame_count = 0;
                self.activated_up_to += 1;
                self.delay_counter = self.swap_delay;
            } else {
                self.delay_counter -= 1;
            }
        }

        // Tick all activated swaps
        let ec = self.error_color;
        let cc = self.correct_color;
        let dm = self.dm;
        for (s1, s2) in &mut self.swaps {
            Self::tick_swap_char(s1, ec, cc, dm);
            Self::tick_swap_char(s2, ec, cc, dm);
        }

        // Render non-swapped chars
        for &(y, x, ch, fc) in &self.non_swapped {
            if y < grid.height && x < grid.width {
                let cell = &mut grid.cells[y][x];
                cell.visible = true;
                cell.ch = ch;
                cell.fg = Some(fc.to_crossterm());
            }
        }

        // Render swapped chars
        for (s1, s2) in &self.swaps {
            Self::render_swap_char(s1, grid, ec, cc);
            Self::render_swap_char(s2, grid, ec, cc);
        }

        // Check completion
        self.swaps
            .iter()
            .all(|(s1, s2)| s1.phase == PairPhase::Done && s2.phase == PairPhase::Done)
    }
}
