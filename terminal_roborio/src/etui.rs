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
    pub fn add_top_left(&mut self, translation: VecI2) {
        self.x = self.x.saturating_add(translation.x);
        self.y = self.y.saturating_add(translation.y);

        self.width = self.width.saturating_sub(translation.x);
        self.height = self.height.saturating_sub(translation.y);
    }

    pub fn sub_top_left(&mut self, translation: VecI2) {
        self.x = self.x.saturating_sub(translation.x);
        self.y = self.y.saturating_sub(translation.y);

        self.width = self.width.saturating_add(translation.x);
        self.height = self.height.saturating_add(translation.y);
    }

    pub fn add_bottom_right(&mut self, translation: VecI2) {
        self.width = self.width.saturating_add(translation.x);
        self.height = self.height.saturating_add(translation.y);
    }

    pub fn sub_bottom_right(&mut self, translation: VecI2) {
        self.width = self.width.saturating_sub(translation.x);
        self.height = self.height.saturating_sub(translation.y);
    }

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

    pub fn top_left(&self) -> VecI2 {
        VecI2 {
            x: self.x,
            y: self.y,
        }
    }

    pub fn top_right(&self) -> VecI2 {
        VecI2 {
            x: self.x.saturating_add(self.width),
            y: self.y,
        }
    }

    pub fn top_right_inner(&self) -> VecI2 {
        VecI2 {
            x: self.x.saturating_add(self.width).saturating_sub(1),
            y: self.y,
        }
    }

    pub fn bottom_left(&self) -> VecI2 {
        VecI2 {
            x: self.x,
            y: self.y.saturating_add(self.height),
        }
    }

    pub fn bottom_left_inner(&self) -> VecI2 {
        VecI2 {
            x: self.x,
            y: self.y.saturating_add(self.height).saturating_sub(1),
        }
    }

    pub fn bottom_right(&self) -> VecI2 {
        VecI2 {
            x: self.x.saturating_add(self.width),
            y: self.y.saturating_add(self.height),
        }
    }

    pub fn bottom_right_inner(&self) -> VecI2 {
        VecI2 {
            x: self.x.saturating_add(self.width).saturating_sub(1),
            y: self.y.saturating_add(self.height).saturating_sub(1),
        }
    }

    pub fn new_pos_size(pos: VecI2, size: VecI2) -> Rect {
        Self {
            x: pos.x,
            y: pos.y,
            width: size.x,
            height: size.y,
        }
    }

    pub fn new_pos_pos(top_left: VecI2, bottom_right: VecI2) -> Rect {
        let width = bottom_right.x.saturating_sub(top_left.x);
        let height = bottom_right.y.saturating_sub(top_left.y);

        Self {
            x: top_left.x,
            y: top_left.y,
            width,
            height,
        }
    }

    pub fn move_top_left_to(&mut self, cursor: VecI2) {
        let bottom_right = self.bottom_right();
        self.x = cursor.x;
        self.y = cursor.y;
        self.width = bottom_right.x.saturating_sub(cursor.x);
        self.height = bottom_right.y.saturating_sub(cursor.y);
    }

    pub fn size(&self) -> VecI2{
        VecI2 { x: self.width, y: self.height }
    }

    pub fn expand_evenly(&mut self, ammount: u16) {
        self.x = self.x.saturating_sub(ammount);
        self.y = self.y.saturating_sub(ammount);

        self.width = self.width.saturating_add(ammount);
        self.width = self.width.saturating_add(ammount);

        self.height = self.height.saturating_add(ammount);
        self.height = self.height.saturating_add(ammount);
    }

    pub fn shrink_evenly(&mut self, ammount: u16) {
        self.x = self.x.saturating_add(ammount);
        self.y = self.y.saturating_add(ammount);

        self.width = self.width.saturating_sub(ammount);
        self.width = self.width.saturating_sub(ammount);

        self.height = self.height.saturating_sub(ammount);
        self.height = self.height.saturating_sub(ammount);
    }

    fn shrink_to_fit_within(&mut self, max_rect: Rect) {
        let mut bottom_left = self.bottom_right();
        let max_bottom_left = max_rect.bottom_right();

        self.x = self.x.max(max_rect.x);
        self.y = self.y.max(max_rect.y);

        bottom_left.x = bottom_left.x.min(max_bottom_left.x);
        bottom_left.y = bottom_left.y.min(max_bottom_left.y);

        self.width = bottom_left.x.saturating_sub(self.x);
        self.height = bottom_left.y.saturating_sub(self.y);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct VecI2 {
    pub x: u16,
    pub y: u16,
}

impl VecI2 {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

impl std::ops::Add for VecI2 {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.x = self.x.saturating_add(rhs.x);
        self.y = self.y.saturating_add(rhs.y);
        self
    }
}

impl std::ops::AddAssign for VecI2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x = self.x.saturating_add(rhs.x);
        self.y = self.y.saturating_add(rhs.y);
    }
}

impl std::ops::SubAssign for VecI2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x = self.x.saturating_sub(rhs.x);
        self.y = self.y.saturating_sub(rhs.y);
    }
}

impl std::ops::Sub for VecI2 {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self.x = self.x.saturating_sub(rhs.x);
        self.y = self.y.saturating_sub(rhs.y);
        self
    }
}

#[derive(Debug)]
pub enum Draw {
    ClearAll(Style),
    Clear(Style, Rect),
    Text(StyledText, VecI2),
}

#[derive(Debug, Default)]
struct ContextInner {
    pub event: Option<Event>,
    pub draws: Vec<Draw>,
    pub max_rect: Rect,
}
impl ContextInner {
    fn new(size: Rect) -> ContextInner {
        Self {
            max_rect: size,
            ..Default::default()
        }
    }
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
            layout: Layout::TopDown,
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
        match event{
            Event::Resize(x, y) => {
                self.inner.write().unwrap().max_rect = Rect::new_pos_size(VecI2::new(0,0), VecI2::new(x,y))
            },
            _ => {}
        }
        self.inner.write().unwrap().event = Some(event)
    }

    pub fn get_event(&self) -> Option<Event> {
        self.inner.read().unwrap().event.clone()
    }

    pub fn new(size: Rect) -> Context {
        Self {
            inner: Arc::new(RwLock::new(ContextInner::new(size))),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layout {
    TopDown,
    DownTop,
    LeftRight,
    RightLeft,
}

#[derive(Clone)]
pub struct Ui {
    context: Context,
    layout: Layout,
    clip: Rect,
    max_rect: Rect,
    cursor: VecI2,
    current: Rect,
}

impl Ui {
    pub fn label(&mut self, text: impl Into<StyledText>) {
        let (_, gallery) = self.create_gallery(text.into());
        self.draw_gallery(gallery)
    }

    pub fn get_clip(&self) -> Rect {
        self.clip
    }

    pub fn get_max(&self) -> Rect {
        self.max_rect
    }

    pub fn get_cursor(&self) -> VecI2 {
        self.cursor
    }

    pub fn get_current(&self) -> Rect {
        self.current
    }

    pub fn ctx(&self) -> &Context {
        &self.context
    }

    fn child(&self) -> Ui {
        let mut ui = self.clone();
        ui.current = Rect::new_pos_size(ui.cursor, VecI2::new(0, 0));
        ui.clip.move_top_left_to(ui.cursor);
        ui
    }

    pub fn with_size(&mut self, size: VecI2, func: impl FnOnce(&mut Ui)) {
        let size = self.allocate_size(size);
        let mut child = self.child();
        child.clip = size;
        child.max_rect = size;
        child.current = size;
        child.cursor = size.top_left();
        func(&mut child)
    }

    pub fn bordered(&mut self, func: impl FnOnce(&mut Ui)) {
        let start_clip = self.clip;
        let start_max_rect = self.max_rect;
        let start = self.cursor;
        
        let mut child = self.child();
        
        child.add_space(VecI2::new(1,1));

        
        child.max_rect = start_max_rect;
        child.max_rect.shrink_evenly(1);
        child.clip = start_clip;
        child.clip.shrink_evenly(1);


        func(&mut child);

        let mut lock = self.context.inner.write().unwrap();

        child.expand(VecI2::new(1,1));
        let mut border = child.current;
        border.expand_to_include(&Rect::new_pos_size(start, VecI2::new(0,0)));

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
                VecI2 {
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
            border.top_right_inner(),
        ));

        lock.draws.push(Draw::Text(
            StyledText {
                text: BOTTOM_LEFT.into(),
                style: Style::default(),
            },
            border.bottom_left_inner(),
        ));
        for i in 0..(border.width - 2) {
            lock.draws.push(Draw::Text(
                StyledText {
                    text: HORIZONTAL.into(),
                    style: Style::default(),
                },
                VecI2 {
                    x: border.x + 1 + i,
                    y: border.bottom_right_inner().y,
                },
            ));
        }

        lock.draws.push(Draw::Text(
            StyledText {
                text: BOTTOM_RIGHT.into(),
                style: Style::default(),
            },
            border.bottom_right_inner(),
        ));

        for i in 0..(border.height - 2) {
            lock.draws.push(Draw::Text(
                StyledText {
                    text: VERTICAL.into(),
                    style: Style::default(),
                },
                VecI2 {
                    x: border.x,
                    y: border.y + 1 + i,
                },
            ));
            lock.draws.push(Draw::Text(
                StyledText {
                    text: VERTICAL.into(),
                    style: Style::default(),
                },
                VecI2 {
                    x: border.bottom_right_inner().x,
                    y: border.y + 1 + i,
                },
            ));
        }
        drop(lock);

        self.allocate_size(border.size());
    }

    fn allocate_area(&mut self, rect: Rect) -> Rect{
        if rect.top_left() == self.cursor{
            self.allocate_size(rect.size())
        }else{
            todo!()
        }
    }

    fn allocate_size(&mut self, desired: VecI2) -> Rect {
        let old_cursor = self.cursor;
        let old_max = self.max_rect;
        self.add_space(desired);
        let new_cursor = self.cursor;

        match self.layout {
            Layout::TopDown => {
                self.cursor.x = old_cursor.x;
                self.max_rect.x = old_max.x;
                self.max_rect.width = old_max.width;
                Rect::new_pos_pos(old_cursor, new_cursor)
            }
            Layout::LeftRight => {
                self.cursor.y = old_cursor.y;
                self.max_rect.y = old_max.y;
                self.max_rect.height = old_max.height;
                Rect::new_pos_pos(old_cursor, new_cursor)
            }
            Layout::DownTop => {
                self.cursor.x = old_cursor.x;
                self.max_rect.x = old_max.x;
                self.max_rect.width = old_max.width;
                Rect::new_pos_pos(VecI2::new(old_cursor.x,new_cursor.y), VecI2::new(new_cursor.x,old_cursor.y))
            }
            Layout::RightLeft => {
                self.cursor.y = old_cursor.y;
                self.max_rect.y = old_max.y;
                self.max_rect.height = old_max.height;
                Rect::new_pos_pos(VecI2::new(new_cursor.x,old_cursor.y), VecI2::new(old_cursor.x,new_cursor.y))
            }
        }
    }

    pub fn vertical(&mut self, func: impl FnOnce(&mut Ui)) {
        self.layout(Layout::TopDown, func)
    }
    pub fn horizontal(&mut self, func: impl FnOnce(&mut Ui)) {
        self.layout(Layout::LeftRight, func)
    }

    fn layout(&mut self, layout: Layout, func: impl FnOnce(&mut Ui)) {
        let mut ui = self.clone();
        ui.current = Rect::new_pos_size(ui.cursor, VecI2::new(0, 0));
        ui.layout = layout;
        func(&mut ui);
        self.allocate_area(ui.current);
    }

    pub fn seperator(&mut self) {
        match self.layout {
            Layout::LeftRight | Layout::RightLeft => {
                let area = self.allocate_size(VecI2::new(1, self.current.height));
 
                let mut lock = self.context.inner.write().unwrap();
                for i in 0..area.height {
                    lock.draws.push(Draw::Text(
                        StyledText {
                            text: VERTICAL.into(),
                            style: Style::default(),
                        },
                        VecI2 {
                            x: area.x,
                            y: self.current.y + i,
                        },
                    ));
                }
            }
            Layout::TopDown | Layout::DownTop => {
                let area = self.allocate_size(VecI2::new(self.current.width, 1));
                let mut lock = self.context.inner.write().unwrap();
                for i in 0..area.width {
                    lock.draws.push(Draw::Text(
                        StyledText {
                            text: HORIZONTAL.into(),
                            style: Style::default(),
                        },
                        VecI2 {
                            x: self.current.x + i,
                            y: area.y,
                        },
                    ));
                }
            }
        }
    }

    fn draw_gallery(&mut self, gallery: Vec<(Rect, StyledText)>) {
        let mut lock = self.context.inner.write().unwrap();
        lock.draws.reserve(gallery.len());
        drop(lock);
        for (bound, text) in gallery {
            self.allocate_area(bound);
            let mut lock = self.context.inner.write().unwrap();
            lock.draws.push(Draw::Text(text, bound.top_left()));
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
        
        let mut rect = Rect::new_pos_size(self.cursor, VecI2::new(0, 0));

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
        self.add_space(VecI2::new(space, 0))
    }

    pub fn add_vertical_space(&mut self, space: u16) {
        self.add_space(VecI2::new(0, space))
    }

    pub fn add_space(&mut self, space: VecI2) {
        match self.layout {
            Layout::LeftRight | Layout::TopDown => {
                self.cursor += space;
                self.clip.move_top_left_to(self.cursor);
                self.max_rect.move_top_left_to(self.cursor);
            }
            Layout::DownTop => {
                self.cursor -= VecI2::new(0, space.y);
                self.cursor += VecI2::new(space.x, 0);
                todo!()
            }
            Layout::RightLeft => {
                self.cursor += VecI2::new(0, space.y);
                self.cursor -= VecI2::new(space.x, 0);
                todo!()
            }
        }
        self.current.expand_to_include(&Rect::new_pos_size(self.cursor, VecI2::new(0,0)));        
    }

    pub fn expand(&mut self, translation: VecI2) {
        match self.layout {
            Layout::LeftRight | Layout::TopDown => {
                self.current.add_bottom_right(translation)
            }
            Layout::DownTop => {
                self.current.add_bottom_right(VecI2::new(translation.x, 0));
                self.current.add_top_left(VecI2::new(0,translation.y))
            }
            Layout::RightLeft => {
                self.current.add_bottom_right(VecI2::new(0, translation.y));
                self.current.add_top_left(VecI2::new(translation.x,0))
            }
        }
    }

    pub fn set_minimum_size(&mut self, mut min: VecI2) {
        min.x = min.x.min(self.max_rect.width);
        min.y = min.y.min(self.max_rect.height);
        match self.layout {
            Layout::LeftRight | Layout::TopDown => {
                self.current.width = self.current.width.max(min.x);
                self.current.height = self.current.height.max(min.y);
            }
            Layout::DownTop => {
                todo!()
            }
            Layout::RightLeft => {
                todo!()
            }
        }
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
