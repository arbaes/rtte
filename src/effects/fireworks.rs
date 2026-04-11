// Fireworks effect — faithful TTE reimplementation
// Shells launch from bottom, explode outward, fall to positions

pub const NAME: &str = "fireworks";
pub const DESCRIPTION: &str = "Characters launch and explode like fireworks and fall into place.";

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::Rng;

const SHELL_COLORS: [Rgb; 5] = [
    Rgb {
        r: 0x88,
        g: 0xF7,
        b: 0xE2,
    },
    Rgb {
        r: 0x44,
        g: 0xD4,
        b: 0x92,
    },
    Rgb {
        r: 0xF5,
        g: 0xEB,
        b: 0x67,
    },
    Rgb {
        r: 0xFF,
        g: 0xA1,
        b: 0x5C,
    },
    Rgb {
        r: 0xFA,
        g: 0x23,
        b: 0x3E,
    },
];

#[derive(Clone, Copy, PartialEq)]
enum FWPhase {
    Waiting,
    Launch,
    Explode,
    Fall,
    Done,
}

struct FWChar {
    final_y: usize,
    final_x: usize,
    cur_y: f64,
    cur_x: f64,
    original_ch: char,
    final_color: Rgb,
    shell_color: Rgb,
    // Launch
    launch_start_y: f64,
    launch_start_x: f64,
    origin_y: f64,
    origin_x: f64,
    // Explode
    explode_target_y: f64,
    explode_target_x: f64,
    // Motion
    phase: FWPhase,
    progress: f64,
    speed: f64,
}

struct Shell {
    char_indices: Vec<usize>,
    origin_y: f64,
    origin_x: f64,
    shell_color: Rgb,
    active: bool,
    launched: bool,
}

pub struct FireworksEffect {
    chars: Vec<FWChar>,
    shells: Vec<Shell>,
    launch_delay: usize,
    delay_counter: usize,
    activated_up_to: usize,
    dm: usize,
    width: usize,
    height: usize,
}

impl FireworksEffect {
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

        let mut rng = rand::thread_rng();
        let total = width * height;
        let volume = 0.05; // 5% per shell

        let mut chars: Vec<FWChar> = Vec::with_capacity(total);
        for y in 0..height {
            for x in 0..width {
                let final_color = final_gradient.color_at_coord(
                    y,
                    x,
                    height,
                    width,
                    GradientDirection::Horizontal,
                );
                chars.push(FWChar {
                    final_y: y,
                    final_x: x,
                    cur_y: height as f64 + 1.0,
                    cur_x: x as f64,
                    original_ch: grid.cells[y][x].ch,
                    final_color,
                    shell_color: Rgb::new(255, 255, 255),
                    launch_start_y: height as f64 + 1.0,
                    launch_start_x: 0.0,
                    origin_y: 0.0,
                    origin_x: 0.0,
                    explode_target_y: 0.0,
                    explode_target_x: 0.0,
                    phase: FWPhase::Waiting,
                    progress: 0.0,
                    speed: 0.0,
                });
            }
        }

        // Group into shells
        let mut indices: Vec<usize> = (0..total).collect();
        use rand::seq::SliceRandom;
        indices.shuffle(&mut rng);

        let shell_size = ((total as f64 * volume) as usize).max(1);
        let mut shells = Vec::new();
        let mut pos = 0;

        while pos < indices.len() {
            let size = shell_size.min(indices.len() - pos);
            let char_indices: Vec<usize> = indices[pos..pos + size].to_vec();
            let shell_color = SHELL_COLORS[rng.gen_range(0..SHELL_COLORS.len())];

            // Random origin (above text area)
            let origin_x = rng.gen_range(0..width.max(1)) as f64;
            let origin_y = rng.gen_range(0..height.max(2).saturating_sub(1)) as f64;

            let explode_dist = (width as f64 * 0.2).clamp(2.0, 15.0);

            for &ci in &char_indices {
                let ch = &mut chars[ci];
                ch.shell_color = shell_color;
                ch.launch_start_x = rng.gen_range(0..width.max(1)) as f64;
                ch.launch_start_y = height as f64 + 1.0;
                ch.origin_y = origin_y;
                ch.origin_x = origin_x;
                // Random explode direction
                let angle = rng.gen_range(0.0..std::f64::consts::TAU);
                let dist = rng.gen_range(1.0..explode_dist);
                ch.explode_target_y = origin_y + angle.sin() * dist;
                ch.explode_target_x = origin_x + angle.cos() * dist;
                ch.cur_y = ch.launch_start_y;
                ch.cur_x = ch.launch_start_x;
            }

            shells.push(Shell {
                char_indices,
                origin_y,
                origin_x,
                shell_color,
                active: false,
                launched: false,
            });
            pos += size;
        }

        let base_delay = 45;
        let launch_delay = (base_delay as f64 * rng.gen_range(0.5..1.5)) as usize * dm;

        FireworksEffect {
            chars,
            shells,
            launch_delay,
            delay_counter: 0,
            activated_up_to: 0,
            dm,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        let dm = self.dm;

        // Activate shells with delay
        if self.activated_up_to < self.shells.len() {
            if self.delay_counter == 0 {
                let shell = &mut self.shells[self.activated_up_to];
                shell.active = true;
                for &ci in &shell.char_indices {
                    self.chars[ci].phase = FWPhase::Launch;
                    self.chars[ci].progress = 0.0;
                    self.chars[ci].speed = (0.35
                        / ((self.chars[ci].launch_start_y - self.chars[ci].origin_y)
                            .abs()
                            .max(1.0)))
                        / dm as f64;
                }
                self.activated_up_to += 1;
                let mut rng = rand::thread_rng();
                self.delay_counter = (45.0 * rng.gen_range(0.5..1.5)) as usize * dm;
            } else {
                self.delay_counter -= 1;
            }
        }

        // Tick chars
        let mut all_done = self.activated_up_to >= self.shells.len();
        let mut rng = rand::thread_rng();

        for ch in &mut self.chars {
            match ch.phase {
                FWPhase::Waiting => {
                    all_done = false;
                }
                FWPhase::Launch => {
                    ch.progress += ch.speed;
                    if ch.progress >= 1.0 {
                        ch.progress = 0.0;
                        ch.phase = FWPhase::Explode;
                        ch.cur_y = ch.origin_y;
                        ch.cur_x = ch.origin_x;
                        let dist = ((ch.explode_target_y - ch.origin_y).powi(2)
                            + (ch.explode_target_x - ch.origin_x).powi(2))
                        .sqrt()
                        .max(1.0);
                        ch.speed = (rng.gen_range(0.2..0.4) / dist) / dm as f64;
                    } else {
                        let eased = easing::out_expo(ch.progress);
                        ch.cur_y = ch.launch_start_y + (ch.origin_y - ch.launch_start_y) * eased;
                        ch.cur_x = ch.launch_start_x + (ch.origin_x - ch.launch_start_x) * eased;
                    }
                    all_done = false;
                }
                FWPhase::Explode => {
                    ch.progress += ch.speed;
                    if ch.progress >= 1.0 {
                        ch.progress = 0.0;
                        ch.phase = FWPhase::Fall;
                        let dist = ((ch.final_y as f64 - ch.explode_target_y).powi(2)
                            + (ch.final_x as f64 - ch.explode_target_x).powi(2))
                        .sqrt()
                        .max(1.0);
                        ch.speed = (0.6 / dist) / dm as f64;
                        ch.cur_y = ch.explode_target_y;
                        ch.cur_x = ch.explode_target_x;
                    } else {
                        let eased = easing::out_circ(ch.progress);
                        ch.cur_y = ch.origin_y + (ch.explode_target_y - ch.origin_y) * eased;
                        ch.cur_x = ch.origin_x + (ch.explode_target_x - ch.origin_x) * eased;
                    }
                    all_done = false;
                }
                FWPhase::Fall => {
                    ch.progress += ch.speed;
                    if ch.progress >= 1.0 {
                        ch.progress = 1.0;
                        ch.phase = FWPhase::Done;
                        ch.cur_y = ch.final_y as f64;
                        ch.cur_x = ch.final_x as f64;
                    } else {
                        let eased = easing::in_out_quart(ch.progress);
                        ch.cur_y =
                            ch.explode_target_y + (ch.final_y as f64 - ch.explode_target_y) * eased;
                        ch.cur_x =
                            ch.explode_target_x + (ch.final_x as f64 - ch.explode_target_x) * eased;
                    }
                    if ch.phase != FWPhase::Done {
                        all_done = false;
                    }
                }
                FWPhase::Done => {}
            }
        }

        // Render
        for row in &mut grid.cells {
            for cell in row {
                cell.visible = false;
            }
        }

        for ch in &self.chars {
            if ch.phase == FWPhase::Waiting {
                continue;
            }
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
                FWPhase::Launch => {
                    cell.ch = 'o';
                    cell.fg = Some(ch.shell_color.to_crossterm());
                }
                FWPhase::Explode => {
                    cell.ch = ch.original_ch;
                    let t = ch.progress;
                    let bright =
                        Rgb::lerp(ch.shell_color, Rgb::new(255, 255, 255), (t * 2.0).min(1.0));
                    let color = if t < 0.5 {
                        bright
                    } else {
                        Rgb::lerp(Rgb::new(255, 255, 255), ch.shell_color, (t - 0.5) * 2.0)
                    };
                    cell.fg = Some(color.to_crossterm());
                }
                FWPhase::Fall => {
                    cell.ch = ch.original_ch;
                    let t = ch.progress;
                    cell.fg = Some(Rgb::lerp(ch.shell_color, ch.final_color, t).to_crossterm());
                }
                FWPhase::Done => {
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.final_color.to_crossterm());
                }
                _ => {}
            }
        }

        all_done
    }
}
