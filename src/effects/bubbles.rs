// Bubbles effect — faithful TTE reimplementation
// Characters grouped into bubbles, float upward, pop, and settle

pub const NAME: &str = "bubbles";
pub const DESCRIPTION: &str = "Characters are formed into bubbles that float down and pop.";
pub const EXTRA_EFFECT: bool = false;

use crate::easing;
use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::Rng;

const BUBBLE_COLORS: [Rgb; 4] = [
    Rgb {
        r: 0xd3,
        g: 0x3a,
        b: 0xff,
    },
    Rgb {
        r: 0x73,
        g: 0x95,
        b: 0xc4,
    },
    Rgb {
        r: 0x43,
        g: 0xc2,
        b: 0xa7,
    },
    Rgb {
        r: 0x02,
        g: 0xff,
        b: 0x7f,
    },
];

#[derive(Clone, Copy, PartialEq)]
enum BubbleCharPhase {
    Floating,
    Pop1,
    Pop2,
    Settling,
    Done,
}

struct BubbleChar {
    final_y: usize,
    final_x: usize,
    cur_y: f64,
    cur_x: f64,
    original_ch: char,
    final_color: Rgb,
    bubble_color: Rgb,
    phase: BubbleCharPhase,
    frame_count: usize,
    settle_progress: f64,
    settle_speed: f64,
    pop_y: f64,
    pop_x: f64,
}

struct Bubble {
    char_indices: Vec<usize>,
    anchor_y: f64,
    anchor_x: f64,
    target_y: f64,
    radius: f64,
    angle: f64,
    speed: f64,
    progress: f64,
    active: bool,
    popped: bool,
}

pub struct BubblesEffect {
    chars: Vec<BubbleChar>,
    bubbles: Vec<Bubble>,
    bubble_delay: usize,
    delay_counter: usize,
    activated_up_to: usize,
    dm: usize,
    width: usize,
    height: usize,
}

impl BubblesEffect {
    pub fn new(grid: &Grid) -> Self {
        let width = grid.width;
        let height = grid.height;
        let dm: usize = 2;

        let final_gradient = Gradient::new(&[Rgb::from_hex("d33aff"), Rgb::from_hex("02ff7f")], 12);

        let mut rng = rand::thread_rng();
        let total = width * height;

        let mut chars: Vec<BubbleChar> = Vec::with_capacity(total);
        for y in 0..height {
            for x in 0..width {
                let final_color =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Diagonal);
                chars.push(BubbleChar {
                    final_y: y,
                    final_x: x,
                    cur_y: 0.0,
                    cur_x: 0.0,
                    original_ch: grid.cells[y][x].ch,
                    final_color,
                    bubble_color: BUBBLE_COLORS[rng.gen_range(0..BUBBLE_COLORS.len())],
                    phase: BubbleCharPhase::Floating,
                    frame_count: 0,
                    settle_progress: 0.0,
                    settle_speed: 0.3 / dm as f64,
                    pop_y: 0.0,
                    pop_x: 0.0,
                });
            }
        }

        // Group into bubbles (5-20 chars each)
        let mut indices: Vec<usize> = (0..total).collect();
        use rand::seq::SliceRandom;
        indices.shuffle(&mut rng);

        let mut bubbles = Vec::new();
        let mut pos = 0;
        while pos < indices.len() {
            let size = rng.gen_range(5..=20).min(indices.len() - pos);
            let char_indices: Vec<usize> = indices[pos..pos + size].to_vec();
            let radius = (size / 5).max(1) as f64;

            let anchor_x = rng.gen_range(0..width) as f64;
            let anchor_y = height as f64 + radius + 2.0;
            let target_y = rng.gen_range(1..height.max(2)) as f64;

            // Position chars on circle around anchor
            for (i, &ci) in char_indices.iter().enumerate() {
                let angle = (i as f64 / size as f64) * std::f64::consts::TAU;
                chars[ci].cur_y = anchor_y + angle.sin() * radius;
                chars[ci].cur_x = anchor_x + angle.cos() * radius;
            }

            let dist = (anchor_y - target_y).abs().max(1.0);
            let speed = (0.5 / dist) / dm as f64;

            bubbles.push(Bubble {
                char_indices,
                anchor_y,
                anchor_x,
                target_y,
                radius,
                angle: 0.0,
                speed,
                progress: 0.0,
                active: false,
                popped: false,
            });

            pos += size;
        }

        BubblesEffect {
            chars,
            bubbles,
            bubble_delay: 20 * dm,
            delay_counter: 0,
            activated_up_to: 0,
            dm,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        // Activate bubbles with delay
        if self.activated_up_to < self.bubbles.len() {
            if self.delay_counter == 0 {
                self.bubbles[self.activated_up_to].active = true;
                self.activated_up_to += 1;
                self.delay_counter = self.bubble_delay;
            } else {
                self.delay_counter -= 1;
            }
        }

        let dm = self.dm;

        // Tick bubbles
        for bubble in &mut self.bubbles {
            if !bubble.active || bubble.popped {
                continue;
            }

            bubble.progress += bubble.speed;
            bubble.angle += 0.02;

            if bubble.progress >= 1.0 {
                bubble.progress = 1.0;
                bubble.popped = true;
                // Start pop phase for all chars
                for &ci in &bubble.char_indices {
                    self.chars[ci].phase = BubbleCharPhase::Pop1;
                    self.chars[ci].frame_count = 0;
                    self.chars[ci].pop_y = self.chars[ci].cur_y;
                    self.chars[ci].pop_x = self.chars[ci].cur_x;
                }
                continue;
            }

            let eased = easing::in_out_sine(bubble.progress);
            let new_y = bubble.anchor_y + (bubble.target_y - bubble.anchor_y) * eased;
            let dy = new_y
                - (bubble.anchor_y
                    + (bubble.target_y - bubble.anchor_y)
                        * easing::in_out_sine((bubble.progress - bubble.speed).max(0.0)));

            // Update char positions (orbit around anchor)
            for (i, &ci) in bubble.char_indices.iter().enumerate() {
                let base_angle =
                    (i as f64 / bubble.char_indices.len() as f64) * std::f64::consts::TAU;
                let angle = base_angle + bubble.angle;
                self.chars[ci].cur_y += dy;
                self.chars[ci].cur_x = bubble.anchor_x + angle.cos() * bubble.radius;
            }
        }

        // Tick individual chars (pop/settle phases)
        let mut all_done = self.activated_up_to >= self.bubbles.len();
        for ch in &mut self.chars {
            match ch.phase {
                BubbleCharPhase::Floating => {
                    all_done = false;
                }
                BubbleCharPhase::Pop1 => {
                    ch.frame_count += 1;
                    if ch.frame_count >= 9 * dm {
                        ch.phase = BubbleCharPhase::Pop2;
                        ch.frame_count = 0;
                    }
                    all_done = false;
                }
                BubbleCharPhase::Pop2 => {
                    ch.frame_count += 1;
                    if ch.frame_count >= 9 * dm {
                        ch.phase = BubbleCharPhase::Settling;
                        ch.settle_progress = 0.0;
                    }
                    all_done = false;
                }
                BubbleCharPhase::Settling => {
                    ch.settle_progress += ch.settle_speed;
                    if ch.settle_progress >= 1.0 {
                        ch.settle_progress = 1.0;
                        ch.phase = BubbleCharPhase::Done;
                    }
                    let eased = easing::in_out_expo(ch.settle_progress);
                    ch.cur_y = ch.pop_y + (ch.final_y as f64 - ch.pop_y) * eased;
                    ch.cur_x = ch.pop_x + (ch.final_x as f64 - ch.pop_x) * eased;
                    if ch.phase != BubbleCharPhase::Done {
                        all_done = false;
                    }
                }
                BubbleCharPhase::Done => {}
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
                BubbleCharPhase::Floating => {
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.bubble_color.to_crossterm());
                }
                BubbleCharPhase::Pop1 => {
                    cell.ch = '*';
                    cell.fg = Some(Rgb::new(255, 255, 255).to_crossterm());
                }
                BubbleCharPhase::Pop2 => {
                    cell.ch = '\'';
                    cell.fg = Some(Rgb::new(255, 255, 255).to_crossterm());
                }
                BubbleCharPhase::Settling => {
                    cell.ch = ch.original_ch;
                    let t = ch.settle_progress;
                    cell.fg =
                        Some(Rgb::lerp(Rgb::new(255, 255, 255), ch.final_color, t).to_crossterm());
                }
                BubbleCharPhase::Done => {
                    cell.ch = ch.original_ch;
                    cell.fg = Some(ch.final_color.to_crossterm());
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
