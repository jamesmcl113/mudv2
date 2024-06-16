use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
};

pub struct TelnetBackend {
    backend: CrosstermBackend<Vec<u8>>,
    size: Rect,
}

impl TelnetBackend {
    pub fn new(size: Rect) -> Self {
        TelnetBackend {
            backend: CrosstermBackend::new(Vec::new()),
            size,
        }
    }
}

impl Backend for TelnetBackend {
    fn draw<'a, I>(&mut self, content: I) -> std::io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::prelude::buffer::Cell)>,
    {
        self.backend.draw(content)
    }

    fn hide_cursor(&mut self) -> std::io::Result<()> {
        self.backend.hide_cursor()
    }

    fn show_cursor(&mut self) -> std::io::Result<()> {
        self.backend.show_cursor()
    }

    fn get_cursor(&mut self) -> std::io::Result<(u16, u16)> {
        self.backend.get_cursor()
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> std::io::Result<()> {
        self.backend.set_cursor(x, y)
    }

    fn clear(&mut self) -> std::io::Result<()> {
        self.backend.clear()
    }

    fn size(&self) -> std::io::Result<ratatui::prelude::Rect> {
        Ok(self.size)
    }

    fn window_size(&mut self) -> std::io::Result<ratatui::prelude::backend::WindowSize> {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.backend.flush()
    }
}
