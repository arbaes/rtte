// Matrix effect — digital rain columns that resolve to text

pub const NAME: &str = "matrix";
pub const DESCRIPTION: &str = "Matrix digital rain effect.";
pub const EXTRA_EFFECT: bool = false;

use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Rain,
    Fill,
    Resolve,
    Done,
}

struct RainColumn {
    x: usize,
    head: isize,
    length: usize,
    speed_counter: usize,
    speed: usize,
    active: bool,
    full: bool,
    hold: usize,
}

struct MatrixChar {
    final_ch: char,
    resolved: bool,
    resolve_step: usize,
}

pub struct MatrixEffect {
    columns: Vec<RainColumn>,
    chars: Vec<Vec<MatrixChar>>,
    phase: Phase,
    frame: usize,
    dm: usize,
    width: usize,
    height: usize,
    rain_time: usize,
    column_delay: usize,
    resolve_delay_counter: usize,
    pending_resolve: Vec<(usize, usize)>,
    rain_gradient: Vec<Rgb>,
    highlight_color: Rgb,
    final_gradient: Gradient,
    rain_symbols: Vec<char>,
}

impl MatrixEffect {
    pub fn new(grid: &Grid) -> Self {
        let (width, height, dm) = (grid.width, grid.height, 2usize);
        let final_gradient = Gradient::new(&[Rgb::from_hex("92be92"), Rgb::from_hex("336b33")], 12);
        let highlight_color = Rgb::from_hex("dbffdb");

        // Rain color gradient: bright to dark green
        let rain_grad: Vec<Rgb> = (0..12)
            .map(|i| {
                let t = i as f64 / 11.0;
                Rgb::lerp(Rgb::from_hex("92be92"), Rgb::from_hex("185318"), t)
            })
            .collect();

        let rain_symbols: Vec<char> = "ﾊﾐﾋｰｳｼﾅﾓﾆｻﾜﾂｵﾘｱﾎﾃﾏｹﾒｴｶｷﾑﾕﾗｾﾈｽﾀﾇﾍ012345789:.<>*+=-"
            .chars()
            .collect();

        let mut chars = Vec::new();
        for y in 0..height {
            let mut row = Vec::new();
            for x in 0..width {
                row.push(MatrixChar {
                    final_ch: grid.cells[y][x].ch,
                    resolved: false,
                    resolve_step: 0,
                });
            }
            chars.push(row);
        }

        let mut columns: Vec<RainColumn> = (0..width)
            .map(|x| RainColumn {
                x,
                head: -(rand::thread_rng().gen_range(0..height.max(1)) as isize),
                length: rand::thread_rng().gen_range(1..height.max(2)),
                speed: rand::thread_rng().gen_range(2..=15) * dm,
                speed_counter: 0,
                active: false,
                full: false,
                hold: 0,
            })
            .collect();

        // Shuffle activation order
        let mut rng = rand::thread_rng();
        let mut order: Vec<usize> = (0..width).collect();
        order.shuffle(&mut rng);

        // Stagger activation
        for (i, &idx) in order.iter().enumerate() {
            columns[idx].speed_counter = i * rng.gen_range(3..=9) * dm;
        }

        let mut pending: Vec<(usize, usize)> = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pending.push((y, x));
            }
        }
        pending.shuffle(&mut rng);

        MatrixEffect {
            columns,
            chars,
            phase: Phase::Rain,
            frame: 0,
            dm,
            width,
            height,
            rain_time: 15 * 60 * dm, // ~15 seconds at 60fps * dm
            column_delay: 0,
            resolve_delay_counter: 0,
            pending_resolve: pending,
            rain_gradient: rain_grad,
            highlight_color,
            final_gradient,
            rain_symbols,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        self.frame += 1;
        let dm = self.dm;
        let mut rng = rand::thread_rng();

        // Activate columns gradually
        for col in &mut self.columns {
            if !col.active {
                if col.speed_counter == 0 {
                    col.active = true;
                } else {
                    col.speed_counter -= 1;
                }
            }
        }

        match self.phase {
            Phase::Rain | Phase::Fill => {
                let fill = self.phase == Phase::Fill;
                for col in &mut self.columns {
                    if !col.active {
                        continue;
                    }
                    col.speed_counter += 1;
                    let spd = if fill {
                        (col.speed / 3).max(1)
                    } else {
                        col.speed
                    };
                    if col.speed_counter >= spd {
                        col.speed_counter = 0;
                        col.head += 1;
                        if col.head >= self.height as isize {
                            if fill {
                                col.full = true;
                            } else {
                                col.head = -(rng.gen_range(0..self.height / 2 + 1) as isize);
                                let lo = self.height / 10 + 1;
                                let hi = self.height.max(lo + 1);
                                col.length = rng.gen_range(lo..hi);
                            }
                        }
                    }
                }
                if self.phase == Phase::Rain && self.frame >= self.rain_time {
                    self.phase = Phase::Fill;
                    // Extend all columns to fill
                    for col in &mut self.columns {
                        col.length = self.height + 5;
                        col.active = true;
                    }
                }
                if fill && self.columns.iter().all(|c| c.full) {
                    self.phase = Phase::Resolve;
                }
            }
            Phase::Resolve => {
                let resolve_per_frame = (3 * dm).max(1);
                for _ in 0..resolve_per_frame {
                    if let Some((y, x)) = self.pending_resolve.pop() {
                        self.chars[y][x].resolved = true;
                    }
                }
                // Advance resolve animation
                for row in &mut self.chars {
                    for ch in row {
                        if ch.resolved && ch.resolve_step < 8 {
                            ch.resolve_step += 1;
                        }
                    }
                }
                if self.pending_resolve.is_empty()
                    && self
                        .chars
                        .iter()
                        .all(|row| row.iter().all(|c| c.resolve_step >= 8))
                {
                    self.phase = Phase::Done;
                }
            }
            Phase::Done => return true,
        }

        // Render
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = &mut grid.cells[y][x];
                if self.phase == Phase::Resolve || self.phase == Phase::Done {
                    let mc = &self.chars[y][x];
                    if mc.resolved {
                        cell.visible = true;
                        cell.ch = mc.final_ch;
                        let t = mc.resolve_step as f64 / 8.0;
                        let fc = self.final_gradient.color_at_coord(
                            y,
                            x,
                            self.height,
                            self.width,
                            GradientDirection::Vertical,
                        );
                        cell.fg = Some(Rgb::lerp(self.highlight_color, fc, t).to_crossterm());
                    } else {
                        // Still show rain
                        cell.visible = true;
                        cell.ch = *self.rain_symbols.choose(&mut rng).unwrap_or(&'0');
                        let depth = (y as f64 / self.height.max(1) as f64 * 11.0) as usize;
                        cell.fg = Some(self.rain_gradient[depth.min(11)].to_crossterm());
                    }
                } else {
                    cell.visible = false;
                    let col = &self.columns[x];
                    if !col.active {
                        continue;
                    }
                    let tail = col.head - col.length as isize;
                    if (y as isize) <= col.head && (y as isize) > tail {
                        cell.visible = true;
                        if y as isize == col.head {
                            cell.ch = *self.rain_symbols.choose(&mut rng).unwrap_or(&'0');
                            cell.fg = Some(self.highlight_color.to_crossterm());
                        } else {
                            // Symbol swap chance
                            if rng.gen::<f64>() < 0.005 {
                                cell.ch = *self.rain_symbols.choose(&mut rng).unwrap_or(&'0');
                            }
                            let dist_from_head = (col.head - y as isize) as f64 / col.length as f64;
                            let idx = (dist_from_head * 11.0) as usize;
                            cell.fg = Some(self.rain_gradient[idx.min(11)].to_crossterm());
                        }
                    }
                }
            }
        }
        false
    }
}
