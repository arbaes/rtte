// Burn effect — faithful TTE reimplementation
// Fire spreads through text with burn symbols, smoke particles

pub const NAME: &str = "burn";
pub const DESCRIPTION: &str = "Burns vertically in the canvas.";
pub const EXTRA_EFFECT: bool = false;

use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use std::collections::VecDeque;

const BURN_CHARS: [char; 9] = ['\'', '.', '▖', '▙', '█', '▜', '▀', '▝', '.'];

#[derive(Clone, Copy, PartialEq)]
enum BurnPhase {
    Waiting,
    Burning,
    Final,
    Done,
}

struct BurnChar {
    y: usize,
    x: usize,
    original_ch: char,
    final_color: Rgb,
    phase: BurnPhase,
    frame: usize,
    burn_total: usize,
    final_total: usize,
}

pub struct BurnEffect {
    chars: Vec<Vec<BurnChar>>,
    bfs_queue: VecDeque<(usize, usize)>,
    visited: Vec<Vec<bool>>,
    burn_gradient: Gradient,
    dm: usize,
    width: usize,
    height: usize,
    started: bool,
}

impl BurnEffect {
    pub fn new(grid: &Grid) -> Self {
        let (width, height, dm) = (grid.width, grid.height, 2usize);
        let final_gradient = Gradient::new(&[Rgb::from_hex("00c3ff"), Rgb::from_hex("ffff1c")], 12);
        let burn_gradient = Gradient::new(
            &[
                Rgb::from_hex("ffffff"),
                Rgb::from_hex("fff75d"),
                Rgb::from_hex("fe650d"),
                Rgb::from_hex("8A003C"),
                Rgb::from_hex("510100"),
            ],
            BURN_CHARS.len() * 4,
        );
        let mut chars = Vec::with_capacity(height);
        for y in 0..height {
            let mut row = Vec::with_capacity(width);
            for x in 0..width {
                let fc =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);
                row.push(BurnChar {
                    y,
                    x,
                    original_ch: grid.cells[y][x].ch,
                    final_color: fc,
                    phase: BurnPhase::Waiting,
                    frame: 0,
                    burn_total: BURN_CHARS.len() * 4 * dm,
                    final_total: 8 * dm,
                });
            }
            chars.push(row);
        }
        BurnEffect {
            chars,
            bfs_queue: VecDeque::new(),
            visited: vec![vec![false; width]; height],
            burn_gradient,
            dm,
            width,
            height,
            started: false,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        let _rng = rand::thread_rng();
        if !self.started {
            // Start from bottom center
            let sy = self.height.saturating_sub(1);
            let sx = self.width / 2;
            self.bfs_queue.push_back((sy, sx));
            self.visited[sy][sx] = true;
            self.chars[sy][sx].phase = BurnPhase::Burning;
            self.started = true;
        }

        // BFS expansion (bottom-up bias)
        for _ in 0..3 {
            if let Some((y, x)) = self.bfs_queue.pop_front() {
                let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                for (dy, dx) in &dirs {
                    let ny = y as isize + dy;
                    let nx = x as isize + dx;
                    if ny >= 0 && nx >= 0 {
                        let (ny, nx) = (ny as usize, nx as usize);
                        if ny < self.height && nx < self.width && !self.visited[ny][nx] {
                            self.visited[ny][nx] = true;
                            self.chars[ny][nx].phase = BurnPhase::Burning;
                            self.bfs_queue.push_back((ny, nx));
                        }
                    }
                }
            }
        }

        let mut all_done = self.bfs_queue.is_empty();
        let spec_len = self.burn_gradient.spectrum().len().max(1);
        let starting_color = Rgb::from_hex("837373");

        for row in &mut self.chars {
            for ch in row {
                match ch.phase {
                    BurnPhase::Waiting => {
                        all_done = false;
                    }
                    BurnPhase::Burning => {
                        ch.frame += 1;
                        if ch.frame >= ch.burn_total {
                            ch.phase = BurnPhase::Final;
                            ch.frame = 0;
                        }
                        all_done = false;
                    }
                    BurnPhase::Final => {
                        ch.frame += 1;
                        if ch.frame >= ch.final_total {
                            ch.phase = BurnPhase::Done;
                        } else {
                            all_done = false;
                        }
                    }
                    BurnPhase::Done => {}
                }
            }
        }

        // Render
        for row in &self.chars {
            for ch in row {
                if ch.y >= grid.height || ch.x >= grid.width {
                    continue;
                }
                let cell = &mut grid.cells[ch.y][ch.x];
                cell.visible = true;
                match ch.phase {
                    BurnPhase::Waiting => {
                        cell.ch = ch.original_ch;
                        cell.fg = Some(starting_color.to_crossterm());
                    }
                    BurnPhase::Burning => {
                        let idx = (ch.frame / (4 * self.dm).max(1)) % BURN_CHARS.len();
                        cell.ch = BURN_CHARS[idx];
                        let ci = (ch.frame * spec_len / ch.burn_total.max(1)).min(spec_len - 1);
                        cell.fg = Some(self.burn_gradient.spectrum()[ci].to_crossterm());
                    }
                    BurnPhase::Final => {
                        cell.ch = ch.original_ch;
                        let t = ch.frame as f64 / ch.final_total as f64;
                        let fire_end = self.burn_gradient.spectrum()[spec_len - 1];
                        cell.fg = Some(Rgb::lerp(fire_end, ch.final_color, t).to_crossterm());
                    }
                    BurnPhase::Done => {
                        cell.ch = ch.original_ch;
                        cell.fg = Some(ch.final_color.to_crossterm());
                    }
                }
            }
        }
        all_done
    }
}
