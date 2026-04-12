// Rings effect — characters on concentric spinning rings, disperse to positions

pub const NAME: &str = "rings";
pub const DESCRIPTION: &str = "Characters are dispersed and form into spinning rings.";
pub const EXTRA_EFFECT: bool = false;

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::Rng;

#[derive(Clone, Copy, PartialEq)]
enum RingPhase {
    Spinning,
    Dispersing,
    Done,
}

struct RingChar {
    final_y: usize,
    final_x: usize,
    cur_y: f64,
    cur_x: f64,
    original_ch: char,
    final_color: Rgb,
    ring_color: Rgb,
    ring_idx: usize,
    angle_offset: f64,
    phase: RingPhase,
    progress: f64,
    speed: f64,
}

struct Ring {
    radius: f64,
    center_y: f64,
    center_x: f64,
    speed: f64,
    angle: f64,
    clockwise: bool,
}

pub struct RingsEffect {
    chars: Vec<RingChar>,
    rings: Vec<Ring>,
    spin_frames: usize,
    disperse_frames: usize,
    frame: usize,
    cycle: usize,
    max_cycles: usize,
    dm: usize,
    width: usize,
    height: usize,
}

impl RingsEffect {
    pub fn new(grid: &Grid) -> Self {
        let (width, height, dm) = (grid.width, grid.height, 2usize);
        let final_gradient = Gradient::new(
            &[
                Rgb::from_hex("ab48ff"),
                Rgb::from_hex("e7b2b2"),
                Rgb::from_hex("fffebd"),
            ],
            12,
        );
        let ring_colors = [
            Rgb::from_hex("ab48ff"),
            Rgb::from_hex("e7b2b2"),
            Rgb::from_hex("fffebd"),
        ];
        let mut rng = rand::thread_rng();
        let center_y = height as f64 / 2.0;
        let center_x = width as f64 / 2.0;
        let ring_gap = (width.min(height) as f64 * 0.1).max(1.0);
        let max_radius = (width.max(height) as f64 / 2.0).max(1.0);
        let num_rings = (max_radius / ring_gap) as usize;

        let mut rings = Vec::new();
        for i in 0..num_rings.max(1) {
            let radius = (i + 1) as f64 * ring_gap;
            rings.push(Ring {
                radius,
                center_y,
                center_x,
                speed: rng.gen_range(0.25..1.0) * 0.02 / dm as f64,
                angle: 0.0,
                clockwise: i % 2 == 0,
            });
        }

        let total = width * height;
        let mut chars = Vec::with_capacity(total);
        let num_rings_actual = rings.len().max(1);

        for y in 0..height {
            for x in 0..width {
                let fc =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);
                let ring_idx = rng.gen_range(0..num_rings_actual);
                let rc = ring_colors[ring_idx % ring_colors.len()];
                let angle_offset = rng.gen_range(0.0..std::f64::consts::TAU);
                chars.push(RingChar {
                    final_y: y,
                    final_x: x,
                    cur_y: center_y,
                    cur_x: center_x,
                    original_ch: grid.cells[y][x].ch,
                    final_color: fc,
                    ring_color: rc,
                    ring_idx,
                    angle_offset,
                    phase: RingPhase::Spinning,
                    progress: 0.0,
                    speed: 0.0,
                });
            }
        }

        RingsEffect {
            chars,
            rings,
            spin_frames: 200 * dm,
            disperse_frames: 200 * dm,
            frame: 0,
            cycle: 0,
            max_cycles: 3,
            dm,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        self.frame += 1;
        let in_spin = self.frame % (self.spin_frames + self.disperse_frames) < self.spin_frames;
        let cycle_frame = self.frame % (self.spin_frames + self.disperse_frames);

        if in_spin {
            for ring in &mut self.rings {
                let dir = if ring.clockwise { 1.0 } else { -1.0 };
                ring.angle += ring.speed * dir;
            }
            for ch in &mut self.chars {
                ch.phase = RingPhase::Spinning;
                let ring = &self.rings[ch.ring_idx % self.rings.len()];
                let angle = ring.angle + ch.angle_offset;
                ch.cur_y = ring.center_y + angle.sin() * ring.radius;
                ch.cur_x = ring.center_x + angle.cos() * ring.radius;
            }
        } else {
            let t = (cycle_frame - self.spin_frames) as f64 / self.disperse_frames as f64;
            // Increment when the last frame of the disperse phase completes.
            // (t never reaches 1.0 via modulo, so we check the frame index instead.)
            if cycle_frame >= self.spin_frames + self.disperse_frames - 1 {
                self.cycle += 1;
            }
            let eased = easing::in_out_sine(t.min(1.0));
            for ch in &mut self.chars {
                ch.phase = RingPhase::Dispersing;
                let ring = &self.rings[ch.ring_idx % self.rings.len()];
                let angle = ring.angle + ch.angle_offset;
                let ring_y = ring.center_y + angle.sin() * ring.radius;
                let ring_x = ring.center_x + angle.cos() * ring.radius;
                ch.cur_y = ring_y + (ch.final_y as f64 - ring_y) * eased;
                ch.cur_x = ring_x + (ch.final_x as f64 - ring_x) * eased;
            }
        }

        let done = self.cycle >= self.max_cycles;

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
            cell.ch = ch.original_ch;
            match ch.phase {
                RingPhase::Spinning => cell.fg = Some(ch.ring_color.to_crossterm()),
                RingPhase::Dispersing => {
                    let t = (self.frame % (self.spin_frames + self.disperse_frames)
                        - self.spin_frames) as f64
                        / self.disperse_frames as f64;
                    cell.fg =
                        Some(Rgb::lerp(ch.ring_color, ch.final_color, t.min(1.0)).to_crossterm());
                }
                _ => cell.fg = Some(ch.final_color.to_crossterm()),
            }
        }

        if done {
            for ch in &self.chars {
                if ch.final_y < self.height && ch.final_x < self.width {
                    let cell = &mut grid.cells[ch.final_y][ch.final_x];
                    cell.visible = true;
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.final_color.to_crossterm());
                }
            }
        }
        done
    }
}
