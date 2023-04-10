use std::sync::{Arc, RwLock};

use crossterm::{
    event::{Event, MouseButton, MouseEvent, MouseEventKind},
    style::{Attribute, Attributes, Color},
};

use self::{
    id::Id,
    math_util::{Rect, VecI2},
    memory::Memory,
    symbols::line::*,
};

pub mod id;
pub mod math_util;
pub mod memory;
pub mod symbols;

#[derive(Debug)]
pub enum Draw {
    ClearAll(Style),
    Clear(Style, Rect),
    Text(StyledText, VecI2),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonState {
    #[default]
    Unpressed,
    Down,
    Held,
    Up,
    Drag,
}

impl MouseButtonState {
    pub fn is_down(&self) -> bool {
        match self {
            MouseButtonState::Unpressed => false,
            MouseButtonState::Down => true,
            MouseButtonState::Held => true,
            MouseButtonState::Up => false,
            MouseButtonState::Drag => true,
        }
    }

    pub fn next_state(&mut self) {
        match self {
            MouseButtonState::Unpressed => {}
            MouseButtonState::Down => *self = MouseButtonState::Held,
            MouseButtonState::Held => {}
            MouseButtonState::Up => *self = MouseButtonState::Unpressed,
            MouseButtonState::Drag => {}
        }
    }

    pub fn is_up(&self) -> bool {
        !self.is_down()
    }
}

#[derive(Debug, Default)]
pub struct MouseState {
    pub position: VecI2,
    pub left: MouseButtonState,
    pub middle: MouseButtonState,
    pub right: MouseButtonState,
    pub scroll: i16,
}

#[derive(Debug, Default)]
struct ContextInner {
    event: Option<Event>,
    mouse: Option<MouseState>,
    draws: Vec<Draw>,
    max_rect: Rect,
    memory: Memory,
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

    pub fn finish_frame(&mut self, vec: &mut Vec<Draw>) {
        let mut lock = self.inner.write().unwrap();

        if let Some(mouse) = &mut lock.mouse {
            mouse.left.next_state();
            mouse.middle.next_state();
            mouse.right.next_state();
        }

        lock.memory.clear_seen();
        vec.append(&mut lock.draws);
    }

    pub fn handle_event(&self, event: Event) {
        let mut lock = self.inner.write().unwrap();
        match event {
            Event::Resize(x, y) => {
                lock.max_rect = Rect::new_pos_size(VecI2::new(0, 0), VecI2::new(x, y))
            }
            Event::Mouse(event) => {
                let mouse = lock.mouse.get_or_insert(MouseState::default());
                mouse.position.x = event.column;
                mouse.position.y = event.row;
                match event.kind {
                    MouseEventKind::Down(button)
                    | MouseEventKind::Up(button)
                    | MouseEventKind::Drag(button) => {
                        let button = match button {
                            MouseButton::Left => &mut mouse.left,
                            MouseButton::Right => &mut mouse.right,
                            MouseButton::Middle => &mut mouse.middle,
                        };
                        match event.kind {
                            MouseEventKind::Down(_) => {
                                *button = MouseButtonState::Down;
                            }
                            MouseEventKind::Up(_) => {
                                *button = MouseButtonState::Up;
                            }
                            MouseEventKind::Drag(_) => {
                                *button = MouseButtonState::Drag;
                            }
                            _ => {}
                        }
                    }
                    MouseEventKind::Moved => {}
                    MouseEventKind::ScrollDown => mouse.scroll -= 1,
                    MouseEventKind::ScrollUp => mouse.scroll += 1,
                }
                // mouse.kind
                // mouse.modifiers
                // lock.last_observed_mouse_pos = Some(VecI2::new(mouse.row, mouse.column));
            }
            _ => {}
        }
        lock.event = Some(event)
    }

    pub fn get_event(&self) -> Option<Event> {
        self.inner.read().unwrap().event.clone()
    }

    pub fn new(size: Rect) -> Context {
        Self {
            inner: Arc::new(RwLock::new(ContextInner::new(size))),
        }
    }

    pub fn clear_event(&self) {
        self.inner.write().unwrap().event = None
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Response{
    clicked: bool,
    pressed: bool,
    hovered: bool,
    released: bool,
    dragged: bool,
}

impl Response{
    pub fn clicked(&self) -> bool{
        self.clicked
    }

    pub fn pressed(&self) -> bool{
        self.pressed
    }

    pub fn hovered(&self) -> bool{
        self.hovered
    }

    pub fn released(&self) -> bool{
        self.released
    }

    fn nothing() -> Response {
        Self::default()
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

    pub fn with_memory_or<T: Clone + 'static, F: FnOnce(T, &mut Self)>(
        &mut self,
        id: Id,
        default: T,
        func: F,
    ) {
        let mut lock = self.context.inner.write().unwrap();
        let res = lock.memory.get_mut_or(id, default);
        drop(lock);

        if let Ok(val) = res {
            func(val, self);
            return;
        }

        let mut style = Style {
            bg: Color::Red,
            fg: Color::White,
            ..Default::default()
        };
        style.attributes.set(Attribute::RapidBlink);
        style.attributes.set(Attribute::Underlined);
        self.label(StyledText::styled(
            format!("IDCOLLISION: {:?}", id.value()),
            style,
        ));
    }

    pub fn with_memory_or_make<T: Clone + 'static, F: FnOnce(T, &mut Self)>(
        &mut self,
        id: Id,
        default: impl FnOnce() -> T,
        func: F,
    ) {
        let mut lock = self.context.inner.write().unwrap();
        let res = lock.memory.get_mut_or(id, default());
        drop(lock);
        if let Ok(val) = res {
            func(val, self);
            return;
        }

        let mut style = Style::default();
        style.bg = Color::Red;
        style.fg = Color::White;
        style.attributes.set(Attribute::RapidBlink);
        style.attributes.set(Attribute::Underlined);
        self.label(StyledText::styled(
            format!("IDCOLLISION: {:?}", id.value()),
            style,
        ));
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

        child.add_space(VecI2::new(1, 1));

        child.max_rect = start_max_rect;
        child.max_rect.shrink_evenly(1);
        child.clip = start_clip;
        child.clip.shrink_evenly(1);

        func(&mut child);

        let mut lock = self.context.inner.write().unwrap();

        child.expand(VecI2::new(1, 1));
        let mut border = child.current;
        border.expand_to_include(&Rect::new_pos_size(start, VecI2::new(0, 0)));

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

    fn allocate_area(&mut self, rect: Rect) -> Rect {
        if rect.top_left() == self.cursor {
            self.allocate_size(rect.size())
        } else {
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
                Rect::new_pos_pos(
                    VecI2::new(old_cursor.x, new_cursor.y),
                    VecI2::new(new_cursor.x, old_cursor.y),
                )
            }
            Layout::RightLeft => {
                self.cursor.y = old_cursor.y;
                self.max_rect.y = old_max.y;
                self.max_rect.height = old_max.height;
                Rect::new_pos_pos(
                    VecI2::new(new_cursor.x, old_cursor.y),
                    VecI2::new(old_cursor.x, new_cursor.y),
                )
            }
        }
    }

    pub fn vertical<R, F: FnOnce(&mut Ui) -> R>(&mut self, func: F) -> R {
        self.layout(Layout::TopDown, func)
    }
    pub fn horizontal<R, F: FnOnce(&mut Ui) -> R>(&mut self, func: F) -> R {
        self.layout(Layout::LeftRight, func)
    }

    fn layout<R, F: FnOnce(&mut Ui) -> R>(&mut self, layout: Layout, func: F) -> R {
        let mut ui = self.clone();
        ui.current = Rect::new_pos_size(ui.cursor, VecI2::new(0, 0));
        ui.layout = layout;
        let res = func(&mut ui);
        self.allocate_area(ui.current);
        res
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

    pub fn interact(&mut self, area: Rect) -> Response{
        if let Some(mouse) = &self.context.inner.read().unwrap().mouse{
            if area.contains(mouse.position){
                Response{
                    clicked: mouse.left == MouseButtonState::Down,
                    pressed: mouse.left.is_down(),
                    hovered: true,
                    released: mouse.left == MouseButtonState::Up,
                    dragged: mouse.left == MouseButtonState::Drag,
                }
            }else{
                Response::nothing()
            }
        }else{
            Response::nothing()
        }
    }

    pub fn button(&mut self, text: impl Into<StyledText>) -> Response {
        let (area, mut gallery) = self.create_gallery(text.into());
        
        let response = self.interact(area);

        if response.pressed(){
            for item in &mut gallery {
                item.1.bg(Color::Blue);
            }
        }

        if response.hovered(){
            for item in &mut gallery {
                item.1.underline(true);
            }
        }
        
        self.draw_gallery(gallery);
        response
    }

    pub fn drop_down(&mut self, title: impl Into<StyledText>, func: impl FnOnce(&mut Ui)) {
        let mut text: StyledText = title.into();
        let id = Id::new(&text.text);
        self.with_memory_or(id, false, move |val, ui| {
            if val {
                text.text.push_str(symbols::pointers::TRIANGLE_DOWN)
            } else {
                text.text.push_str(symbols::pointers::TRIANGLE_RIGHT);
            }

            if ui.button(text).clicked() {
                ui.context.inner.write().unwrap().memory.insert(id, !val);
            }

            let layout = ui.layout;
            let used = ui.horizontal(|ui| {
                ui.add_horizontal_space(1);
                ui.layout(layout, |ui| {
                    if val {
                        func(ui)
                    }
                    ui.current
                })
            });

            let mut lock = ui.context.inner.write().unwrap();
            for i in 0..used.height {
                lock.draws.push(Draw::Text(
                    StyledText {
                        text: VERTICAL.into(),
                        style: Style::default(),
                    },
                    VecI2 {
                        x: used.x - 1,
                        y: used.y + i,
                    },
                ));
            }
        });
    }

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
        self.current
            .expand_to_include(&Rect::new_pos_size(self.cursor, VecI2::new(0, 0)));
    }

    pub fn expand(&mut self, translation: VecI2) {
        match self.layout {
            Layout::LeftRight | Layout::TopDown => self.current.add_bottom_right(translation),
            Layout::DownTop => {
                self.current.add_bottom_right(VecI2::new(translation.x, 0));
                self.current.add_top_left(VecI2::new(0, translation.y))
            }
            Layout::RightLeft => {
                self.current.add_bottom_right(VecI2::new(0, translation.y));
                self.current.add_top_left(VecI2::new(translation.x, 0))
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
