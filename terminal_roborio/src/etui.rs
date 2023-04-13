use std::{
    num::NonZeroU8,
    sync::{Arc, RwLock},
};

use crossterm::{
    event::{Event, MouseButton, MouseEventKind},
    style::{Attribute, Attributes, Color},
};

use self::{
    id::Id,
    math_util::{Rect, VecI2},
    memory::Memory,
    screen::{Screen, ScreenDrain, ScreenIter},
    symbols::line::*,
};

pub mod id;
pub mod math_util;
pub mod memory;
pub mod screen;
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
    pub buttons: [MouseButtonState; 3],
    pub scroll: i16,
}

#[derive(Debug, Default)]
pub struct ContextInner {
    event: Option<Event>,
    mouse: Option<MouseState>,
    max_rect: Rect,
    memory: Memory,

    current: Screen,
    last: Screen,
}
impl ContextInner {
    fn new(size: VecI2) -> ContextInner {
        let mut myself = Self {
            max_rect: Rect::new_pos_size(VecI2::new(0,0), size),
            ..Default::default()
        };
        myself.current.resize(size);
        myself.last.resize(size);
        myself
    }

    pub fn draw(&mut self, str: &str, style: Style, start: VecI2, layer: NonZeroU8, clip: Rect) {
        self.current.push_text(
            str,
            style,
            start,
            layer,
            clip,
        )
    }

    pub fn finish_frame(&mut self) -> (ScreenIter<'_>, ScreenDrain<'_>) {
        if let Some(mouse) = &mut self.mouse {
            for button in &mut mouse.buttons {
                button.next_state();
            }
        }

        self.memory.clear_seen();
        let ContextInner { current, last, .. } = self;
        std::mem::swap(last, current);
        (last.iter(), current.drain())
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
            layout: Layout::TopLeftVertical,
            max_rect: lock.max_rect,
            cursor: {
                drop(lock);
                Default::default()
            },
            context: (*self).clone(),
            current: Default::default(),
            layer: NonZeroU8::new(1).unwrap(),
        };
        func(&mut ui);
    }

    pub fn inner(&mut self) -> &mut Arc<RwLock<ContextInner>> {
        &mut self.inner
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
                            MouseButton::Left => &mut mouse.buttons[0],
                            MouseButton::Right => &mut mouse.buttons[2],
                            MouseButton::Middle => &mut mouse.buttons[1],
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

    pub fn new(size: VecI2) -> Context {
        Self {
            inner: Arc::new(RwLock::new(ContextInner::new(size))),
        }
    }

    pub fn clear_event(&self) {
        self.inner.write().unwrap().event = None
    }

    pub fn draw(&mut self, str: &str, style: Style, start: VecI2, layer: NonZeroU8, clip: Rect) {
        let mut lock = self.inner.write().unwrap();
        lock.current.push_text(str, style, start, layer, clip)
    }

    fn interact(&self, clip: Rect, id: Id, area: Rect) -> Response {
        let lock = self.inner.read().unwrap();
        if let Some(mouse) = &lock.mouse {
            if area.contains(mouse.position) {
                let mut response = Response::new(area, id, Some(mouse.position));
                response.buttons = mouse.buttons;
                response
            } else {
                Response::new(area, id, None)
            }
        } else {
            Response::new(area, id, None)
        }
    }

    pub fn set_size(&self, last_observed_size: VecI2) -> bool {
        let mut lock = self.inner.write().unwrap();
        if lock.max_rect.size() != last_observed_size{
            lock.max_rect = Rect::new_pos_size(VecI2::new(0,0), last_observed_size);
            lock.current.resize(last_observed_size);
            lock.last.resize(last_observed_size);
            true
        }else{
            false
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Response {
    hovered: bool,
    buttons: [MouseButtonState; 3],
    id: Id,
    rect: Rect,
    mouse_pos: Option<VecI2>,
}

impl Response {
    pub fn new(rect: Rect, id: Id, mouse: Option<VecI2>) -> Self {
        Self {
            hovered: mouse.map(|m| rect.contains(m)).unwrap_or(false),
            buttons: Default::default(),
            id,
            rect,
            mouse_pos: mouse,
        }
    }
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    pub fn released(&self) -> bool {
        self.buttons[0] == MouseButtonState::Up
    }

    pub fn clicked(&self) -> bool {
        self.buttons[0] == MouseButtonState::Down
    }

    pub fn pressed(&self) -> bool {
        self.buttons[0].is_down()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layout {
    TopLeftVertical,
    TopLeftHorizontal,
    TopRightVertical,
    TopRightHorizontal,
    BottomLeftVertical,
    BottomLeftHorizontal,
    BottomRightVertical,
    BottomRightHorizontal,
}

impl Layout {
    pub fn is_primary_vertical(&self) -> bool {
        match self {
            Layout::TopLeftVertical => true,
            Layout::TopLeftHorizontal => false,
            Layout::TopRightVertical => true,
            Layout::TopRightHorizontal => false,
            Layout::BottomLeftVertical => true,
            Layout::BottomLeftHorizontal => false,
            Layout::BottomRightVertical => true,
            Layout::BottomRightHorizontal => false,
        }
    }

    pub fn is_primary_horizontal(&self) -> bool {
        !self.is_primary_vertical()
    }

    pub fn to_vertical(&self) -> Self {
        match self {
            Layout::TopLeftVertical | Layout::TopLeftHorizontal => Layout::TopLeftVertical,
            Layout::TopRightVertical | Layout::TopRightHorizontal => Layout::TopRightVertical,
            Layout::BottomLeftVertical | Layout::BottomLeftHorizontal => Layout::BottomLeftVertical,
            Layout::BottomRightVertical | Layout::BottomRightHorizontal => {
                Layout::BottomRightVertical
            }
        }
    }

    pub fn to_horizontal(&self) -> Self {
        match self {
            Layout::TopLeftVertical | Layout::TopLeftHorizontal => Layout::TopLeftHorizontal,
            Layout::TopRightVertical | Layout::TopRightHorizontal => Layout::TopRightHorizontal,
            Layout::BottomLeftVertical | Layout::BottomLeftHorizontal => {
                Layout::BottomLeftHorizontal
            }
            Layout::BottomRightVertical | Layout::BottomRightHorizontal => {
                Layout::BottomRightHorizontal
            }
        }
    }

    pub fn opposite_primary_direction(&self) -> Self {
        if self.is_primary_vertical() {
            self.to_horizontal()
        } else {
            self.to_vertical()
        }
    }
}

#[derive(Clone)]
pub struct Ui {
    context: Context,
    layout: Layout,
    clip: Rect,
    max_rect: Rect,
    cursor: VecI2,
    current: Rect,
    layer: NonZeroU8,
}

impl Ui {
    pub fn label(&mut self, text: impl Into<StyledText>) {
        let gallery = self.create_gallery(text.into());
        self.allocate_area(gallery.bound);
        self.draw_gallery(gallery);
    }

    pub fn get_clip(&self) -> Rect {
        self.clip
    }

    pub fn get_max(&self) -> Rect {
        self.max_rect
    }

    pub fn set_max(&mut self, max: Rect) {
        self.max_rect = max;
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

    pub fn with_memory_or<T: Clone + 'static, F: FnOnce(T, &mut Self) -> R, R>(
        &mut self,
        id: Id,
        default: T,
        func: F,
    ) -> Option<R> {
        let mut lock = self.context.inner.write().unwrap();
        let res = lock.memory.get_mut_or(id, default);
        drop(lock);

        if let Ok(val) = res {
            return Some(func(val, self));
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
        None
    }

    pub fn with_memory_or_make<T: Clone + 'static, F: FnOnce(T, &mut Self) -> R, R>(
        &mut self,
        id: Id,
        default: impl FnOnce() -> T,
        func: F,
    ) -> Option<R> {
        let mut lock = self.context.inner.write().unwrap();
        let res = lock.memory.get_mut_or(id, default());
        drop(lock);
        if let Ok(val) = res {
            return Some(func(val, self));
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
        None
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

    pub fn tabbed_area<F: FnOnce(usize, &mut Self) -> R, R, const L: usize>(
        &mut self,
        id: Id,
        titles: [impl Into<StyledText>; L],
        func: F,
    ) -> Option<R> {
        self.with_memory_or(id, 0usize, |mut val, ui| {
            // let start = ui.cursor;
            ui.layout(ui.layout, |ui| {
                ui.add_space_primary_direction(1);
                ui.layout(ui.layout.opposite_primary_direction(), |ui| {
                    ui.add_space_primary_direction(1);
                    for (i, title) in titles.into_iter().enumerate() {
                        let mut title: StyledText = title.into();
                        if i == val {
                            title.bg(Color::DarkGrey)
                        }
                        if ui.button(title).clicked() {
                            val = i;
                            ui.context.inner.write().unwrap().memory.insert(id, i);
                        }
                        ui.add_space_primary_direction(1);
                    }
                });
                ui.add_space_primary_direction(1);

                let tab_box = ui.current;

                let res = func(val, ui);

                let mut bruh = BoxedArea::default();
                bruh.add_line(tab_box.top_left(), tab_box.top_right_inner());
                bruh.add_line(tab_box.top_right_inner(), tab_box.bottom_right_inner());
                bruh.add_line(tab_box.bottom_right_inner(), tab_box.bottom_left_inner());
                bruh.add_line(tab_box.bottom_left_inner(), tab_box.top_left());
                bruh.draw(
                    ui.ctx(),
                    Style::default(),
                    &crate::etui::symbols::line::NORMAL,
                );

                res
            })
        })
    }

    pub fn progress_bar(
        &mut self,
        mut style: Style,
        min_size: u16,
        max_size: u16,
        width: u16,
        layout: Layout,
        progress: f32,
    ) -> Response {
        let mut string = String::new();

        let cursor = self.cursor;

        let (len, area) = if self.layout.is_primary_horizontal() {
            let size = self.current.width.clamp(min_size, max_size);
            let rect = self.allocate_size(VecI2::new(size, 1));
            (rect.width, rect)
        } else {
            let size = self.current.height.clamp(min_size, max_size);
            let rect = self.allocate_size(VecI2::new(1, size));
            (rect.height, rect)
        };

        let complete = (len as f32 * progress.clamp(0.0, 1.0) * 8.0) as u32;
        let whole = complete / 8;
        let remaining = ((len as u32 * 8) - complete) / 8;

        for _ in 0..whole {
            for _ in 0..width {
                string.push('█');
            }
            if layout.is_primary_vertical() {
                string.push('\n');
            }
        }
        match layout {
            Layout::TopLeftVertical => style.attributes.set(Attribute::Reverse),
            Layout::TopLeftHorizontal => style.attributes.set(Attribute::NoReverse),
            Layout::TopRightVertical => style.attributes.set(Attribute::Reverse),
            Layout::TopRightHorizontal => style.attributes.set(Attribute::Reverse),
            Layout::BottomLeftVertical => style.attributes.set(Attribute::NoReverse),
            Layout::BottomLeftHorizontal => style.attributes.set(Attribute::NoReverse),
            Layout::BottomRightVertical => style.attributes.set(Attribute::NoReverse),
            Layout::BottomRightHorizontal => style.attributes.set(Attribute::Reverse),
        }

        if whole + remaining != len as u32 {
            let t = if layout.is_primary_horizontal() {
                match complete % 8 {
                    0 => ' ',
                    1 => '▏',
                    2 => '▎',
                    3 => '▍',
                    4 => '▌',
                    5 => '▋',
                    6 => '▊',
                    7 => '▉',
                    // not gonna happen
                    _ => ' ',
                }
            } else {
                match complete % 8 {
                    0 => ' ',
                    1 => '▁',
                    2 => '▂',
                    3 => '▃',
                    4 => '▄',
                    5 => '▅',
                    6 => '▆',
                    7 => '▇',
                    // not gonna happen
                    _ => ' ',
                }
            };
            for _ in 0..width {
                string.push(t);
            }
            if layout.is_primary_vertical() {
                string.push('\n');
            }
        }
        for _ in 0..remaining {
            for _ in 0..width {
                string.push(' ');
            }
            if layout.is_primary_vertical() {
                string.push('\n');
            }
        }
        if self.layout.is_primary_vertical() {
            string = string.chars().rev().collect();
        }
        string = string.trim_matches('\n').to_owned();
        let gallery = self.create_gallery_at(cursor, StyledText::styled(string, style));
        // assert_eq!(gallery.bound, area, "{:#?}", gallery.items);
        self.draw_gallery(gallery);

        self.interact(Id::new("Bruh"), area)
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
        child.current = Rect::new_pos_size(child.cursor, VecI2::new(0, 0));

        func(&mut child);

        child
            .current
            .expand_to_include(&Rect::new_pos_size(start, VecI2::new(0, 0)));
        child.expand(VecI2::new(1, 1));
        let border = child.current;

        let mut lock = self.context.inner.write().unwrap();

        

        lock.draw(
            TOP_LEFT,
            Style::default(),
            border.top_left(),
            self.layer,
            border,
        );
        lock.draw(
            TOP_RIGHT,
            Style::default(),
            border.top_right_inner(),
            self.layer,
            border,
        );
        lock.draw(
            BOTTOM_RIGHT,
            Style::default(),
            border.bottom_right_inner(),
            self.layer,
            border,
        );
        lock.draw(
            BOTTOM_LEFT,
            Style::default(),
            border.bottom_left_inner(),
            self.layer,
            border,
        );

        for i in 1..(border.width - 1) {
            lock.draw(
                HORIZONTAL,
                Style::default(),
                VecI2 {
                    x: border.x + i,
                    y: border.y,
                },
                self.layer,
                border,
            );
            lock.draw(
                HORIZONTAL,
                Style::default(),
                VecI2 {
                    x: border.x + i,
                    y: border.bottom_right_inner().y,
                },
                self.layer,
                border,
            );
        }

        for i in 1..(border.height - 1) {
            lock.draw(
                VERTICAL,
                Style::default(),
                VecI2 {
                    x: border.x,
                    y: border.y + i,
                },
                self.layer,
                border,
            );
            lock.draw(
                VERTICAL,
                Style::default(),
                VecI2 {
                    x: border.bottom_right_inner().x,
                    y: border.y + i,
                },
                self.layer,
                border,
            );
        }
        drop(lock);
        self.allocate_size(child.current.size());
    }

    fn allocate_area(&mut self, rect: Rect) -> Rect {
        let start = match self.layout {
            Layout::TopLeftVertical | Layout::TopLeftHorizontal => rect.top_left(),
            Layout::TopRightVertical | Layout::TopRightHorizontal => rect.top_right(),
            Layout::BottomLeftVertical | Layout::BottomLeftHorizontal => rect.bottom_left(),
            Layout::BottomRightVertical | Layout::BottomRightHorizontal => rect.bottom_right(),
        };
        if start == self.cursor {
            self.allocate_size(rect.size())
        } else {
            let mut cpy = rect;
            cpy.shrink_evenly(1);
            if cpy.contains(self.cursor) {
                panic!("Cannot allocate before cursor")
            } else {
                cpy.expand_to_include(&Rect::new_pos_size(self.cursor, VecI2::new(0, 0)));
                self.allocate_size(cpy.size())
            }
        }
    }

    pub fn vertical<R, F: FnOnce(&mut Ui) -> R>(&mut self, func: F) -> R {
        self.layout(self.layout.to_vertical(), func)
    }
    pub fn horizontal<R, F: FnOnce(&mut Ui) -> R>(&mut self, func: F) -> R {
        self.layout(self.layout.to_horizontal(), func)
    }

    pub fn layout<R, F: FnOnce(&mut Ui) -> R>(&mut self, layout: Layout, func: F) -> R {
        let mut ui = self.clone();

        match layout {
            Layout::TopLeftHorizontal | Layout::TopLeftVertical => {
                ui.cursor = ui.max_rect.top_left();
            }
            Layout::TopRightHorizontal | Layout::TopRightVertical => {
                ui.cursor = ui.max_rect.top_right_inner();
            }
            Layout::BottomLeftHorizontal | Layout::BottomLeftVertical => {
                ui.cursor = ui.max_rect.bottom_left_inner();
            }
            Layout::BottomRightHorizontal | Layout::BottomRightVertical => {
                ui.cursor = ui.max_rect.bottom_right_inner();
            }
        }
        ui.current = Rect::new_pos_size(ui.cursor, VecI2::new(0, 0));
        ui.layout = layout;
        let res = func(&mut ui);

        self.allocate_area(ui.current);

        res
    }

    pub fn seperator(&mut self) {
        if self.layout.is_primary_horizontal() {
            let area = self.allocate_size(VecI2::new(1, self.current.height));

            let mut lock = self.context.inner.write().unwrap();
            for i in 0..area.height {
                lock.draw(
                    VERTICAL,
                    Style::default(),
                    VecI2 {
                        x: area.x,
                        y: self.current.y + i,
                    },
                    self.layer,
                    area,
                );
            }
        } else {
            let area = self.allocate_size(VecI2::new(self.current.width, 1));
            let mut lock = self.context.inner.write().unwrap();
            for i in 0..area.width {
                lock.draw(
                    HORIZONTAL,
                    Style::default(),
                    VecI2 {
                        x: self.current.x + i,
                        y: area.y,
                    },
                    self.layer,
                    area,
                );
            }
        }
    }

    fn draw_gallery(&mut self, gallery: Gallery) {
        let mut lock = self.context.inner.write().unwrap();
        
        for (bound, text) in gallery.items {
            lock.draw(
                &text.text,
                text.style,
                bound.top_left(),
                self.layer,
                bound,
            );
        }
    }

    pub fn interact(&mut self, id: Id, area: Rect) -> Response {
        self.context.interact(self.clip, id, area)
    }

    pub fn button(&mut self, text: impl Into<StyledText>) -> Response {
        let mut gallery = self.create_gallery(text.into());
        let area = self.allocate_area(gallery.bound);
        gallery.bound = area;
        let response = self.interact(Id::new("As"), gallery.bound);

        if response.pressed() {
            for item in &mut gallery.items {
                item.1.bg(Color::Blue);
            }
        }

        if response.hovered() {
            for item in &mut gallery.items {
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
                // lock.draws.push(Draw::Text(
                //     StyledText {
                //         text: VERTICAL.into(),
                //         style: Style::default(),
                //     },
                //     VecI2 {
                //         x: used.x - 1,
                //         y: used.y + i,
                //     },
                // ));
                lock.draw(
                    VERTICAL,
                    Style::default(),
                    VecI2 {
                        x: used.x - 1,
                        y: used.y + i,
                    },
                    ui.layer,
                    ui.clip,
                );
            }
        });
    }

    fn create_gallery(&self, text: StyledText) -> Gallery {
        self.create_gallery_at(self.cursor, text)
    }

    fn create_gallery_at(&self, pos: VecI2, text: StyledText) -> Gallery {
        let mut rect = Rect::new_pos_size(pos, VecI2::new(0, 0));

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
            // rect.in
            rect.height += 1;
            rect.width = rect.width.max(line_width);
        }

        match self.layout {
            Layout::TopLeftVertical | Layout::TopLeftHorizontal => {}
            Layout::TopRightVertical | Layout::TopRightHorizontal => {
                rect.x = rect.x.saturating_sub(rect.width) + 1;
                for (bound, _item) in &mut gallery {
                    bound.x = bound.x.saturating_sub(rect.width) + 1;
                }
            }
            Layout::BottomLeftVertical | Layout::BottomLeftHorizontal => {
                rect.y = rect.y.saturating_sub(rect.height) + 1;
                for (bound, _item) in &mut gallery {
                    bound.y = bound.y.saturating_sub(rect.height) + 1;
                }
            }
            Layout::BottomRightVertical | Layout::BottomRightHorizontal => {
                rect.y = rect.y.saturating_sub(rect.height) + 1;
                rect.x = rect.x.saturating_sub(rect.width) + 1;
                for (bound, _item) in &mut gallery {
                    bound.x = bound.x.saturating_sub(rect.width) + 1;
                    bound.y = bound.y.saturating_sub(rect.height) + 1;
                }
            }
        }

        Gallery {
            bound: rect,
            items: gallery,
        }
    }

    fn allocate_size(&mut self, desired: VecI2) -> Rect {
        let old_cursor = self.cursor;
        let old_max = self.max_rect;
        self.add_space(desired);
        let new_cursor = self.cursor;

        if self.layout.is_primary_vertical() {
            self.cursor.x = old_cursor.x;
            self.max_rect.x = old_max.x;
            self.max_rect.width = old_max.width;
            Rect::new_pos_pos(old_cursor, new_cursor)
        } else {
            self.cursor.y = old_cursor.y;
            self.max_rect.y = old_max.y;
            self.max_rect.height = old_max.height;
            Rect::new_pos_pos(old_cursor, new_cursor)
        }
    }

    pub fn add_horizontal_space(&mut self, space: u16) {
        self.add_space(VecI2::new(space, 0))
    }

    pub fn add_vertical_space(&mut self, space: u16) {
        self.add_space(VecI2::new(0, space))
    }

    pub fn add_space(&mut self, space: VecI2) {
        match self.layout {
            Layout::TopLeftHorizontal | Layout::TopLeftVertical => {
                self.cursor += space;

                self.clip.move_top_left_to(self.cursor);
                self.max_rect.move_top_left_to(self.cursor);
            }
            Layout::TopRightHorizontal | Layout::TopRightVertical => {
                self.cursor += VecI2::new(0, space.y);
                self.cursor -= VecI2::new(space.x, 0);

                self.clip.move_top_right_to(self.cursor + VecI2::new(1, 0));
                self.max_rect
                    .move_top_right_to(self.cursor + VecI2::new(1, 0));
            }
            Layout::BottomLeftHorizontal | Layout::BottomLeftVertical => {
                self.cursor -= VecI2::new(0, space.y);
                self.cursor += VecI2::new(space.x, 0);

                self.clip
                    .move_bottom_left_to(self.cursor + VecI2::new(0, 1));
                self.max_rect
                    .move_bottom_left_to(self.cursor + VecI2::new(0, 1));
            }
            Layout::BottomRightHorizontal | Layout::BottomRightVertical => {
                self.cursor -= VecI2::new(space.x, space.y);

                self.clip
                    .move_bottom_right_to(self.cursor + VecI2::new(1, 1));
                self.max_rect
                    .move_bottom_right_to(self.cursor + VecI2::new(1, 1));
            }
        }
        self.current
            .expand_to_include(&Rect::new_pos_size(self.cursor, VecI2::new(0, 0)));
    }

    pub fn expand(&mut self, translation: VecI2) {
        match self.layout {
            Layout::TopLeftHorizontal | Layout::TopLeftVertical => {
                self.current.add_bottom_right(translation)
            }
            Layout::TopRightHorizontal | Layout::TopRightVertical => {
                self.current.add_bottom_right(VecI2::new(translation.x, 0));
                self.current.add_top_left(VecI2::new(0, translation.y))
            }
            Layout::BottomLeftHorizontal | Layout::BottomLeftVertical => {
                self.current.add_bottom_right(VecI2::new(0, translation.y));
                self.current.add_top_left(VecI2::new(translation.x, 0))
            }
            Layout::BottomRightHorizontal | Layout::BottomRightVertical => {
                self.current.add_top_left(translation)
            }
        }
    }

    pub fn set_minimum_size(&mut self, mut min: VecI2) {
        min.x = min.x.min(self.max_rect.width);
        min.y = min.y.min(self.max_rect.height);
        match self.layout {
            Layout::TopLeftHorizontal | Layout::TopLeftVertical => {
                self.current.width = self.current.width.max(min.x);
                self.current.height = self.current.height.max(min.y);
            }
            Layout::TopRightHorizontal | Layout::TopRightVertical => {
                todo!();
            }
            Layout::BottomLeftHorizontal | Layout::BottomLeftVertical => {
                todo!();
            }
            Layout::BottomRightHorizontal | Layout::BottomRightVertical => {
                todo!();
            }
        }
    }

    pub fn add_space_primary_direction(&mut self, space: u16) {
        if self.layout.is_primary_horizontal() {
            self.add_space(VecI2::new(space, 0));
        } else {
            self.add_space(VecI2::new(0, space));
        }
    }
}

struct Gallery {
    bound: Rect,
    items: Vec<(Rect, StyledText)>,
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

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
struct NodeAttachements {
    up: bool,
    left: bool,
    right: bool,
    down: bool,
}

#[derive(Debug, Default, Clone)]
struct BoxedArea {
    vertices: std::collections::HashMap<VecI2, NodeAttachements>,
    lines: Vec<(VecI2, VecI2, bool)>,
}

impl BoxedArea {
    pub fn add_line(&mut self, p1: VecI2, p2: VecI2) {
        assert!(p1 != p2);
        if p1.x == p2.x {
            let p1_node = self.vertices.entry(p1).or_insert_with(Default::default);
            if p1.y > p2.y {
                p1_node.down = true;
            } else {
                p1_node.up = true;
            }

            let p2_node = self.vertices.entry(p2).or_insert_with(Default::default);
            if p1.y > p2.y {
                p2_node.up = true;
            } else {
                p2_node.down = true;
            }
            self.lines.push((p1, p2, false))
        } else if p1.y == p2.y {
            let p1_node = self.vertices.entry(p1).or_insert_with(Default::default);
            if p1.x > p2.x {
                p1_node.right = true;
            } else {
                p1_node.left = true;
            }

            let p2_node = self.vertices.entry(p2).or_insert_with(Default::default);
            if p1.x > p2.x {
                p2_node.left = true;
            } else {
                p2_node.right = true;
            }
            self.lines.push((p1, p2, true))
        } else {
            panic!("Not stright line");
        }
    }

    pub fn draw(&self, ctx: &Context, style: Style, set: &crate::etui::symbols::line::Set) {
        let mut lock = ctx.inner.write().unwrap();

        for (pos, node) in &self.vertices {
            let val = match (node.up, node.right, node.down, node.left) {
                (true, false, true, false) => set.vertical,
                (true, true, true, false) => set.vertical_right,
                (true, false, true, true) => set.vertical_left,

                (false, true, false, true) => set.horizontal,
                (true, true, false, true) => set.horizontal_down,
                (false, true, true, true) => set.horizontal_up,

                (true, true, false, false) => set.top_right,
                (false, true, true, false) => set.bottom_right,
                (false, false, true, true) => set.bottom_left,
                (true, false, false, true) => set.top_left,

                (true, true, true, true) => set.cross,
                _ => "*",
            };
            let clip = lock.max_rect;
            lock.draw(val, style, *pos, NonZeroU8::new(1).unwrap(), clip);
            // lock.draws.push(Draw::Text(
            //     StyledText {
            //         text: val.to_owned(),
            //         style,
            //     },
            //     *pos,
            // ))
        }
    }
}
