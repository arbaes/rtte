// Decrypt effect — faithful TTE reimplementation
//
// Three phases: typing → fast_decrypt → slow_decrypt → discovered
// Each character independently transitions through scenes via events.

use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use rand::Rng;

#[derive(Clone)]
struct SceneFrame {
    symbol: char,
    color: Rgb,
    duration: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum CharPhase {
    Pending,
    Typing,
    FastDecrypt,
    SlowDecrypt,
    Discovered,
    Done,
}

#[derive(Clone)]
struct CharAnim {
    y: usize,
    x: usize,
    original_ch: char,
    visible: bool,
    phase: CharPhase,
    scene: Vec<SceneFrame>,
    scene_idx: usize,
    hold_count: usize,
    scene_complete: bool,
    final_color: Rgb,
    cipher_color: Rgb,
    // Pre-built scenes
    fast_decrypt_scene: Vec<SceneFrame>,
    slow_decrypt_scene: Vec<SceneFrame>,
    discovered_scene: Vec<SceneFrame>,
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
        if self.scene.is_empty() {
            return self.original_ch;
        }
        self.scene[self.scene_idx].symbol
    }

    fn current_color(&self) -> Rgb {
        if self.scene.is_empty() {
            return self.final_color;
        }
        self.scene[self.scene_idx].color
    }

    fn transition_to_next_phase(&mut self) {
        match self.phase {
            CharPhase::Typing => {
                self.phase = CharPhase::FastDecrypt;
                self.scene = self.fast_decrypt_scene.clone();
                self.scene_idx = 0;
                self.hold_count = 0;
                self.scene_complete = false;
            }
            CharPhase::FastDecrypt => {
                self.phase = CharPhase::SlowDecrypt;
                self.scene = self.slow_decrypt_scene.clone();
                self.scene_idx = 0;
                self.hold_count = 0;
                self.scene_complete = false;
            }
            CharPhase::SlowDecrypt => {
                self.phase = CharPhase::Discovered;
                self.scene = self.discovered_scene.clone();
                self.scene_idx = 0;
                self.hold_count = 0;
                self.scene_complete = false;
            }
            CharPhase::Discovered => {
                self.phase = CharPhase::Done;
                self.scene_complete = true;
            }
            _ => {}
        }
    }
}

pub struct DecryptEffect {
    chars: Vec<CharAnim>,
    typing_order: Vec<usize>,
    typing_pos: usize,
    typing_speed: usize,
    active_indices: Vec<usize>,
    phase: EffectPhase,
    encrypted_symbols: Vec<char>,
    width: usize,
    height: usize,
}

#[derive(PartialEq)]
enum EffectPhase {
    Typing,
    Decrypting,
    Complete,
}

impl DecryptEffect {
    pub fn new(grid: &Grid) -> Self {
        let mut rng = rand::thread_rng();
        let width = grid.width;
        let height = grid.height;

        let cipher_colors = [
            Rgb::from_hex("008000"),
            Rgb::from_hex("00cb00"),
            Rgb::from_hex("00ff00"),
        ];

        // TTE default: single orange color
        let final_gradient = Gradient::new(
            &[
                Rgb::from_hex("8A008A"),
                Rgb::from_hex("00D1FF"),
                Rgb::from_hex("ffffff"),
            ],
            12,
        );

        // Build encrypted symbol pool
        let mut encrypted_symbols: Vec<char> = Vec::new();
        // ASCII printable
        for b in 33u8..=126 {
            encrypted_symbols.push(b as char);
        }
        // Block drawing
        for c in '█'..='▓' {
            encrypted_symbols.push(c);
        }
        encrypted_symbols.extend(&['░', '▒', '▓', '█', '▉', '▊', '▋', '▌', '▍', '▎', '▏', '▐']);

        let block_chars = ['▉', '▓', '▒', '░'];

        // Duration multiplier for 60fps (TTE runs ~25fps effectively)
        let dm: usize = 2;

        // Build per-char state
        let mut chars = Vec::with_capacity(height * width);
        for y in 0..height {
            for x in 0..width {
                let original_ch = grid.cells[y][x].ch;
                let final_color =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);
                let cipher_color = cipher_colors[rng.gen_range(0..cipher_colors.len())];

                // Build typing scene: 4 block chars + 1 encrypted symbol
                let mut typing_scene = Vec::new();
                for &blk in &block_chars {
                    typing_scene.push(SceneFrame {
                        symbol: blk,
                        color: cipher_colors[rng.gen_range(0..cipher_colors.len())],
                        duration: 2 * dm,
                    });
                }
                typing_scene.push(SceneFrame {
                    symbol: encrypted_symbols[rng.gen_range(0..encrypted_symbols.len())],
                    color: cipher_color,
                    duration: 1 * dm,
                });

                // Build fast_decrypt scene: 80 random symbols
                let fast_decrypt_scene: Vec<SceneFrame> = (0..80)
                    .map(|_| SceneFrame {
                        symbol: encrypted_symbols[rng.gen_range(0..encrypted_symbols.len())],
                        color: cipher_color,
                        duration: 2 * dm,
                    })
                    .collect();

                // Build slow_decrypt scene: 1-15 iterations with variable timing
                let slow_iters = rng.gen_range(1..=15);
                let slow_decrypt_scene: Vec<SceneFrame> = (0..slow_iters)
                    .map(|_| {
                        let dur = if rng.gen_range(0..100) < 30 {
                            rng.gen_range(35..=60) * dm
                        } else {
                            rng.gen_range(3..=6) * dm
                        };
                        SceneFrame {
                            symbol: encrypted_symbols[rng.gen_range(0..encrypted_symbols.len())],
                            color: cipher_color,
                            duration: dur,
                        }
                    })
                    .collect();

                // Build discovered scene: white → final_color gradient
                let discover_steps = 10;
                let mut discovered_scene: Vec<SceneFrame> = (0..discover_steps)
                    .map(|i| {
                        let t = (i + 1) as f64 / discover_steps as f64;
                        let color = Rgb::lerp(Rgb::new(255, 255, 255), final_color, t);
                        SceneFrame {
                            symbol: original_ch,
                            color,
                            duration: 2 * dm,
                        }
                    })
                    .collect();
                // Final hold
                discovered_scene.push(SceneFrame {
                    symbol: original_ch,
                    color: final_color,
                    duration: 5 * dm,
                });

                chars.push(CharAnim {
                    y,
                    x,
                    original_ch,
                    visible: false,
                    phase: CharPhase::Pending,
                    scene: typing_scene,
                    scene_idx: 0,
                    hold_count: 0,
                    scene_complete: false,
                    final_color,
                    cipher_color,
                    fast_decrypt_scene,
                    slow_decrypt_scene,
                    discovered_scene,
                });
            }
        }

        // Typing order: left-to-right, top-to-bottom
        let typing_order: Vec<usize> = (0..chars.len()).collect();

        DecryptEffect {
            chars,
            typing_order,
            typing_pos: 0,
            typing_speed: 2,
            active_indices: Vec::new(),
            phase: EffectPhase::Typing,
            encrypted_symbols,
            width,
            height,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        let mut rng = rand::thread_rng();

        match self.phase {
            EffectPhase::Typing => {
                // 75% chance to type this frame
                if self.typing_pos < self.typing_order.len() && rng.gen_range(0..100) <= 75 {
                    for _ in 0..self.typing_speed {
                        if self.typing_pos >= self.typing_order.len() {
                            break;
                        }

                        // Skip spaces
                        while self.typing_pos < self.typing_order.len() {
                            let peek = self.typing_order[self.typing_pos];
                            if peek < self.chars.len() && self.chars[peek].original_ch == ' ' {
                                self.chars[peek].visible = true;
                                self.chars[peek].phase = CharPhase::Done;
                                self.chars[peek].scene_complete = true;
                                self.typing_pos += 1;
                            } else {
                                break;
                            }
                        }
                        if self.typing_pos >= self.typing_order.len() {
                            break;
                        }

                        let idx = self.typing_order[self.typing_pos];
                        self.typing_pos += 1;

                        if idx < self.chars.len() {
                            let ca = &mut self.chars[idx];
                            ca.visible = true;
                            ca.phase = CharPhase::Typing;
                            ca.scene_idx = 0;
                            ca.hold_count = 0;
                            ca.scene_complete = false;
                            self.active_indices.push(idx);
                        }
                    }
                }

                // Check if typing is done
                if self.typing_pos >= self.typing_order.len() {
                    let all_typed = self.active_indices.iter().all(|&idx| {
                        self.chars[idx].scene_complete || self.chars[idx].phase != CharPhase::Typing
                    });
                    if all_typed {
                        self.phase = EffectPhase::Decrypting;
                    }
                }
            }
            EffectPhase::Decrypting => {
                // Check completion
                let all_done = self.active_indices.is_empty()
                    || self
                        .active_indices
                        .iter()
                        .all(|&idx| self.chars[idx].phase == CharPhase::Done);
                if all_done {
                    self.phase = EffectPhase::Complete;
                }
            }
            EffectPhase::Complete => {
                // Set final state
                for ca in &self.chars {
                    if ca.y < grid.height && ca.x < grid.width {
                        let cell = &mut grid.cells[ca.y][ca.x];
                        cell.visible = true;
                        cell.ch = ca.original_ch;
                        cell.fg = Some(ca.final_color.to_crossterm());
                    }
                }
                return true;
            }
        }

        // Tick all active chars and handle transitions
        for &idx in &self.active_indices {
            let ca = &mut self.chars[idx];
            ca.tick();
            if ca.scene_complete && ca.phase != CharPhase::Done {
                ca.transition_to_next_phase();
            }
        }

        // Remove finished chars from active list
        self.active_indices
            .retain(|&idx| self.chars[idx].phase != CharPhase::Done);

        // Render to grid
        for ca in &self.chars {
            if ca.y < grid.height && ca.x < grid.width {
                let cell = &mut grid.cells[ca.y][ca.x];
                cell.visible = ca.visible;
                if ca.visible && ca.phase != CharPhase::Pending {
                    if ca.phase == CharPhase::Done {
                        cell.ch = ca.original_ch;
                        cell.fg = Some(ca.final_color.to_crossterm());
                    } else {
                        cell.ch = ca.current_symbol();
                        cell.fg = Some(ca.current_color().to_crossterm());
                    }
                }
            }
        }

        false
    }
}
