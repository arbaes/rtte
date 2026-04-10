// Highlight effect — faithful TTE reimplementation
//
// A specular highlight beam sweeps diagonally (bottom-left → top-right)
// across text. Each character brightens then dims as the beam passes.
// Characters are visible at their final gradient color throughout.

use crate::engine::Grid;
use crate::easing;
use crate::gradient::{Gradient, Rgb, GradientDirection};

struct CharHighlight {
    y: usize,
    x: usize,
    base_color: Rgb,
    // Highlight gradient: base → bright → bright → base
    // 3 ramp-up + width peak + 3 ramp-down frames
    highlight_colors: Vec<Rgb>,
    frame_idx: usize,
    frames_per_tick: usize,
    tick_count: usize,
    active: bool,
    done: bool,
}

impl CharHighlight {
    fn tick(&mut self) {
        if !self.active || self.done {
            return;
        }
        self.tick_count += 1;
        if self.tick_count >= self.frames_per_tick {
            self.tick_count = 0;
            self.frame_idx += 1;
            if self.frame_idx >= self.highlight_colors.len() {
                self.frame_idx = self.highlight_colors.len() - 1;
                self.done = true;
            }
        }
    }

    fn current_color(&self) -> Rgb {
        if !self.active || self.done {
            return self.base_color;
        }
        self.highlight_colors[self.frame_idx]
    }
}

pub struct HighlightEffect {
    chars: Vec<CharHighlight>,
    // Diagonal groups: chars sorted by (x + y) diagonal index
    groups: Vec<Vec<usize>>,  // group_idx → list of char indices
    // Easer state
    total_groups: usize,
    easer_step: f64,
    easer_speed: f64,
    activated_up_to: usize,   // how many groups have been activated
    width: usize,
    height: usize,
}

impl HighlightEffect {
    pub fn new(grid: &Grid) -> Self {
        let width = grid.width;
        let height = grid.height;
        let dm: usize = 2;

        let highlight_brightness = 1.75_f64;
        let highlight_width = 8_usize;
        let frames_per_tick = 2 * dm;

        let final_gradient = Gradient::new(
            &[Rgb::from_hex("8A008A"), Rgb::from_hex("00D1FF"), Rgb::from_hex("FFFFFF")],
            12,
        );

        // Build per-character data
        let mut chars: Vec<CharHighlight> = Vec::with_capacity(width * height);
        // Group by diagonal (bottom-left to top-right: group key = x + (height-1-y))
        let max_diag = width + height;
        let mut groups: Vec<Vec<usize>> = vec![Vec::new(); max_diag];

        for y in 0..height {
            for x in 0..width {
                let base_color = final_gradient.color_at_coord(
                    y, x, height, width, GradientDirection::Vertical,
                );

                // Build highlight gradient: base → bright → bright → base
                // with steps=(3, highlight_width, 3)
                let bright = base_color.adjust_brightness(highlight_brightness);
                let ramp_up = 3;
                let ramp_down = 3;
                let total_steps = ramp_up + highlight_width + ramp_down;
                let mut highlight_colors = Vec::with_capacity(total_steps);

                // Ramp up: base → bright (3 steps)
                for i in 0..ramp_up {
                    let t = (i + 1) as f64 / ramp_up as f64;
                    highlight_colors.push(Rgb::lerp(base_color, bright, t));
                }
                // Peak: bright (highlight_width steps)
                for _ in 0..highlight_width {
                    highlight_colors.push(bright);
                }
                // Ramp down: bright → base (3 steps)
                for i in 0..ramp_down {
                    let t = (i + 1) as f64 / ramp_down as f64;
                    highlight_colors.push(Rgb::lerp(bright, base_color, t));
                }

                let idx = chars.len();
                // Diagonal: bottom-left to top-right
                let diag = x + (height.saturating_sub(1).saturating_sub(y));
                if diag < max_diag {
                    groups[diag].push(idx);
                }

                chars.push(CharHighlight {
                    y,
                    x,
                    base_color,
                    highlight_colors,
                    frame_idx: 0,
                    frames_per_tick,
                    tick_count: 0,
                    active: false,
                    done: false,
                });
            }
        }

        // Remove empty groups
        groups.retain(|g| !g.is_empty());
        let total_groups = groups.len();

        // Easer speed: step through all groups over a reasonable duration
        // in_out_circ easing applied to progress
        let easer_speed = 1.0 / (total_groups as f64 * dm as f64).max(1.0);

        HighlightEffect {
            chars,
            groups,
            total_groups,
            easer_step: 0.0,
            easer_speed,
            activated_up_to: 0,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        // Advance easer and activate groups
        self.easer_step += self.easer_speed;
        if self.easer_step > 1.0 {
            self.easer_step = 1.0;
        }

        let eased = easing::in_out_circ(self.easer_step);
        let target_group = (eased * self.total_groups as f64).round() as usize;
        let target_group = target_group.min(self.total_groups);

        // Activate newly reached groups
        while self.activated_up_to < target_group {
            for &char_idx in &self.groups[self.activated_up_to] {
                self.chars[char_idx].active = true;
            }
            self.activated_up_to += 1;
        }

        // Tick all active animations
        let mut any_active = false;
        for ch in &mut self.chars {
            ch.tick();
            if ch.active && !ch.done {
                any_active = true;
            }
        }

        // Check completion
        let all_activated = self.activated_up_to >= self.total_groups;
        let complete = all_activated && !any_active;

        // Render
        for ch in &self.chars {
            if ch.y < grid.height && ch.x < grid.width {
                let cell = &mut grid.cells[ch.y][ch.x];
                cell.visible = true;
                cell.fg = Some(ch.current_color().to_crossterm());
            }
        }

        if complete {
            // Ensure final colors
            for ch in &self.chars {
                if ch.y < grid.height && ch.x < grid.width {
                    let cell = &mut grid.cells[ch.y][ch.x];
                    cell.visible = true;
                    cell.fg = Some(ch.base_color.to_crossterm());
                }
            }
            return true;
        }

        false
    }
}
