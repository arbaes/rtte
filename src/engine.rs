use crossterm::{
    cursor, execute, queue,
    style::{self, Color, ResetColor, SetForegroundColor},
    terminal,
};
use std::io::{self, BufWriter, IsTerminal, Write};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthChar;

/// A single cell in the grid
#[derive(Clone)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<Color>,
    pub visible: bool,
}

impl Cell {
    pub fn new(ch: char) -> Self {
        Self {
            ch,
            fg: None,
            visible: false,
        }
    }
}

/// The rendering grid
pub struct Grid {
    pub cells: Vec<Vec<Cell>>,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    pub fn from_input(input: &str) -> Self {
        // Strip ANSI escape sequences from input
        let stripped = strip_ansi(input);
        let lines: Vec<&str> = stripped.lines().collect();
        let height = lines.len();
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

        let mut cells = Vec::with_capacity(height);
        for line in &lines {
            let mut row = Vec::with_capacity(width);
            for ch in line.chars() {
                row.push(Cell::new(ch));
            }
            // Pad to width
            while row.len() < width {
                row.push(Cell::new(' '));
            }
            cells.push(row);
        }

        Grid {
            cells,
            width,
            height,
        }
    }

    pub fn all_visible(&self) -> bool {
        self.cells.iter().all(|row| row.iter().all(|c| c.visible))
    }

    pub fn set_all_visible(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                cell.visible = true;
                cell.fg = None;
            }
        }
    }

    pub fn set_all_invisible(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                cell.visible = false;
                cell.fg = None;
            }
        }
    }

    /// Get all non-space character positions
    pub fn char_positions(&self) -> Vec<(usize, usize)> {
        let mut pos = Vec::new();
        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if cell.ch != ' ' {
                    pos.push((y, x));
                }
            }
        }
        pos
    }

    /// Get all character positions including spaces
    pub fn all_positions(&self) -> Vec<(usize, usize)> {
        let mut pos = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                pos.push((y, x));
            }
        }
        pos
    }
}

/// Strip ANSI escape sequences from a string
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip ESC [ ... (final byte 0x40-0x7E)
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii() && (0x40..=0x7E).contains(&(c as u8)) {
                        break;
                    }
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Render a single frame, repositioning cursor to `origin_row`
/// Uses synchronized output to prevent flicker — the terminal holds
/// all updates until the end marker, then paints in one pass.
pub fn render_frame(
    grid: &Grid,
    out: &mut BufWriter<io::Stdout>,
    origin_row: u16,
    term_width: u16,
) {
    // Begin synchronized update (DEC private mode 2026)
    // Terminals that support this will buffer all output until the end marker
    out.write_all(b"\x1b[?2026h").ok();

    queue!(out, cursor::MoveTo(0, origin_row)).ok();

    let mut last_fg: Option<Color> = Some(Color::Reset); // sentinel to force first set

    for (i, row) in grid.cells.iter().enumerate() {
        let mut col = 0u16;
        for cell in row {
            let w = cell.ch.width().unwrap_or(1);

            if cell.visible {
                if cell.fg != last_fg {
                    if let Some(fg) = cell.fg {
                        queue!(out, SetForegroundColor(fg)).ok();
                    } else if last_fg.is_some() {
                        queue!(out, ResetColor).ok();
                    }
                    last_fg = cell.fg;
                }
                queue!(out, style::Print(cell.ch)).ok();
            } else {
                if last_fg.is_some() && last_fg != Some(Color::Reset) {
                    queue!(out, ResetColor).ok();
                    last_fg = None;
                }
                queue!(out, style::Print(' ')).ok();
            }
            col += w as u16;
        }
        // Pad remainder of line with spaces to overwrite any stale content
        while col < term_width {
            queue!(out, style::Print(' ')).ok();
            col += 1;
        }
        if i < grid.cells.len() - 1 {
            queue!(out, style::Print('\n')).ok();
        }
    }

    if last_fg.is_some() && last_fg != Some(Color::Reset) {
        queue!(out, ResetColor).ok();
    }

    // End synchronized update
    out.write_all(b"\x1b[?2026l").ok();

    out.flush().ok();
}

/// Run the animation loop
pub fn run_animation<F>(grid: &mut Grid, frame_rate: u32, mut tick: F)
where
    F: FnMut(&mut Grid, usize) -> bool, // returns true when done
{
    let mut stdout = BufWriter::with_capacity(64 * 1024, io::stdout());

    // Save cursor position — only query if both stdin and stdout are terminals
    // (DSR sends escape to stdout but reads response from stdin, blocks on pipes)
    let origin_row = if io::stdin().is_terminal() && io::stdout().is_terminal() {
        cursor::position().map(|(_, y)| y).unwrap_or(0)
    } else {
        0
    };

    execute!(stdout, cursor::Hide).ok();

    let term_width = terminal::size().map(|(w, _)| w).unwrap_or(80);
    let frame_duration = Duration::from_micros(1_000_000 / frame_rate as u64);
    let mut frame = 0;

    loop {
        let start = Instant::now();

        let done = tick(grid, frame);
        render_frame(grid, &mut stdout, origin_row, term_width);

        if done {
            break;
        }

        frame += 1;

        let elapsed = start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    // Final frame — the effect already set final colors, just ensure visibility
    for row in &mut grid.cells {
        for cell in row {
            cell.visible = true;
        }
    }
    render_frame(grid, &mut stdout, origin_row, term_width);

    // Move cursor below the grid
    queue!(stdout, cursor::MoveTo(0, origin_row + grid.height as u16)).ok();
    execute!(stdout, cursor::Show).ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_from_single_line() {
        let g = Grid::from_input("hello");
        assert_eq!(g.width, 5);
        assert_eq!(g.height, 1);
        assert_eq!(g.cells[0][0].ch, 'h');
        assert_eq!(g.cells[0][4].ch, 'o');
    }

    #[test]
    fn grid_from_multiline_pads_to_width() {
        let g = Grid::from_input("ab\nxyz");
        assert_eq!(g.width, 3);
        assert_eq!(g.height, 2);
        // short row padded with spaces
        assert_eq!(g.cells[0][2].ch, ' ');
    }

    #[test]
    fn grid_cells_start_invisible() {
        let g = Grid::from_input("hi");
        assert!(g.cells[0].iter().all(|c| !c.visible));
    }

    #[test]
    fn grid_set_all_visible() {
        let mut g = Grid::from_input("hi");
        g.set_all_visible();
        assert!(g.all_visible());
    }

    #[test]
    fn grid_set_all_invisible() {
        let mut g = Grid::from_input("hi");
        g.set_all_visible();
        g.set_all_invisible();
        assert!(!g.all_visible());
    }

    #[test]
    fn grid_char_positions_skips_spaces() {
        let g = Grid::from_input("a b");
        let pos = g.char_positions();
        assert_eq!(pos.len(), 2);
        assert!(pos.contains(&(0, 0)));
        assert!(pos.contains(&(0, 2)));
    }

    #[test]
    fn grid_strips_ansi_escapes() {
        let input = "\x1b[32mgreen\x1b[0m";
        let g = Grid::from_input(input);
        assert_eq!(g.width, 5);
        assert_eq!(g.cells[0][0].ch, 'g');
        assert_eq!(g.cells[0][4].ch, 'n');
    }

    #[test]
    fn grid_from_empty_input_is_empty() {
        let g = Grid::from_input("");
        assert_eq!(g.height, 0);
        assert_eq!(g.width, 0);
    }
}
