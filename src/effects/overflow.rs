// Overflow effect — text scrolls/overflows then settles into correct order

pub const NAME: &str = "overflow";
pub const DESCRIPTION: &str = "Input text overflows and scrolls the terminal in a random order until eventually appearing ordered.";
pub const EXTRA_EFFECT: bool = false;

use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Overflow,
    Settle,
    Done,
}

struct OverflowRow {
    chars: Vec<(char, Rgb)>,
    y_offset: f64,
}

pub struct OverflowEffect {
    original: Vec<Vec<char>>,
    rows: Vec<OverflowRow>,
    phase: Phase,
    frame: usize,
    dm: usize,
    width: usize,
    height: usize,
    scroll_pos: f64,
    total_overflow_rows: usize,
    final_gradient: Gradient,
    overflow_gradient: Gradient,
    settled_count: usize,
}

impl OverflowEffect {
    pub fn new(grid: &Grid) -> Self {
        let (width, height, dm) = (grid.width, grid.height, 2usize);
        let final_gradient = Gradient::new(
            &[
                Rgb::from_hex("8A008A"),
                Rgb::from_hex("00D1FF"),
                Rgb::from_hex("FFFFFF"),
            ],
            12,
        );
        let overflow_gradient = Gradient::new(
            &[
                Rgb::from_hex("f2ebc0"),
                Rgb::from_hex("8dbfb3"),
                Rgb::from_hex("f2ebc0"),
            ],
            12,
        );

        let mut rng = rand::thread_rng();
        let mut original: Vec<Vec<char>> = Vec::new();
        for y in 0..height {
            let row: Vec<char> = (0..width).map(|x| grid.cells[y][x].ch).collect();
            original.push(row);
        }

        // Generate overflow rows: cycles of shuffled rows, then final correct rows
        let cycles = rng.gen_range(2..=4);
        let mut rows: Vec<OverflowRow> = Vec::new();

        for _cycle in 0..cycles {
            let mut shuffled_indices: Vec<usize> = (0..height).collect();
            shuffled_indices.shuffle(&mut rng);
            for &idx in &shuffled_indices {
                let row_chars: Vec<(char, Rgb)> = (0..width)
                    .map(|x| {
                        let color = overflow_gradient.color_at_coord(
                            rows.len(),
                            0,
                            height * (cycles + 1),
                            1,
                            GradientDirection::Vertical,
                        );
                        (original[idx][x], color)
                    })
                    .collect();
                rows.push(OverflowRow {
                    chars: row_chars,
                    y_offset: (height + rows.len()) as f64,
                });
            }
        }

        // Add final correct rows
        for (y, orig_row) in original.iter().enumerate() {
            let row_chars: Vec<(char, Rgb)> = (0..width)
                .map(|x| {
                    let color = final_gradient.color_at_coord(
                        y,
                        x,
                        height,
                        width,
                        GradientDirection::Vertical,
                    );
                    (orig_row[x], color)
                })
                .collect();
            rows.push(OverflowRow {
                chars: row_chars,
                y_offset: (height + rows.len()) as f64,
            });
        }

        let total = rows.len();

        OverflowEffect {
            original,
            rows,
            phase: Phase::Overflow,
            frame: 0,
            dm,
            width,
            height,
            scroll_pos: 0.0,
            total_overflow_rows: total,
            final_gradient,
            overflow_gradient,
            settled_count: 0,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        self.frame += 1;
        let dm = self.dm;

        match self.phase {
            Phase::Overflow => {
                let speed = 3.0 / dm as f64;
                self.scroll_pos += speed;

                // Check if all final rows are in view
                let last_row_offset = self.rows.last().map(|r| r.y_offset).unwrap_or(0.0);
                let visible_top = self.scroll_pos;
                if visible_top + self.height as f64 >= last_row_offset + self.height as f64 {
                    self.phase = Phase::Settle;
                }
            }
            Phase::Settle => {
                // Snap rows to final positions
                let target_start = self.total_overflow_rows - self.height;
                let mut all_settled = true;
                for (i, row) in self.rows.iter_mut().enumerate() {
                    if i >= target_start {
                        let target_y = (i - target_start) as f64;
                        let diff = target_y - (row.y_offset - self.scroll_pos);
                        if diff.abs() > 0.01 {
                            row.y_offset += diff * 0.3;
                            all_settled = false;
                        }
                    }
                }
                if all_settled {
                    self.phase = Phase::Done;
                }
            }
            Phase::Done => return true,
        }

        // Render
        for row in &mut grid.cells {
            for cell in row {
                cell.visible = false;
            }
        }
        for overflow_row in &self.rows {
            let screen_y = overflow_row.y_offset - self.scroll_pos;
            let ry = screen_y.round() as isize;
            if ry < 0 || ry >= self.height as isize {
                continue;
            }
            let ry = ry as usize;
            for (x, &(ch, color)) in overflow_row.chars.iter().enumerate() {
                if x >= self.width {
                    break;
                }
                let cell = &mut grid.cells[ry][x];
                cell.visible = true;
                cell.ch = ch;
                cell.fg = Some(color.to_crossterm());
            }
        }
        false
    }
}
