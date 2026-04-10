// ColorShift effect — animated gradient cycle with traveling wave
use crate::engine::Grid;
use crate::gradient::{Gradient, Rgb, GradientDirection};

pub struct ColorShiftEffect {
    frame: usize,
    dm: usize,
    width: usize,
    height: usize,
    spectrum: Vec<Rgb>,
    gradient_frames: usize,
    cycles: usize,
    current_cycle: usize,
    spectrum_offset: usize,
    step_counter: usize,
    final_gradient: Gradient,
    done: bool,
    original: Vec<Vec<char>>,
}

impl ColorShiftEffect {
    pub fn new(grid: &Grid) -> Self {
        let (width, height, dm) = (grid.width, grid.height, 2usize);

        // Rainbow spectrum
        let rainbow_stops = [
            Rgb::from_hex("e81416"), Rgb::from_hex("ffa500"), Rgb::from_hex("faeb36"),
            Rgb::from_hex("79c314"), Rgb::from_hex("487de7"), Rgb::from_hex("4b369d"),
            Rgb::from_hex("70369d"),
        ];
        let spectrum_gradient = Gradient::new(&rainbow_stops, 12);
        // Build a looping spectrum
        let mut spectrum = spectrum_gradient.spectrum().to_vec();
        // Add reversed for smooth loop
        let mut rev = spectrum.clone();
        rev.reverse();
        rev.pop(); // avoid duplicate at seam
        spectrum.extend(rev);

        let final_gradient = Gradient::new(&rainbow_stops, 12);

        let mut original = Vec::new();
        for y in 0..height {
            let row: Vec<char> = (0..width).map(|x| grid.cells[y][x].ch).collect();
            original.push(row);
        }

        ColorShiftEffect {
            frame: 0, dm, width, height, spectrum,
            gradient_frames: 2 * dm, cycles: 3,
            current_cycle: 0, spectrum_offset: 0, step_counter: 0,
            final_gradient, done: false, original,
        }
    }

    pub fn tick(&mut self, grid: &mut Grid) -> bool {
        if self.done { return true; }
        self.frame += 1;
        self.step_counter += 1;

        if self.step_counter >= self.gradient_frames {
            self.step_counter = 0;
            self.spectrum_offset += 1;
            if self.spectrum_offset >= self.spectrum.len() {
                self.spectrum_offset = 0;
                self.current_cycle += 1;
                if self.current_cycle >= self.cycles {
                    self.done = true;
                    // Final state
                    for y in 0..self.height { for x in 0..self.width {
                        let cell = &mut grid.cells[y][x];
                        cell.visible = true;
                        cell.ch = self.original[y][x];
                        let fc = self.final_gradient.color_at_coord(y, x, self.height, self.width, GradientDirection::Vertical);
                        cell.fg = Some(fc.to_crossterm());
                    }}
                    return true;
                }
            }
        }

        let spec_len = self.spectrum.len();
        // Radial travel direction
        let center_y = self.height as f64 / 2.0;
        let center_x = self.width as f64 / 2.0;
        let max_dist = ((center_y * center_y) + (center_x * center_x)).sqrt();

        for y in 0..self.height { for x in 0..self.width {
            let cell = &mut grid.cells[y][x];
            cell.visible = true;
            cell.ch = self.original[y][x];

            // Radial distance index
            let dy = y as f64 - center_y;
            let dx = x as f64 - center_x;
            let dist = (dy * dy + dx * dx).sqrt();
            let norm_dist = dist / max_dist.max(1.0);
            let shift = (norm_dist * spec_len as f64) as usize;
            let idx = (self.spectrum_offset + shift) % spec_len;
            cell.fg = Some(self.spectrum[idx].to_crossterm());
        }}

        false
    }
}
