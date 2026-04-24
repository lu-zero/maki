use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::components::Overlay;
use crate::components::keybindings::key;
use crate::components::modal::Modal;

const TITLE: &str = " Usage ";

pub struct InfoModal {
    open: bool,
    lines: Vec<Line<'static>>,
}

impl InfoModal {
    pub fn new() -> Self {
        Self {
            open: false,
            lines: Vec::new(),
        }
    }

    pub fn open(&mut self, content: String) {
        self.lines = content.lines().map(|l| Line::from(l.to_string())).collect();
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.lines.clear();
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) -> bool {
        if key_event.code == KeyCode::Esc
            || key::QUIT.matches(key_event)
            || key::HELP.matches(key_event)
        {
            self.close();
        }
        self.open
    }

    pub fn view(&mut self, frame: &mut Frame, area: Rect) -> Rect {
        if !self.open {
            return Rect::default();
        }

        let total = self.lines.len() as u16;
        let modal = Modal {
            title: TITLE,
            width_percent: 50,
            max_height_percent: 80,
        };
        let (popup, inner) = modal.render(frame, area, total);

        let paragraph = Paragraph::new(self.lines.clone());
        frame.render_widget(paragraph, inner);

        popup
    }
}

impl Overlay for InfoModal {
    fn is_open(&self) -> bool {
        self.open
    }

    fn close(&mut self) {
        self.close();
    }
}
