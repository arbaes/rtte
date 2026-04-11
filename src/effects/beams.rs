// Beams effect — faithful TTE reimplementation with per-character state machines
//
// Algorithm:
// 1. Characters grouped by row and column. Groups shuffled randomly.
// 2. Groups activated in batches (1-5) every beam_delay frames.
// 3. Each group has a speed counter; when it crosses 1.0, next character activates.
// 4. Activated character plays beam scene: beam_symbols with gradient, then char faded.
// 5. When all groups done, final diagonal wipe activates "brighten" scene per char.

use crate::engine::Grid;
use crate::gradient::{Gradient, GradientDirection, Rgb};
use crossterm::style::Color;
use rand::Rng;

// --- Per-character scene frame ---
#[derive(Clone)]
struct SceneFrame {
    symbol: char,
    color: Rgb,
    duration: usize, // ticks to hold this frame
}

// --- Per-character animation state ---
#[derive(Clone)]
struct CharAnim {
    y: usize,
    x: usize,
    original_ch: char,
    visible: bool,
    // Scene frames to play
    scene: Vec<SceneFrame>,
    scene_idx: usize,
    hold_count: usize,
    scene_complete: bool,
    // Final resting color (from final gradient)
    final_color: Rgb,
    // Faded color (30% brightness of final)
    faded_color: Rgb,
    // Has been activated at all
    activated: bool,
    // Is in brighten phase
    brightening: bool,
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
        if self.scene.is_empty() || !self.activated {
            return self.original_ch;
        }
        self.scene[self.scene_idx].symbol
    }

    fn current_color(&self) -> Rgb {
        if self.scene.is_empty() || !self.activated {
            return self.faded_color;
        }
        self.scene[self.scene_idx].color
    }
}

// --- Group of characters for beam traversal ---
struct Group {
    /// Indices into the chars array (in order they should activate)
    char_indices: Vec<usize>,
    next_idx: usize,
    direction: Direction,
    speed: f64,
    counter: f64,
}

#[derive(Clone, Copy)]
enum Direction {
    Row,
    Column,
}

impl Group {
    fn is_complete(&self) -> bool {
        self.next_idx >= self.char_indices.len()
    }

    /// Advance the smooth counter and return indices of newly-activated chars
    fn tick(&mut self) -> Vec<usize> {
        let mut activated = Vec::new();
        self.counter += self.speed;
        while self.counter >= 1.0 && self.next_idx < self.char_indices.len() {
            activated.push(self.char_indices[self.next_idx]);
            self.next_idx += 1;
            self.counter -= 1.0;
        }
        activated
    }
}

// --- Beams effect state ---
pub struct BeamsState {
    chars: Vec<CharAnim>,
    pending_groups: Vec<Group>,
    active_groups: Vec<Group>,
    final_wipe_groups: Vec<Vec<usize>>, // diagonal groups of char indices
    delay: usize,
    delay_max: usize,
    phase: Phase,
    final_wipe_speed: usize,
    width: usize,
    height: usize,
}

#[derive(PartialEq)]
enum Phase {
    Beams,
    FinalWipe,
    Complete,
}

impl BeamsState {
    pub fn new(grid: &Grid) -> Self {
        let mut rng = rand::thread_rng();
        let width = grid.width;
        let height = grid.height;

        // Build beam gradient: white → cyan → purple
        // steps=(2, 6) → 2 steps white→cyan, 6 steps cyan→purple = ~10 colors
        let beam_gradient = Gradient::new(
            &[
                Rgb::from_hex("ffffff"),
                Rgb::from_hex("00D1FF"),
                Rgb::from_hex("8A008A"),
            ],
            4, // gives us about 8-10 spectrum colors
        );
        let beam_spectrum = beam_gradient.spectrum().to_vec();

        // Build final gradient: purple → cyan → white, VERTICAL mapping
        let final_gradient = Gradient::new(
            &[
                Rgb::from_hex("8A008A"),
                Rgb::from_hex("00D1FF"),
                Rgb::from_hex("ffffff"),
            ],
            12,
        );

        let beam_row_symbols = ['▂', '▁', '_'];
        let beam_col_symbols = ['▌', '▍', '▎', '▏'];
        let beam_gradient_frames = 2; // ticks per beam symbol color
        let final_gradient_frames = 4; // ticks per brighten step

        // --- Build per-character state ---
        let mut chars = Vec::with_capacity(height * width);
        for y in 0..height {
            for x in 0..width {
                let original_ch = grid.cells[y][x].ch;
                let final_color =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);
                let faded_color = final_color.adjust_brightness(0.3);

                chars.push(CharAnim {
                    y,
                    x,
                    original_ch,
                    visible: false,
                    scene: Vec::new(), // will be set when activated
                    scene_idx: 0,
                    hold_count: 0,
                    scene_complete: false,
                    final_color,
                    faded_color,
                    activated: false,
                    brightening: false,
                });
            }
        }

        // Helper: build beam_row scene for a character
        let build_row_scene = |ch: char, faded: Rgb, beam_spec: &[Rgb]| -> Vec<SceneFrame> {
            let mut frames = Vec::new();
            // Beam symbols with gradient colors
            for (i, &sym) in beam_row_symbols.iter().enumerate() {
                let ci = if beam_spec.is_empty() {
                    0
                } else {
                    i * beam_spec.len() / beam_row_symbols.len().max(1)
                };
                let color = beam_spec[ci.min(beam_spec.len() - 1)];
                frames.push(SceneFrame {
                    symbol: sym,
                    color,
                    duration: beam_gradient_frames,
                });
            }
            // Then the actual character at faded brightness
            frames.push(SceneFrame {
                symbol: ch,
                color: faded,
                duration: beam_gradient_frames,
            });
            frames
        };

        let build_col_scene = |ch: char, faded: Rgb, beam_spec: &[Rgb]| -> Vec<SceneFrame> {
            let mut frames = Vec::new();
            for (i, &sym) in beam_col_symbols.iter().enumerate() {
                let ci = if beam_spec.is_empty() {
                    0
                } else {
                    i * beam_spec.len() / beam_col_symbols.len().max(1)
                };
                let color = beam_spec[ci.min(beam_spec.len() - 1)];
                frames.push(SceneFrame {
                    symbol: sym,
                    color,
                    duration: beam_gradient_frames,
                });
            }
            frames.push(SceneFrame {
                symbol: ch,
                color: faded,
                duration: beam_gradient_frames,
            });
            frames
        };

        let build_brighten_scene = |ch: char, faded: Rgb, final_c: Rgb| -> Vec<SceneFrame> {
            let mut frames = Vec::new();
            let steps = 10;
            for i in 0..steps {
                let t = i as f64 / (steps - 1) as f64;
                let color = Rgb::lerp(faded, final_c, t);
                frames.push(SceneFrame {
                    symbol: ch,
                    color,
                    duration: final_gradient_frames,
                });
            }
            frames
        };

        // --- Build groups ---
        let mut groups: Vec<Group> = Vec::new();

        // ROW groups
        for y in 0..height {
            let mut indices: Vec<usize> = (0..width).map(|x| y * width + x).collect();
            // 50% chance to reverse
            if rng.gen_bool(0.5) {
                indices.reverse();
            }

            // Pre-build row scenes for each char in this group
            for &idx in &indices {
                let ca = &chars[idx];
                let scene = build_row_scene(ca.original_ch, ca.faded_color, &beam_spectrum);
                // Store scene — but only set it when the char is first activated
                // We'll store row scenes in a side buffer
                // Actually: store both scenes and pick when activating
                // For now, we just store the row scene
                let _ = scene; // We'll set it during activation
            }

            let speed = rng.gen_range(15..=60) as f64 * 0.1;
            groups.push(Group {
                char_indices: indices,
                next_idx: 0,
                direction: Direction::Row,
                speed,
                counter: 0.0,
            });
        }

        // COLUMN groups
        for x in 0..width {
            let mut indices: Vec<usize> = (0..height).map(|y| y * width + x).collect();
            if rng.gen_bool(0.5) {
                indices.reverse();
            }
            let speed = rng.gen_range(9..=15) as f64 * 0.1;
            groups.push(Group {
                char_indices: indices,
                next_idx: 0,
                direction: Direction::Column,
                speed,
                counter: 0.0,
            });
        }

        // Shuffle all groups
        use rand::seq::SliceRandom;
        groups.shuffle(&mut rng);

        // Pre-store scenes for each char based on which group activates them first
        // We'll lazy-build scenes on activation
        // Store beam_spectrum and scene builders as captured state
        // Actually: we need to store beam scenes per-char. Since a char can be in both
        // a row group and a column group, the first activation sets the scene.
        // We'll pre-compute both scene types and select at activation time.

        // Store pre-computed scenes in a parallel array
        // row_scenes[idx] and col_scenes[idx] for each char
        let row_scenes: Vec<Vec<SceneFrame>> = chars
            .iter()
            .map(|ca| build_row_scene(ca.original_ch, ca.faded_color, &beam_spectrum))
            .collect();
        let col_scenes: Vec<Vec<SceneFrame>> = chars
            .iter()
            .map(|ca| build_col_scene(ca.original_ch, ca.faded_color, &beam_spectrum))
            .collect();
        let brighten_scenes: Vec<Vec<SceneFrame>> = chars
            .iter()
            .map(|ca| build_brighten_scene(ca.original_ch, ca.faded_color, ca.final_color))
            .collect();

        // Store scenes in chars — we'll use a two-scene approach:
        // scene is initially empty, set on first activation based on Direction
        // When brighten phase hits, replace scene with brighten_scene
        // We need to keep brighten_scenes accessible, so let's store them in state

        // For group activation, we need to know which scene to set.
        // Let's tag each group with its direction, and on activation:
        //   if !char.activated: set scene = row_scenes or col_scenes, mark activated
        //   if char.activated: reset scene (re-trigger)

        // Store the pre-computed scenes in the state
        // We'll use a simple approach: store row_scenes, col_scenes, brighten_scenes as fields

        // --- Build diagonal groups for final wipe ---
        let mut diag_map: std::collections::BTreeMap<usize, Vec<usize>> =
            std::collections::BTreeMap::new();
        for y in 0..height {
            for x in 0..width {
                let diag = x + y;
                diag_map.entry(diag).or_default().push(y * width + x);
            }
        }
        let final_wipe_groups: Vec<Vec<usize>> = diag_map.into_values().collect();

        BeamsState {
            chars,
            pending_groups: groups,
            active_groups: Vec::new(),
            final_wipe_groups,
            delay: 0,
            delay_max: 6, // beam_delay
            phase: Phase::Beams,
            final_wipe_speed: 3,
            width,
            height,
            // We need to store the pre-computed scenes — add fields
        }
    }
}

// We need a wrapper to hold scenes separately since CharAnim doesn't own them until activation
pub struct BeamsEffect {
    pub state: BeamsState,
    row_scenes: Vec<Vec<SceneFrame>>,
    col_scenes: Vec<Vec<SceneFrame>>,
    brighten_scenes: Vec<Vec<SceneFrame>>,
    active_char_indices: Vec<usize>, // chars with active (non-complete) scenes
}

impl BeamsEffect {
    pub fn new(grid: &Grid) -> Self {
        let mut rng = rand::thread_rng();
        let width = grid.width;
        let height = grid.height;

        let beam_gradient = Gradient::new(
            &[
                Rgb::from_hex("ffffff"),
                Rgb::from_hex("00D1FF"),
                Rgb::from_hex("8A008A"),
            ],
            4,
        );
        let beam_spectrum = beam_gradient.spectrum().to_vec();

        let final_gradient = Gradient::new(
            &[
                Rgb::from_hex("8A008A"),
                Rgb::from_hex("00D1FF"),
                Rgb::from_hex("ffffff"),
            ],
            12,
        );

        let beam_row_symbols = ['▂', '▁', '_'];
        let beam_col_symbols = ['▌', '▍', '▎', '▏'];
        let beam_gradient_frames: usize = 4;
        let final_gradient_frames: usize = 8;

        // Build per-char state
        let mut chars = Vec::with_capacity(height * width);
        for y in 0..height {
            for x in 0..width {
                let original_ch = grid.cells[y][x].ch;
                let final_color =
                    final_gradient.color_at_coord(y, x, height, width, GradientDirection::Vertical);
                let faded_color = final_color.adjust_brightness(0.3);
                chars.push(CharAnim {
                    y,
                    x,
                    original_ch,
                    visible: false,
                    scene: Vec::new(),
                    scene_idx: 0,
                    hold_count: 0,
                    scene_complete: false,
                    final_color,
                    faded_color,
                    activated: false,
                    brightening: false,
                });
            }
        }

        // Pre-compute scenes
        let row_scenes: Vec<Vec<SceneFrame>> = chars
            .iter()
            .map(|ca| {
                let mut frames = Vec::new();
                for (i, &sym) in beam_row_symbols.iter().enumerate() {
                    let ci = i * beam_spectrum.len() / beam_row_symbols.len().max(1);
                    let color = beam_spectrum[ci.min(beam_spectrum.len() - 1)];
                    frames.push(SceneFrame {
                        symbol: sym,
                        color,
                        duration: beam_gradient_frames,
                    });
                }
                frames.push(SceneFrame {
                    symbol: ca.original_ch,
                    color: ca.faded_color,
                    duration: beam_gradient_frames,
                });
                frames
            })
            .collect();

        let col_scenes: Vec<Vec<SceneFrame>> = chars
            .iter()
            .map(|ca| {
                let mut frames = Vec::new();
                for (i, &sym) in beam_col_symbols.iter().enumerate() {
                    let ci = i * beam_spectrum.len() / beam_col_symbols.len().max(1);
                    let color = beam_spectrum[ci.min(beam_spectrum.len() - 1)];
                    frames.push(SceneFrame {
                        symbol: sym,
                        color,
                        duration: beam_gradient_frames,
                    });
                }
                frames.push(SceneFrame {
                    symbol: ca.original_ch,
                    color: ca.faded_color,
                    duration: beam_gradient_frames,
                });
                frames
            })
            .collect();

        let brighten_scenes: Vec<Vec<SceneFrame>> = chars
            .iter()
            .map(|ca| {
                let steps = 10;
                (0..steps)
                    .map(|i| {
                        let t = i as f64 / (steps - 1) as f64;
                        let color = Rgb::lerp(ca.faded_color, ca.final_color, t);
                        SceneFrame {
                            symbol: ca.original_ch,
                            color,
                            duration: final_gradient_frames,
                        }
                    })
                    .collect()
            })
            .collect();

        // Build groups
        let mut groups: Vec<Group> = Vec::new();

        // Row groups
        for y in 0..height {
            let mut indices: Vec<usize> = (0..width).map(|x| y * width + x).collect();
            if rng.gen_bool(0.5) {
                indices.reverse();
            }
            let speed = rng.gen_range(15..=60) as f64 * 0.05;
            groups.push(Group {
                char_indices: indices,
                next_idx: 0,
                direction: Direction::Row,
                speed,
                counter: 0.0,
            });
        }

        // Column groups
        for x in 0..width {
            let mut indices: Vec<usize> = (0..height).map(|y| y * width + x).collect();
            if rng.gen_bool(0.5) {
                indices.reverse();
            }
            let speed = rng.gen_range(9..=15) as f64 * 0.05;
            groups.push(Group {
                char_indices: indices,
                next_idx: 0,
                direction: Direction::Column,
                speed,
                counter: 0.0,
            });
        }

        use rand::seq::SliceRandom;
        groups.shuffle(&mut rng);

        // Diagonal groups for final wipe
        let mut diag_map: std::collections::BTreeMap<usize, Vec<usize>> =
            std::collections::BTreeMap::new();
        for y in 0..height {
            for x in 0..width {
                diag_map.entry(x + y).or_default().push(y * width + x);
            }
        }
        let final_wipe_groups: Vec<Vec<usize>> = diag_map.into_values().collect();

        BeamsEffect {
            state: BeamsState {
                chars,
                pending_groups: groups,
                active_groups: Vec::new(),
                final_wipe_groups,
                delay: 0,
                delay_max: 12,
                phase: Phase::Beams,
                final_wipe_speed: 3,
                width,
                height,
            },
            row_scenes,
            col_scenes,
            brighten_scenes,
            active_char_indices: Vec::new(),
        }
    }

    /// Process one frame. Returns true when effect is complete.
    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        let s = &mut self.state;

        match s.phase {
            Phase::Beams => {
                // Group activation with delay
                if s.delay == 0 {
                    let mut rng = rand::thread_rng();
                    let count = rng.gen_range(1..=5);
                    for _ in 0..count {
                        if let Some(group) = s.pending_groups.pop() {
                            s.active_groups.push(group);
                        }
                    }
                    s.delay = s.delay_max;
                } else {
                    s.delay -= 1;
                }

                // Process active groups — activate characters
                for group in &mut s.active_groups {
                    let newly_activated = group.tick();
                    for idx in newly_activated {
                        if idx < s.chars.len() {
                            let ca = &mut s.chars[idx];
                            if !ca.activated {
                                // First activation
                                ca.activated = true;
                                ca.visible = true;
                                ca.scene = match group.direction {
                                    Direction::Row => self.row_scenes[idx].clone(),
                                    Direction::Column => self.col_scenes[idx].clone(),
                                };
                                ca.scene_idx = 0;
                                ca.hold_count = 0;
                                ca.scene_complete = false;
                                self.active_char_indices.push(idx);
                            } else {
                                // Re-hit by another group — reset scene
                                ca.scene = match group.direction {
                                    Direction::Row => self.row_scenes[idx].clone(),
                                    Direction::Column => self.col_scenes[idx].clone(),
                                };
                                ca.scene_idx = 0;
                                ca.hold_count = 0;
                                ca.scene_complete = false;
                            }
                        }
                    }
                }

                // Remove completed groups
                s.active_groups.retain(|g| !g.is_complete());

                // Check phase transition
                if s.pending_groups.is_empty() && s.active_groups.is_empty() {
                    // Check if all active chars have completed their scenes
                    let all_scenes_done = self
                        .active_char_indices
                        .iter()
                        .all(|&idx| s.chars[idx].scene_complete);
                    if all_scenes_done {
                        s.phase = Phase::FinalWipe;
                        self.active_char_indices.clear();
                    }
                }
            }

            Phase::FinalWipe => {
                // Activate diagonal groups
                for _ in 0..s.final_wipe_speed {
                    if let Some(diag_group) = s.final_wipe_groups.first() {
                        let group = s.final_wipe_groups.remove(0);
                        for idx in group {
                            if idx < s.chars.len() {
                                let ca = &mut s.chars[idx];
                                ca.visible = true;
                                ca.activated = true;
                                ca.brightening = true;
                                ca.scene = self.brighten_scenes[idx].clone();
                                ca.scene_idx = 0;
                                ca.hold_count = 0;
                                ca.scene_complete = false;
                                self.active_char_indices.push(idx);
                            }
                        }
                    } else {
                        s.phase = Phase::Complete;
                        break;
                    }
                }
            }

            Phase::Complete => {
                // Check if all animations done
                let all_done = self
                    .active_char_indices
                    .iter()
                    .all(|&idx| s.chars[idx].scene_complete);
                if all_done {
                    // Set final state
                    for ca in &s.chars {
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
        }

        // Tick all active character animations
        for &idx in &self.active_char_indices {
            s.chars[idx].tick();
        }

        // Remove completed chars from active list (but keep them visible)
        self.active_char_indices
            .retain(|&idx| !s.chars[idx].scene_complete);

        // Render to grid
        for ca in &s.chars {
            if ca.y < grid.height && ca.x < grid.width {
                let cell = &mut grid.cells[ca.y][ca.x];
                cell.visible = ca.visible;
                if ca.activated {
                    if ca.scene_complete {
                        // Show original char at faded color (between beam and brighten)
                        cell.ch = ca.original_ch;
                        cell.fg = Some(ca.faded_color.to_crossterm());
                    } else {
                        cell.ch = ca.current_symbol();
                        cell.fg = Some(ca.current_color().to_crossterm());
                    }
                }
                // If brightened and complete, show final color
                if ca.brightening && ca.scene_complete {
                    cell.ch = ca.original_ch;
                    cell.fg = Some(ca.final_color.to_crossterm());
                }
            }
        }

        false
    }
}
