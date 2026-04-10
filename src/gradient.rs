/// Color gradient system matching TTE's gradient engine.
/// Supports multi-stop linear interpolation, coordinate mapping, and direction-based gradients.

use crossterm::style::Color;

/// An RGB color for interpolation
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Self { r, g, b }
    }

    pub fn to_crossterm(self) -> Color {
        Color::Rgb { r: self.r, g: self.g, b: self.b }
    }

    /// Adjust brightness by a factor (0.0 = black, 1.0 = same, 2.0 = double)
    pub fn adjust_brightness(self, factor: f64) -> Rgb {
        Rgb {
            r: (self.r as f64 * factor).clamp(0.0, 255.0) as u8,
            g: (self.g as f64 * factor).clamp(0.0, 255.0) as u8,
            b: (self.b as f64 * factor).clamp(0.0, 255.0) as u8,
        }
    }

    /// Lerp between two colors
    pub fn lerp(a: Rgb, b: Rgb, t: f64) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        Rgb {
            r: (a.r as f64 + (b.r as f64 - a.r as f64) * t) as u8,
            g: (a.g as f64 + (b.g as f64 - a.g as f64) * t) as u8,
            b: (a.b as f64 + (b.b as f64 - a.b as f64) * t) as u8,
        }
    }
}

/// Direction for coordinate-mapped gradients
#[derive(Clone, Copy, Debug)]
pub enum GradientDirection {
    Vertical,
    Horizontal,
    Diagonal,
    Radial,
}

/// A multi-stop color gradient
#[derive(Clone, Debug)]
pub struct Gradient {
    pub stops: Vec<Rgb>,
    pub steps: usize,
    spectrum: Vec<Rgb>,
}

impl Gradient {
    /// Create a gradient with the given color stops and number of interpolation steps per segment
    pub fn new(stops: &[Rgb], steps: usize) -> Self {
        let steps = steps.max(1);
        let mut spectrum = Vec::new();

        if stops.is_empty() {
            spectrum.push(Rgb::new(255, 255, 255));
        } else if stops.len() == 1 {
            spectrum.push(stops[0]);
        } else {
            for i in 0..stops.len() - 1 {
                for s in 0..steps {
                    let t = s as f64 / steps as f64;
                    spectrum.push(Rgb::lerp(stops[i], stops[i + 1], t));
                }
            }
            spectrum.push(*stops.last().unwrap());
        }

        Self {
            stops: stops.to_vec(),
            steps,
            spectrum,
        }
    }

    /// Get the color at position t in [0.0, 1.0]
    pub fn at(&self, t: f64) -> Rgb {
        if self.spectrum.is_empty() {
            return Rgb::new(255, 255, 255);
        }
        let t = t.clamp(0.0, 1.0);
        let idx = (t * (self.spectrum.len() - 1) as f64) as usize;
        self.spectrum[idx.min(self.spectrum.len() - 1)]
    }

    /// Get full spectrum
    pub fn spectrum(&self) -> &[Rgb] {
        &self.spectrum
    }

    /// Number of colors in the spectrum
    pub fn len(&self) -> usize {
        self.spectrum.len()
    }

    /// Get color by index
    pub fn get(&self, idx: usize) -> Rgb {
        self.spectrum[idx.min(self.spectrum.len() - 1)]
    }

    /// Build a coordinate-to-color mapping for the given bounds
    pub fn color_at_coord(
        &self,
        row: usize,
        col: usize,
        max_row: usize,
        max_col: usize,
        direction: GradientDirection,
    ) -> Rgb {
        let t = match direction {
            GradientDirection::Vertical => {
                if max_row == 0 { 0.0 } else { 1.0 - row as f64 / max_row as f64 }
            }
            GradientDirection::Horizontal => {
                if max_col == 0 { 0.0 } else { col as f64 / max_col as f64 }
            }
            GradientDirection::Diagonal => {
                let max = max_row + max_col;
                if max == 0 { 0.0 } else { (row + col) as f64 / max as f64 }
            }
            GradientDirection::Radial => {
                let cr = max_row as f64 / 2.0;
                let cc = max_col as f64 / 2.0;
                let max_dist = (cr * cr + cc * cc).sqrt();
                if max_dist == 0.0 {
                    0.0
                } else {
                    let dr = row as f64 - cr;
                    let dc = col as f64 - cc;
                    (dr * dr + dc * dc).sqrt() / max_dist
                }
            }
        };
        self.at(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_from_hex() {
        let c = Rgb::from_hex("ff8000");
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn rgb_from_hex_with_hash() {
        let c = Rgb::from_hex("#00ff00");
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn rgb_lerp_midpoint() {
        let a = Rgb::new(0, 0, 0);
        let b = Rgb::new(200, 100, 50);
        let mid = Rgb::lerp(a, b, 0.5);
        assert_eq!(mid.r, 100);
        assert_eq!(mid.g, 50);
        assert_eq!(mid.b, 25);
    }

    #[test]
    fn rgb_lerp_clamped() {
        let a = Rgb::new(10, 10, 10);
        let b = Rgb::new(20, 20, 20);
        assert_eq!(Rgb::lerp(a, b, -1.0), a);
        assert_eq!(Rgb::lerp(a, b, 2.0), b);
    }

    #[test]
    fn rgb_adjust_brightness() {
        let c = Rgb::new(100, 100, 100);
        let doubled = c.adjust_brightness(2.0);
        assert_eq!(doubled.r, 200);
        let zeroed = c.adjust_brightness(0.0);
        assert_eq!(zeroed.r, 0);
        // clamped at 255
        let capped = c.adjust_brightness(10.0);
        assert_eq!(capped.r, 255);
    }

    #[test]
    fn gradient_at_endpoints() {
        let g = Gradient::new(&[Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)], 10);
        let start = g.at(0.0);
        let end = g.at(1.0);
        assert_eq!(start.r, 0);
        assert_eq!(end.r, 255);
    }

    #[test]
    fn gradient_single_stop() {
        let g = Gradient::new(&[Rgb::new(42, 42, 42)], 5);
        assert_eq!(g.len(), 1);
        assert_eq!(g.at(0.5).r, 42);
    }

    #[test]
    fn gradient_color_at_coord_vertical() {
        let g = Gradient::new(&[Rgb::new(0, 0, 0), Rgb::new(255, 0, 0)], 10);
        // row 0 of 4 => t = 1.0 (top maps to 1.0 - 0/4)
        // row 3 of 4 => t = 0.25
        let top = g.color_at_coord(0, 0, 4, 4, GradientDirection::Vertical);
        let bot = g.color_at_coord(3, 0, 4, 4, GradientDirection::Vertical);
        assert!(top.r > bot.r, "top should be brighter in vertical gradient");
    }

    #[test]
    fn gradient_color_at_coord_horizontal() {
        let g = Gradient::new(&[Rgb::new(0, 0, 0), Rgb::new(255, 0, 0)], 10);
        let left = g.color_at_coord(0, 0, 4, 4, GradientDirection::Horizontal);
        let right = g.color_at_coord(0, 3, 4, 4, GradientDirection::Horizontal);
        assert!(right.r > left.r, "right should be brighter in horizontal gradient");
    }
}

/// Predefined color palettes matching TTE defaults
pub mod palettes {
    use super::Rgb;

    pub fn matrix_rain() -> Vec<Rgb> {
        vec![Rgb::from_hex("92be92"), Rgb::from_hex("185318")]
    }
    pub fn matrix_highlight() -> Rgb {
        Rgb::from_hex("dbffdb")
    }
    pub fn fire() -> Vec<Rgb> {
        vec![
            Rgb::new(255, 255, 255),
            Rgb::new(255, 255, 0),
            Rgb::new(255, 165, 0),
            Rgb::new(200, 50, 0),
            Rgb::new(40, 40, 40),
        ]
    }
    pub fn decrypt_cipher() -> Vec<Rgb> {
        vec![
            Rgb::from_hex("008000"),
            Rgb::from_hex("00cb00"),
            Rgb::from_hex("00ff00"),
        ]
    }
    pub fn decrypt_final() -> Rgb {
        Rgb::from_hex("eda000")
    }
    pub fn default_final() -> Vec<Rgb> {
        vec![Rgb::from_hex("8A008A"), Rgb::from_hex("00D1FF"), Rgb::from_hex("FFFFFF")]
    }
    pub fn purple_cyan_white() -> Vec<Rgb> {
        vec![Rgb::from_hex("8A008A"), Rgb::from_hex("00D1FF"), Rgb::from_hex("FFFFFF")]
    }
    pub fn rainbow() -> Vec<Rgb> {
        vec![
            Rgb::new(255, 0, 0),
            Rgb::new(255, 165, 0),
            Rgb::new(255, 255, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
            Rgb::new(75, 0, 130),
            Rgb::new(238, 130, 238),
        ]
    }
    pub fn star_colors() -> Vec<Rgb> {
        vec![
            Rgb::from_hex("ffcc0d"),
            Rgb::from_hex("ff7326"),
            Rgb::from_hex("ff194d"),
            Rgb::from_hex("bf2669"),
            Rgb::from_hex("702a8c"),
            Rgb::new(255, 255, 255),
        ]
    }
    pub fn lightning() -> Rgb {
        Rgb::from_hex("68A3E8")
    }
    pub fn error_red() -> Rgb {
        Rgb::from_hex("e74c3c")
    }
    pub fn correct_green() -> Rgb {
        Rgb::from_hex("45bf55")
    }
}
