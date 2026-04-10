// BinaryPath effect — binary digits travel right-angle paths to final positions
use crate::engine::Grid;
use crate::gradient::{Gradient, Rgb, GradientDirection};
use rand::Rng;
use rand::seq::SliceRandom;

struct BinaryChar {
    final_y: usize, final_x: usize,
    path: Vec<(f64, f64)>,
    path_idx: usize,
    progress: f64,
    speed: f64,
    original_ch: char,
    final_color: Rgb,
    binary_color: Rgb,
    active: bool,
    arrived: bool,
    brighten_step: usize,
    binary_symbol: char,
}

pub struct BinaryPathEffect {
    chars: Vec<BinaryChar>,
    dm: usize,
    width: usize, height: usize,
    frame: usize,
    max_active: usize,
    pending: Vec<usize>,
    binary_colors: Vec<Rgb>,
    final_gradient: Gradient,
}

impl BinaryPathEffect {
    pub fn new(grid: &Grid) -> Self {
        let (width, height, dm) = (grid.width, grid.height, 2usize);
        let final_gradient = Gradient::new(
            &[Rgb::from_hex("00d500"), Rgb::from_hex("007500")], 12,
        );
        let binary_colors = vec![
            Rgb::from_hex("044E29"), Rgb::from_hex("157e38"),
            Rgb::from_hex("45bf55"), Rgb::from_hex("95ed87"),
        ];

        let mut rng = rand::thread_rng();
        let mut chars = Vec::new();

        for y in 0..height { for x in 0..width {
            let fc = final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);
            let bc = binary_colors[rng.gen_range(0..binary_colors.len())];

            // Generate right-angle path from outside to (y, x)
            let mut path = Vec::new();
            // Start outside
            let side = rng.gen_range(0..4);
            let (mut cy, mut cx): (f64, f64) = match side {
                0 => (rng.gen_range(0..height) as f64, -(rng.gen_range(3..15) as f64)),
                1 => (rng.gen_range(0..height) as f64, (width + rng.gen_range(3..15)) as f64),
                2 => (-(rng.gen_range(3..15) as f64), rng.gen_range(0..width) as f64),
                _ => ((height + rng.gen_range(3..15)) as f64, rng.gen_range(0..width) as f64),
            };
            path.push((cy, cx));

            // Alternate horizontal/vertical segments toward target
            let target_y = y as f64;
            let target_x = x as f64;
            let mut horizontal = rng.gen_bool(0.5);
            for _ in 0..20 {
                if (cy - target_y).abs() < 0.5 && (cx - target_x).abs() < 0.5 { break; }
                if horizontal {
                    let remaining = target_x - cx;
                    let step = remaining.signum() * remaining.abs().min(10.0);
                    cx += step;
                } else {
                    let remaining = target_y - cy;
                    let step = remaining.signum() * remaining.abs().min(4.0);
                    cy += step;
                }
                path.push((cy, cx));
                horizontal = !horizontal;
            }
            path.push((target_y, target_x));

            chars.push(BinaryChar {
                final_y: y, final_x: x,
                path, path_idx: 0, progress: 0.0,
                speed: 1.0 / dm as f64,
                original_ch: grid.cells[y][x].ch,
                final_color: fc, binary_color: bc,
                active: false, arrived: false, brighten_step: 0,
                binary_symbol: if rng.gen_bool(0.5) { '0' } else { '1' },
            });
        }}

        let total = chars.len();
        let max_active = (total as f64 * 0.08).max(1.0) as usize;
        let mut pending: Vec<usize> = (0..total).collect();
        pending.shuffle(&mut rng);

        BinaryPathEffect { chars, dm, width, height, frame: 0, max_active, pending, binary_colors, final_gradient }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        self.frame += 1;
        let dm = self.dm;

        // Activate new chars
        let active_count = self.chars.iter().filter(|c| c.active && !c.arrived).count();
        let to_activate = self.max_active.saturating_sub(active_count);
        for _ in 0..to_activate {
            if let Some(idx) = self.pending.pop() {
                self.chars[idx].active = true;
            }
        }

        // Move active chars along paths
        for ch in &mut self.chars {
            if !ch.active || ch.arrived { continue; }
            if ch.path_idx + 1 >= ch.path.len() {
                ch.arrived = true;
                continue;
            }
            ch.progress += ch.speed;
            while ch.progress >= 1.0 && ch.path_idx + 1 < ch.path.len() {
                ch.progress -= 1.0;
                ch.path_idx += 1;
            }
            if ch.path_idx + 1 >= ch.path.len() {
                ch.arrived = true;
            }
        }

        // Brighten arrived chars
        for ch in &mut self.chars {
            if ch.arrived && ch.brighten_step < 10 {
                ch.brighten_step += 1;
            }
        }

        // Check done
        if self.pending.is_empty() && self.chars.iter().all(|c| c.arrived && c.brighten_step >= 10) {
            // Set final state
            for row in &mut grid.cells { for cell in row { cell.visible = true; } }
            for ch in &self.chars {
                let cell = &mut grid.cells[ch.final_y][ch.final_x];
                cell.ch = ch.original_ch;
                cell.fg = Some(ch.final_color.to_crossterm());
            }
            return true;
        }

        // Render
        for row in &mut grid.cells { for cell in row { cell.visible = false; } }
        for ch in &self.chars {
            if !ch.active { continue; }
            if ch.arrived {
                let cell = &mut grid.cells[ch.final_y][ch.final_x];
                cell.visible = true;
                cell.ch = ch.original_ch;
                let t = ch.brighten_step as f64 / 10.0;
                let white50 = Rgb { r: 128, g: 128, b: 128 };
                cell.fg = Some(Rgb::lerp(white50, ch.final_color, t).to_crossterm());
            } else {
                // Show at current path position
                let idx = ch.path_idx;
                let (y0, x0) = ch.path[idx];
                let (y1, x1) = if idx + 1 < ch.path.len() { ch.path[idx + 1] } else { ch.path[idx] };
                let t = ch.progress.min(1.0);
                let cy = y0 + (y1 - y0) * t;
                let cx = x0 + (x1 - x0) * t;
                let ry = cy.round() as isize;
                let rx = cx.round() as isize;
                if ry >= 0 && rx >= 0 && (ry as usize) < self.height && (rx as usize) < self.width {
                    let cell = &mut grid.cells[ry as usize][rx as usize];
                    cell.visible = true;
                    cell.ch = ch.binary_symbol;
                    cell.fg = Some(ch.binary_color.to_crossterm());
                }
            }
        }
        false
    }
}
