// Print effect — faithful TTE reimplementation
//
// Typewriter: print head moves L→R, types characters with block animation,
// carriage returns to next row with eased motion.

use crate::engine::Grid;
use crate::easing;
use crate::gradient::{Gradient, Rgb, GradientDirection};

#[derive(Clone)]
struct SceneFrame {
    symbol: char,
    color: Rgb,
    duration: usize,
}

#[derive(Clone)]
struct CharAnim {
    y: usize,
    x: usize,
    original_ch: char,
    visible: bool,
    scene: Vec<SceneFrame>,
    scene_idx: usize,
    hold_count: usize,
    scene_complete: bool,
    final_color: Rgb,
}

impl CharAnim {
    fn tick(&mut self) {
        if self.scene_complete || self.scene.is_empty() {
            return;
        }
        self.hold_count += 1;
        if self.hold_count >= self.scene[self.scene_idx].duration {
            self.hold_count = 0;
            self.scene_idx += 1;
            if self.scene_idx >= self.scene.len() {
                self.scene_idx = self.scene.len() - 1;
                self.scene_complete = true;
            }
        }
    }

    fn current_symbol(&self) -> char {
        if self.scene.is_empty() { return self.original_ch; }
        self.scene[self.scene_idx].symbol
    }

    fn current_color(&self) -> Rgb {
        if self.scene.is_empty() { return self.final_color; }
        self.scene[self.scene_idx].color
    }
}

#[derive(PartialEq)]
enum Phase {
    Printing,
    CarriageReturn,
    Complete,
}

pub struct PrintEffect {
    chars: Vec<Vec<CharAnim>>,  // [row][col]
    current_row: usize,
    col_pos: usize,            // next char to type in current row
    print_speed: usize,
    active_indices: Vec<(usize, usize)>,
    phase: Phase,
    // Print head
    head_visible: bool,
    head_y: usize,
    head_x: f64,
    // Carriage return motion
    cr_start_x: f64,
    cr_target_x: f64,
    cr_progress: f64,
    cr_speed: f64,
    width: usize,
    height: usize,
    // Duration multiplier for 60fps
    dm: usize,
}

impl PrintEffect {
    pub fn new(grid: &Grid) -> Self {
        let width = grid.width;
        let height = grid.height;
        let dm: usize = 2;

        let final_gradient = Gradient::new(
            &[Rgb::from_hex("02b8bd"), Rgb::from_hex("c1f0e3"), Rgb::from_hex("00ffa0")],
            12,
        );

        let block_syms = ['█', '▓', '▒', '░'];

        let mut chars: Vec<Vec<CharAnim>> = Vec::with_capacity(height);
        for y in 0..height {
            let mut row = Vec::with_capacity(width);
            for x in 0..width {
                let original_ch = grid.cells[y][x].ch;
                let final_color = final_gradient.color_at_coord(
                    y, x, height, width, GradientDirection::Diagonal,
                );

                // Build typed animation: █→▓→▒→░→original_ch
                // 5-step gradient from white to final color
                let mut scene = Vec::new();
                let grad_steps = 5;
                for (i, &sym) in block_syms.iter().enumerate() {
                    let t = (i + 1) as f64 / grad_steps as f64;
                    let color = Rgb::lerp(Rgb::new(255, 255, 255), final_color, t);
                    scene.push(SceneFrame {
                        symbol: sym,
                        color,
                        duration: 3 * dm,
                    });
                }
                // Final: original character at final color
                scene.push(SceneFrame {
                    symbol: original_ch,
                    color: final_color,
                    duration: 3 * dm,
                });

                row.push(CharAnim {
                    y, x, original_ch,
                    visible: false,
                    scene,
                    scene_idx: 0,
                    hold_count: 0,
                    scene_complete: false,
                    final_color,
                });
            }
            chars.push(row);
        }

        PrintEffect {
            chars,
            current_row: 0,
            col_pos: 0,
            print_speed: 2,
            active_indices: Vec::new(),
            phase: Phase::Printing,
            head_visible: true,
            head_y: 0,
            head_x: 0.0,
            cr_start_x: 0.0,
            cr_target_x: 0.0,
            cr_progress: 0.0,
            cr_speed: 0.03, // ~33 frames for full-width return at 60fps
            width,
            height,
            dm,
        }
    }

    fn first_non_space_col(&self, row: usize) -> usize {
        if row >= self.chars.len() { return 0; }
        self.chars[row].iter().position(|c| c.original_ch != ' ').unwrap_or(0)
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        match self.phase {
            Phase::Printing => {
                if self.current_row >= self.height {
                    self.phase = Phase::Complete;
                } else {
                    // Show print head during L→R typing
                    self.head_visible = true;
                    self.head_y = self.current_row;

                    // Type print_speed characters (spaces still consume movement)
                    let mut typed_this_frame = 0;
                    while typed_this_frame < self.print_speed && self.col_pos < self.width {
                        let ca = &mut self.chars[self.current_row][self.col_pos];
                        ca.visible = true;
                        ca.scene_idx = 0;
                        ca.hold_count = 0;
                        ca.scene_complete = false;

                        if ca.original_ch == ' ' {
                            ca.scene_complete = true;
                        } else {
                            self.active_indices.push((self.current_row, self.col_pos));
                        }

                        self.head_x = self.col_pos as f64;
                        self.col_pos += 1;
                        typed_this_frame += 1;
                    }

                    // Row complete?
                    if self.col_pos >= self.width {
                        if self.current_row + 1 < self.height {
                            // Start carriage return
                            self.phase = Phase::CarriageReturn;
                            self.cr_start_x = self.head_x;
                            self.cr_target_x = self.first_non_space_col(self.current_row + 1) as f64;
                            self.cr_progress = 0.0;
                            self.head_visible = true;
                        } else {
                            // Last row done
                            self.head_visible = false;
                            self.current_row += 1;
                        }
                    }
                }
            }

            Phase::CarriageReturn => {
                self.cr_progress += self.cr_speed;
                if self.cr_progress >= 1.0 {
                    self.cr_progress = 1.0;
                    // Carriage return complete
                    self.current_row += 1;
                    self.col_pos = 0;
                    self.head_visible = false;
                    self.phase = Phase::Printing;
                }
                // Eased position
                let t = easing::in_out_quad(self.cr_progress);
                self.head_x = self.cr_start_x + (self.cr_target_x - self.cr_start_x) * t;
                self.head_y = self.current_row; // stays on current row during return
            }

            Phase::Complete => {
                // Wait for all animations to finish
                let all_done = self.active_indices.is_empty();
                if all_done {
                    for row in &self.chars {
                        for ca in row {
                            if ca.y < grid.height && ca.x < grid.width {
                                let cell = &mut grid.cells[ca.y][ca.x];
                                cell.visible = true;
                                cell.ch = ca.original_ch;
                                cell.fg = Some(ca.final_color.to_crossterm());
                            }
                        }
                    }
                    return true;
                }
            }
        }

        // Tick all active character animations
        for &(row, col) in &self.active_indices {
            self.chars[row][col].tick();
        }
        self.active_indices.retain(|&(r, c)| !self.chars[r][c].scene_complete);

        // Render to grid
        for row in &self.chars {
            for ca in row {
                if ca.y < grid.height && ca.x < grid.width {
                    let cell = &mut grid.cells[ca.y][ca.x];
                    cell.visible = ca.visible;
                    if ca.visible {
                        if ca.scene_complete {
                            cell.ch = ca.original_ch;
                            cell.fg = Some(ca.final_color.to_crossterm());
                        } else if ca.scene.is_empty() || ca.scene_idx == 0 && ca.hold_count == 0 && !ca.scene_complete {
                            // Not yet started animating (space or pending)
                            cell.ch = ca.original_ch;
                            cell.fg = Some(ca.final_color.to_crossterm());
                        } else {
                            cell.ch = ca.current_symbol();
                            cell.fg = Some(ca.current_color().to_crossterm());
                        }
                    }
                }
            }
        }

        // Render print head
        if self.head_visible && self.phase != Phase::Complete {
            let hx = self.head_x.round() as usize;
            let hy = if self.phase == Phase::CarriageReturn {
                self.current_row + 1 // next row during CR
            } else {
                self.current_row
            };
            if hy < grid.height && hx < grid.width {
                let cell = &mut grid.cells[hy][hx];
                cell.visible = true;
                cell.ch = '█';
                cell.fg = Some(Rgb::new(255, 255, 255).to_crossterm());
            }
        }

        false
    }
}
