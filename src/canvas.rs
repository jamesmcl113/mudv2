use core::panic;
use std::{error::Error, fmt::Display};

use crossterm::{
    style::{Attribute, Color},
    QueueableCommand,
};

use crate::Result;

#[derive(Clone, Copy, PartialEq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            fg: None,
            bg: None,
            bold: false,
            italic: false,
        }
    }
}

#[derive(Clone, PartialEq)]
struct Cell {
    ch: char,
    style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            style: Style::default(),
        }
    }
}

struct BufferChange<'a> {
    cell: &'a Cell,
    x: usize,
    y: usize,
}

// TODO:
// maybe change this to use Vec<String> for rows.
// this would allow unicode graphemes but it'll be more complex.
#[derive(Clone)]
pub struct RenderBuffer {
    data: Vec<Cell>,
    width: usize,
    height: usize,
}

impl Display for RenderBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for j in 0..self.height {
            let line = &self.data[j * self.width..j * self.width + self.width];
            let line: String = line.iter().map(|cell| cell.ch).collect();
            write!(f, "{}\n", line)?;
        }

        Ok(())
    }
}

impl RenderBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        RenderBuffer {
            data: vec![Cell::default(); width * height],
            width,
            height,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    fn coord_to_idx(&self, x: usize, y: usize) -> usize {
        if x >= self.width || y >= self.height {
            panic!(
                "Coord ({x}, {y}) outside bounds: w = {}, h = {}",
                self.width, self.height
            )
        }

        y * self.width + x
    }

    /// Draw bordered rect with top left at (`x`, `y`).
    pub fn draw_border(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        style: Option<&Style>,
    ) -> Result<()> {
        if x + width > self.width {
            return Err("Bordered area width exceeds buffer width".into());
        }

        if y + height > self.height {
            return Err("Bordered area height exceeds buffer height".into());
        }

        for i in x + 1..x + width - 1 {
            self.set_char('─', style, i, y)?;
            self.set_char('─', style, i, y + height - 1)?;
        }

        for j in y + 1..y + height - 1 {
            self.set_char('│', style, x, j)?;
            self.set_char('│', style, x + width - 1, j)?;
        }

        self.set_char('┌', style, x, y)?;
        self.set_char('┐', style, x + width - 1, y)?;
        self.set_char('┘', style, x + width - 1, y + height - 1)?;
        self.set_char('└', style, x, y + height - 1)?;

        Ok(())
    }

    pub fn clear(&mut self) {
        self.data = vec![Cell::default(); self.width * self.height];
    }

    pub fn set_char(&mut self, ch: char, style: Option<&Style>, x: usize, y: usize) -> Result<()> {
        let idx = self.coord_to_idx(x, y);
        let cell_to_change = self.data.get_mut(idx).ok_or("Coords out of range.")?;

        *cell_to_change = Cell {
            style: style.unwrap_or(&Style::default()).clone(),
            ch,
        };

        Ok(())
    }

    pub fn set_text(
        &mut self,
        text: &str,
        style: Option<&Style>,
        x: usize,
        y: usize,
    ) -> Result<()> {
        if x + text.chars().count() > self.width {
            return Err(format!(
                "Text: '{text}' is too long for canvas. {x} + {} exceeds width: {}",
                text.len(),
                self.width
            )
            .into());
        }

        for (i, ch) in text.chars().enumerate() {
            self.set_char(ch, style, x + i, y)?;
        }

        Ok(())
    }

    pub fn cell_at(&self, x: usize, y: usize) -> &Cell {
        let idx = self.coord_to_idx(x, y);
        &self.data.get(idx).unwrap()
    }

    fn diff(&self, other: &Self) -> Vec<BufferChange<'_>> {
        let mut changes = vec![];

        for y in 0..self.height {
            for x in 0..self.width {
                let current_cell = self.cell_at(x, y);
                if current_cell != other.cell_at(x, y) {
                    changes.push(BufferChange {
                        cell: current_cell,
                        x,
                        y,
                    });
                }
            }
        }

        changes
    }
}

pub struct Canvas {
    buffer: RenderBuffer,
    width: usize,
    height: usize,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Canvas {
            buffer: RenderBuffer::new(width, height),
            width,
            height,
        }
    }

    pub fn redraw<F, W>(&mut self, writer: &mut W, f: F) -> Result<()>
    where
        F: Fn(&mut RenderBuffer) -> Result<()>,
        W: std::io::Write,
    {
        let old_buffer = self.buffer.clone();
        f(&mut self.buffer)?;

        let diff = self.buffer.diff(&old_buffer);

        for BufferChange { cell, x, y } in diff {
            writer
                .queue(crossterm::cursor::MoveTo(x as u16, y as u16))?
                .queue(crossterm::style::SetForegroundColor(
                    cell.style.fg.unwrap_or(Color::Reset),
                ))?
                .queue(crossterm::style::SetBackgroundColor(
                    cell.style.bg.unwrap_or(Color::Reset),
                ))?
                .queue(crossterm::style::SetAttribute(if cell.style.bold {
                    Attribute::Bold
                } else {
                    Attribute::NormalIntensity
                }))?
                .queue(crossterm::style::Print(cell.ch))?;
        }

        writer.flush()?;

        Ok(())
    }
}

pub fn restore_screen() -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    buf.queue(crossterm::terminal::LeaveAlternateScreen)?
        .queue(crossterm::cursor::Show)?;

    Ok(buf)
}

pub fn clear_screen() -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    buf.queue(crossterm::terminal::EnterAlternateScreen)?
        .queue(crossterm::cursor::Hide)?
        .queue(crossterm::cursor::MoveTo(0, 0))?;

    Ok(buf)
}

/*
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn redraw() {
        let mut canvas = Canvas::new(20, 20);
        let mut buf: Vec<u8> = Vec::new();

        canvas
            .redraw(&mut buf, |ctx| {
                ctx.set_char('A', 0, 0)?;
                ctx.set_text("Hello!", 0, 1)?;
                Ok(())
            })
            .unwrap();

        // remember that ASNI escape codes for moving the cursor are 1-based.
        assert_eq!(&buf[..7], b"\x1b[1;1HA");
        assert_eq!(
            &buf[7..],
            b"\x1b[2;1HH\x1b[2;2He\x1b[2;3Hl\x1b[2;4Hl\x1b[2;5Ho\x1b[2;6H!"
        );
    }

    #[test]
    fn buffer_set_text() {
        let mut rb = RenderBuffer::new(10, 10);

        assert!(rb.set_text(&"@".repeat(10), 0, 0).is_ok());
        assert!(rb.set_text(&"N".repeat(11), 0, 0).is_err());
    }

    #[test]
    fn buffer_diff() {
        let mut buffer = RenderBuffer::new(3, 3);
        let old_buffer = buffer.clone();

        buffer.set_text("ABC", 0, 0).unwrap();

        let diff = buffer.diff(&old_buffer);

        assert!(diff.len() == 3);

        assert!(diff[0].ch == &'A');
        assert!(diff[1].ch == &'B');
        assert!(diff[2].ch == &'C');

        assert!(diff[0].x == 0);
        assert!(diff[1].x == 1);
        assert!(diff[2].x == 2);

        assert!(diff[0].y == 0);
        assert!(diff[1].y == 0);
        assert!(diff[2].y == 0);
    }

    #[test]
    fn buffer_coords() {
        let mut rb = RenderBuffer::new(10, 10);

        let res = std::panic::catch_unwind(move || rb.set_char('A', 3, 20));
        assert!(res.is_err());
    }
}
*/
