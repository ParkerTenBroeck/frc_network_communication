use std::sync::{Arc, RwLock};

use crossterm::{
    event::{Event, MouseButton, MouseEvent, MouseEventKind},
    style::{Attribute, Attributes, Color},
};

use crate::symbols::line::{BOTTOM_LEFT, BOTTOM_RIGHT, HORIZONTAL, TOP_LEFT, TOP_RIGHT, VERTICAL};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn expand_to_include(&mut self, other: &Self) {
        let sx2 = self.x.saturating_add(self.width);
        let sy2 = self.y.saturating_add(self.height);

        let ox2 = other.x.saturating_add(other.width);
        let oy2 = other.y.saturating_add(other.height);

        self.x = self.x.min(other.x);
        self.y = self.y.min(other.y);

        let sx2 = sx2.max(ox2);
        let sy2 = sy2.max(oy2);

        self.width = sx2 - self.x;
        self.height = sy2 - self.y;
    }

    pub fn overlap(&self, other: &Self) -> bool {
        if self.height == 0 || self.width == 0 || other.width == 0 || other.height == 0 {
            return false;
        }
        let sx2 = self.x.saturating_add(self.width);
        let sy2 = self.y.saturating_add(self.height);

        let ox2 = other.x.saturating_add(other.width);
        let oy2 = other.y.saturating_add(other.height);

        if self.x > ox2 || other.x > sx2 {
            return false;
        }

        if sy2 > other.y || oy2 > self.y {
            return false;
        }

        true
    }

    pub fn contains(&self, column: u16, row: u16) -> bool {
        self.x <= column
            && (self.x.saturating_add(self.width)) > column
            && self.y <= row
            && (self.y.saturating_add(self.height)) > row
    }

    pub fn top_left(&self) -> Pos2 {
        Pos2 {
            x: self.x,
            y: self.y,
        }
    }

    pub fn top_right(&self) -> Pos2 {
        Pos2 {
            x: self.x.saturating_add(self.width).saturating_sub(1),
            y: self.y,
        }
    }

    pub fn bottom_left(&self) -> Pos2 {
        Pos2 {
            x: self.x,
            y: self.y.saturating_add(self.height).saturating_sub(1),
        }
    }

    pub fn bottom_right(&self) -> Pos2 {
        Pos2 {
            x: self.x.saturating_add(self.width).saturating_sub(1),
            y: self.y.saturating_add(self.height).saturating_sub(1),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Pos2 {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug)]
pub enum Draw {
    ClearAll(Style),
    Clear(Style, Rect),
    Text(StyledText, Pos2),
}

#[derive(Debug, Default)]
struct ContextInner {
    pub event: Option<Event>,
    pub draws: Vec<Draw>,
    pub max_rect: Rect,
}

#[derive(Clone, Default)]
pub struct Context {
    inner: Arc<RwLock<ContextInner>>,
}

impl Context {
    pub fn frame(&self, func: impl FnOnce(&mut Ui)) {
        let lock = self.inner.read().unwrap();
        let mut ui = Ui {
            clip: lock.max_rect,
            mix_rect: Default::default(),
            layout: Layout::Vertical,
            max_rect: lock.max_rect,
            cursor: {
                drop(lock);
                Default::default()
            },
            context: (*self).clone(),
            current: Default::default(),
        };
        func(&mut ui);
    }

    pub fn take_draw_commands(&mut self, vec: &mut Vec<Draw>) {
        vec.append(&mut self.inner.write().unwrap().draws);
    }

    pub fn new_event(&self, event: Event) {
        self.inner.write().unwrap().event = Some(event)
    }

    pub fn get_event(&self) -> Option<Event> {
        self.inner.read().unwrap().event.clone()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layout {
    Vertical,
    Horizontal,
}

#[derive(Clone)]
pub struct Ui {
    context: Context,
    layout: Layout,
    clip: Rect,
    mix_rect: Rect,
    max_rect: Rect,
    cursor: Rect,
    current: Rect,
}

impl Ui {
    pub fn label(&mut self, text: impl Into<StyledText>) {
        let (_, gallery) = self.create_gallery(text.into());
        self.draw_gallery(gallery)
    }

    pub fn ctx(&self) -> &Context {
        &self.context
    }

    fn child(&self) -> Ui {
        let mut ui = self.clone();
        ui.current = ui.cursor;
        ui.current.width = 0;
        ui.current.height = 0;
        ui.clip.x = ui.cursor.x;
        ui.clip.y = ui.cursor.y;
        ui
    }

    pub fn bordered(&mut self, func: impl FnOnce(&mut Ui)) {
        let mut child = self.child();
        // child.clip.x += 1;
        // child.clip.y += 1;
        // child.clip.width -= 1;
        // child.clip.height -= 1;

        child.cursor.x += 1;
        child.cursor.y += 1;

        child.current.width = 0;
        child.current.height = 0;
        child.current.x = child.cursor.x;
        child.current.y = child.cursor.y;

        func(&mut child);


        let mut lock = self.context.inner.write().unwrap();

        child.current.x -= 1;
        child.current.y -= 1;
        child.current.height += 2;
        child.current.width += 2;
        let border = child.current;

        lock.draws.push(Draw::Text(
            StyledText {
                text: TOP_LEFT.into(),
                style: Style::default(),
            },
            border.top_left(),
        ));
        for i in 0..(border.width - 2) {
            lock.draws.push(Draw::Text(
                StyledText {
                    text: HORIZONTAL.into(),
                    style: Style::default(),
                },
                Pos2 {
                    x: border.x + 1 + i,
                    y: border.y,
                },
            ));
        }

        lock.draws.push(Draw::Text(
            StyledText {
                text: TOP_RIGHT.into(),
                style: Style::default(),
            },
            border.top_right(),
        ));

        lock.draws.push(Draw::Text(
            StyledText {
                text: BOTTOM_LEFT.into(),
                style: Style::default(),
            },
            border.bottom_left(),
        ));
        for i in 0..(border.width - 2) {
            lock.draws.push(Draw::Text(
                StyledText {
                    text: HORIZONTAL.into(),
                    style: Style::default(),
                },
                Pos2 {
                    x: border.x + 1 + i,
                    y: border.bottom_right().y,
                },
            ));
        }

        lock.draws.push(Draw::Text(
            StyledText {
                text: BOTTOM_RIGHT.into(),
                style: Style::default(),
            },
            border.bottom_right(),
        ));

        for i in 0..(border.height - 2) {
            lock.draws.push(Draw::Text(
                StyledText {
                    text: VERTICAL.into(),
                    style: Style::default(),
                },
                Pos2 {
                    x: border.x,
                    y: border.y + 1 + i,
                },
            ));
            lock.draws.push(Draw::Text(
                StyledText {
                    text: VERTICAL.into(),
                    style: Style::default(),
                },
                Pos2 {
                    x: border.bottom_right().x,
                    y: border.y + 1 + i,
                },
            ));
        }
        drop(lock);

        self.allocate_space(border);
    }

    fn allocate_space(&mut self, rect: Rect) -> () {
        self.current.expand_to_include(&rect);

        //TODO: the rect could start at a different position
        match self.layout {
            Layout::Vertical => self.cursor.y += rect.height,
            Layout::Horizontal => self.cursor.x += rect.width + 1,
        }
    }

    pub fn vertical(&mut self, func: impl FnOnce(&mut Ui)) {
        self.layout(Layout::Vertical, func)
    }
    pub fn horizontal(&mut self, func: impl FnOnce(&mut Ui)) {
        self.layout(Layout::Horizontal, func)
    }

    pub fn seperator(&mut self) {
        match self.layout {
            Layout::Vertical => {
                let height = self.current.height;
                let x = self.current.x + self.current.width;
                self.current.width += 1;
                self.cursor.x += 1;
                let mut lock = self.context.inner.write().unwrap();
                for i in 0..height {
                    lock.draws.push(Draw::Text(
                        StyledText {
                            text: VERTICAL.into(),
                            style: Style::default(),
                        },
                        Pos2 {
                            x,
                            y: self.current.y + i,
                        },
                    ));
                }
            }
            Layout::Horizontal => {
                let width = self.current.width;
                let y = self.current.y + self.current.height;
                self.current.height += 1;
                self.cursor.y += 1;
                let mut lock = self.context.inner.write().unwrap();
                for i in 0..width {
                    lock.draws.push(Draw::Text(
                        StyledText {
                            text: HORIZONTAL.into(),
                            style: Style::default(),
                        },
                        Pos2 {
                            x: self.current.x + i,
                            y,
                        },
                    ));
                }
            }
        }
    }

    fn layout(&mut self, layout: Layout, func: impl FnOnce(&mut Ui)) {
        let mut ui = self.clone();
        ui.current = ui.cursor;
        ui.current.width = 0;
        ui.current.height = 0;
        ui.layout = layout;
        func(&mut ui);
        self.allocate_space(ui.current)
    }

    fn draw_gallery(&mut self, gallery: Vec<(Rect, StyledText)>) {
        let mut lock = self.context.inner.write().unwrap();
        lock.draws.reserve(gallery.len());
        for text in gallery {
            lock.draws.push(Draw::Text(text.1, text.0.top_left()));

            match self.layout {
                Layout::Vertical => self.cursor.y = self.cursor.y.max(text.0.y + text.0.height),
                Layout::Horizontal => self.cursor.x = self.cursor.x.max(text.0.x + text.0.width),
            }

            self.current.expand_to_include(&text.0);
        }
        match self.layout {
            Layout::Vertical => {}
            Layout::Horizontal => self.cursor.x += 1,
        }
    }

    pub fn button(&mut self, text: impl Into<StyledText>) -> bool {
        let (rect, mut gallery) = self.create_gallery(text.into());
        let pressed = if let Some(Event::Mouse(MouseEvent {
            kind, column, row, ..
        })) = self.context.inner.read().unwrap().event
        {
            if rect.contains(column, row) {
                match kind {
                    MouseEventKind::Down(_) | MouseEventKind::Drag(MouseButton::Left) => {
                        for item in &mut gallery {
                            item.1.bg(Color::Blue);
                        }
                    }
                    MouseEventKind::Up(_) | MouseEventKind::Moved => {
                        for item in &mut gallery {
                            item.1.underline(true);
                        }
                    }
                    _ => {}
                }

                matches!(kind, MouseEventKind::Down(MouseButton::Left))
            } else {
                false
            }
        } else {
            false
        };
        self.draw_gallery(gallery);
        pressed
    }

    pub fn bordered_frame(&mut self, func: impl FnOnce(&mut Ui)) {}

    pub fn drop_down(&mut self, title: &str, func: impl FnOnce(&mut Ui)) {}

    fn create_gallery(&self, text: StyledText) -> (Rect, Vec<(Rect, StyledText)>) {
        // todo!();
        let mut rect = self.cursor;
        rect.width = 0;
        rect.height = 0;

        let mut gallery = Vec::new();

        for (line_num, line) in text.text.split('\n').enumerate() {
            let mut line_width = 0;
            for char in line.chars() {
                line_width += unicode_width::UnicodeWidthChar::width(char).unwrap_or(0) as u16;
            }
            gallery.push((
                Rect {
                    x: rect.x,
                    y: rect.y + line_num as u16,
                    width: line_width,
                    height: 1,
                },
                StyledText {
                    text: line.to_owned(),
                    style: text.style,
                },
            ));
            rect.height += 1;
            rect.width = rect.width.max(line_width);
        }

        (rect, gallery)
    }

    pub fn add_horizontal_space(&mut self, space: u16) {
        self.cursor.x += space;
        self.clip.x += space;
        self.clip.width -= space;
        self.current.width = self.current.width.max(space);
        // self.
    }
}

#[derive(Clone, Debug, Default)]
pub struct StyledText {
    pub text: String,
    pub style: Style,
}

impl StyledText {
    pub fn new(text: impl Into<String>) -> Self {
        text.into().into()
    }

    pub fn fg(&mut self, color: Color) {
        self.style.fg = color;
    }

    pub fn bg(&mut self, color: Color) {
        self.style.bg = color;
    }

    pub fn modifiers(&mut self, attributes: Attributes) {
        self.style.attributes = attributes;
    }

    pub fn underline(&mut self, show: bool) {
        if show {
            self.style.attributes.set(Attribute::Underlined);
        } else {
            self.style.attributes.unset(Attribute::Underlined);
        }
    }

    pub fn bold(&mut self, show: bool) {
        if show {
            self.style.attributes.set(Attribute::Bold);
        } else {
            self.style.attributes.unset(Attribute::Bold);
        }
    }

    pub fn slow_blink(&mut self, show: bool) {
        if show {
            self.style.attributes.set(Attribute::SlowBlink);
        } else {
            self.style.attributes.unset(Attribute::SlowBlink);
        }
    }

    pub fn rapid_blink(&mut self, show: bool) {
        if show {
            self.style.attributes.set(Attribute::RapidBlink);
        } else {
            self.style.attributes.unset(Attribute::RapidBlink);
        }
    }

    pub fn italic(&mut self, show: bool) {
        if show {
            self.style.attributes.set(Attribute::Italic);
        } else {
            self.style.attributes.unset(Attribute::Italic);
        }
    }

    pub fn dim(&mut self, show: bool) {
        if show {
            self.style.attributes.set(Attribute::Dim);
        } else {
            self.style.attributes.unset(Attribute::Dim);
        }
    }

    pub fn crossed_out(&mut self, show: bool) {
        if show {
            self.style.attributes.set(Attribute::CrossedOut);
        } else {
            self.style.attributes.unset(Attribute::CrossedOut);
        }
    }

    pub fn hidden(&mut self, show: bool) {
        if show {
            self.style.attributes.set(Attribute::Hidden);
        } else {
            self.style.attributes.unset(Attribute::Hidden);
        }
    }

    pub fn reversed(&mut self, show: bool) {
        if show {
            self.style.attributes.set(Attribute::Reverse);
        } else {
            self.style.attributes.unset(Attribute::Reverse);
        }
    }

    pub fn styled(text: String, style: Style) -> Self {
        Self { text, style }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attributes: Attributes,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fg: Color::Reset,
            bg: Color::Reset,
            attributes: Attributes::default(),
        }
    }
}

impl From<&str> for StyledText {
    fn from(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            ..Default::default()
        }
    }
}

impl From<String> for StyledText {
    fn from(text: String) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }
}
