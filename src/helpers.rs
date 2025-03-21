use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryEntityError, QueryFilter, QueryItem, ROQueryItem},
    system::Query,
};
use bevy_input::keyboard::{Key, KeyCode};

/// Translates [`egui::CursorIcon`] into [`bevy_window::SystemCursorIcon`].
#[inline(always)]
pub fn egui_to_winit_cursor_icon(
    cursor_icon: egui::CursorIcon,
) -> Option<bevy_window::SystemCursorIcon> {
    match cursor_icon {
        egui::CursorIcon::Default => Some(bevy_window::SystemCursorIcon::Default),
        egui::CursorIcon::PointingHand => Some(bevy_window::SystemCursorIcon::Pointer),
        egui::CursorIcon::ResizeHorizontal => Some(bevy_window::SystemCursorIcon::EwResize),
        egui::CursorIcon::ResizeNeSw => Some(bevy_window::SystemCursorIcon::NeswResize),
        egui::CursorIcon::ResizeNwSe => Some(bevy_window::SystemCursorIcon::NwseResize),
        egui::CursorIcon::ResizeVertical => Some(bevy_window::SystemCursorIcon::NsResize),
        egui::CursorIcon::Text => Some(bevy_window::SystemCursorIcon::Text),
        egui::CursorIcon::Grab => Some(bevy_window::SystemCursorIcon::Grab),
        egui::CursorIcon::Grabbing => Some(bevy_window::SystemCursorIcon::Grabbing),
        egui::CursorIcon::ContextMenu => Some(bevy_window::SystemCursorIcon::ContextMenu),
        egui::CursorIcon::Help => Some(bevy_window::SystemCursorIcon::Help),
        egui::CursorIcon::Progress => Some(bevy_window::SystemCursorIcon::Progress),
        egui::CursorIcon::Wait => Some(bevy_window::SystemCursorIcon::Wait),
        egui::CursorIcon::Cell => Some(bevy_window::SystemCursorIcon::Cell),
        egui::CursorIcon::Crosshair => Some(bevy_window::SystemCursorIcon::Crosshair),
        egui::CursorIcon::VerticalText => Some(bevy_window::SystemCursorIcon::VerticalText),
        egui::CursorIcon::Alias => Some(bevy_window::SystemCursorIcon::Alias),
        egui::CursorIcon::Copy => Some(bevy_window::SystemCursorIcon::Copy),
        egui::CursorIcon::Move => Some(bevy_window::SystemCursorIcon::Move),
        egui::CursorIcon::NoDrop => Some(bevy_window::SystemCursorIcon::NoDrop),
        egui::CursorIcon::NotAllowed => Some(bevy_window::SystemCursorIcon::NotAllowed),
        egui::CursorIcon::AllScroll => Some(bevy_window::SystemCursorIcon::AllScroll),
        egui::CursorIcon::ZoomIn => Some(bevy_window::SystemCursorIcon::ZoomIn),
        egui::CursorIcon::ZoomOut => Some(bevy_window::SystemCursorIcon::ZoomOut),
        egui::CursorIcon::ResizeEast => Some(bevy_window::SystemCursorIcon::EResize),
        egui::CursorIcon::ResizeSouthEast => Some(bevy_window::SystemCursorIcon::SeResize),
        egui::CursorIcon::ResizeSouth => Some(bevy_window::SystemCursorIcon::SResize),
        egui::CursorIcon::ResizeSouthWest => Some(bevy_window::SystemCursorIcon::SwResize),
        egui::CursorIcon::ResizeWest => Some(bevy_window::SystemCursorIcon::WResize),
        egui::CursorIcon::ResizeNorthWest => Some(bevy_window::SystemCursorIcon::NwResize),
        egui::CursorIcon::ResizeNorth => Some(bevy_window::SystemCursorIcon::NResize),
        egui::CursorIcon::ResizeNorthEast => Some(bevy_window::SystemCursorIcon::NeResize),
        egui::CursorIcon::ResizeColumn => Some(bevy_window::SystemCursorIcon::ColResize),
        egui::CursorIcon::ResizeRow => Some(bevy_window::SystemCursorIcon::RowResize),
        egui::CursorIcon::None => None,
    }
}

/// Matches the implementation of <https://github.com/emilk/egui/blob/68b3ef7f6badfe893d3bbb1f791b481069d807d9/crates/egui-winit/src/lib.rs#L1005>.
#[inline(always)]
pub fn bevy_to_egui_key(key: &Key) -> Option<egui::Key> {
    let key = match key {
        Key::Character(str) => return egui::Key::from_name(str.as_str()),
        Key::Unidentified(_) | Key::Dead(_) => return None,

        Key::Enter => egui::Key::Enter,
        Key::Tab => egui::Key::Tab,
        Key::Space => egui::Key::Space,
        Key::ArrowDown => egui::Key::ArrowDown,
        Key::ArrowLeft => egui::Key::ArrowLeft,
        Key::ArrowRight => egui::Key::ArrowRight,
        Key::ArrowUp => egui::Key::ArrowUp,
        Key::End => egui::Key::End,
        Key::Home => egui::Key::Home,
        Key::PageDown => egui::Key::PageDown,
        Key::PageUp => egui::Key::PageUp,
        Key::Backspace => egui::Key::Backspace,
        Key::Delete => egui::Key::Delete,
        Key::Insert => egui::Key::Insert,
        Key::Escape => egui::Key::Escape,
        Key::F1 => egui::Key::F1,
        Key::F2 => egui::Key::F2,
        Key::F3 => egui::Key::F3,
        Key::F4 => egui::Key::F4,
        Key::F5 => egui::Key::F5,
        Key::F6 => egui::Key::F6,
        Key::F7 => egui::Key::F7,
        Key::F8 => egui::Key::F8,
        Key::F9 => egui::Key::F9,
        Key::F10 => egui::Key::F10,
        Key::F11 => egui::Key::F11,
        Key::F12 => egui::Key::F12,
        Key::F13 => egui::Key::F13,
        Key::F14 => egui::Key::F14,
        Key::F15 => egui::Key::F15,
        Key::F16 => egui::Key::F16,
        Key::F17 => egui::Key::F17,
        Key::F18 => egui::Key::F18,
        Key::F19 => egui::Key::F19,
        Key::F20 => egui::Key::F20,

        _ => return None,
    };
    Some(key)
}

/// Matches the implementation of <https://github.com/emilk/egui/blob/68b3ef7f6badfe893d3bbb1f791b481069d807d9/crates/egui-winit/src/lib.rs#L1080>.
#[inline(always)]
pub fn bevy_to_egui_physical_key(key: &KeyCode) -> Option<egui::Key> {
    let key = match key {
        KeyCode::ArrowDown => egui::Key::ArrowDown,
        KeyCode::ArrowLeft => egui::Key::ArrowLeft,
        KeyCode::ArrowRight => egui::Key::ArrowRight,
        KeyCode::ArrowUp => egui::Key::ArrowUp,

        KeyCode::Escape => egui::Key::Escape,
        KeyCode::Tab => egui::Key::Tab,
        KeyCode::Backspace => egui::Key::Backspace,
        KeyCode::Enter | KeyCode::NumpadEnter => egui::Key::Enter,

        KeyCode::Insert => egui::Key::Insert,
        KeyCode::Delete => egui::Key::Delete,
        KeyCode::Home => egui::Key::Home,
        KeyCode::End => egui::Key::End,
        KeyCode::PageUp => egui::Key::PageUp,
        KeyCode::PageDown => egui::Key::PageDown,

        // Punctuation
        KeyCode::Space => egui::Key::Space,
        KeyCode::Comma => egui::Key::Comma,
        KeyCode::Period => egui::Key::Period,
        // KeyCode::Colon => egui::Key::Colon, // NOTE: there is no physical colon key on an american keyboard
        KeyCode::Semicolon => egui::Key::Semicolon,
        KeyCode::Backslash => egui::Key::Backslash,
        KeyCode::Slash | KeyCode::NumpadDivide => egui::Key::Slash,
        KeyCode::BracketLeft => egui::Key::OpenBracket,
        KeyCode::BracketRight => egui::Key::CloseBracket,
        KeyCode::Backquote => egui::Key::Backtick,

        KeyCode::Cut => egui::Key::Cut,
        KeyCode::Copy => egui::Key::Copy,
        KeyCode::Paste => egui::Key::Paste,
        KeyCode::Minus | KeyCode::NumpadSubtract => egui::Key::Minus,
        KeyCode::NumpadAdd => egui::Key::Plus,
        KeyCode::Equal => egui::Key::Equals,

        KeyCode::Digit0 | KeyCode::Numpad0 => egui::Key::Num0,
        KeyCode::Digit1 | KeyCode::Numpad1 => egui::Key::Num1,
        KeyCode::Digit2 | KeyCode::Numpad2 => egui::Key::Num2,
        KeyCode::Digit3 | KeyCode::Numpad3 => egui::Key::Num3,
        KeyCode::Digit4 | KeyCode::Numpad4 => egui::Key::Num4,
        KeyCode::Digit5 | KeyCode::Numpad5 => egui::Key::Num5,
        KeyCode::Digit6 | KeyCode::Numpad6 => egui::Key::Num6,
        KeyCode::Digit7 | KeyCode::Numpad7 => egui::Key::Num7,
        KeyCode::Digit8 | KeyCode::Numpad8 => egui::Key::Num8,
        KeyCode::Digit9 | KeyCode::Numpad9 => egui::Key::Num9,

        KeyCode::KeyA => egui::Key::A,
        KeyCode::KeyB => egui::Key::B,
        KeyCode::KeyC => egui::Key::C,
        KeyCode::KeyD => egui::Key::D,
        KeyCode::KeyE => egui::Key::E,
        KeyCode::KeyF => egui::Key::F,
        KeyCode::KeyG => egui::Key::G,
        KeyCode::KeyH => egui::Key::H,
        KeyCode::KeyI => egui::Key::I,
        KeyCode::KeyJ => egui::Key::J,
        KeyCode::KeyK => egui::Key::K,
        KeyCode::KeyL => egui::Key::L,
        KeyCode::KeyM => egui::Key::M,
        KeyCode::KeyN => egui::Key::N,
        KeyCode::KeyO => egui::Key::O,
        KeyCode::KeyP => egui::Key::P,
        KeyCode::KeyQ => egui::Key::Q,
        KeyCode::KeyR => egui::Key::R,
        KeyCode::KeyS => egui::Key::S,
        KeyCode::KeyT => egui::Key::T,
        KeyCode::KeyU => egui::Key::U,
        KeyCode::KeyV => egui::Key::V,
        KeyCode::KeyW => egui::Key::W,
        KeyCode::KeyX => egui::Key::X,
        KeyCode::KeyY => egui::Key::Y,
        KeyCode::KeyZ => egui::Key::Z,

        KeyCode::F1 => egui::Key::F1,
        KeyCode::F2 => egui::Key::F2,
        KeyCode::F3 => egui::Key::F3,
        KeyCode::F4 => egui::Key::F4,
        KeyCode::F5 => egui::Key::F5,
        KeyCode::F6 => egui::Key::F6,
        KeyCode::F7 => egui::Key::F7,
        KeyCode::F8 => egui::Key::F8,
        KeyCode::F9 => egui::Key::F9,
        KeyCode::F10 => egui::Key::F10,
        KeyCode::F11 => egui::Key::F11,
        KeyCode::F12 => egui::Key::F12,
        KeyCode::F13 => egui::Key::F13,
        KeyCode::F14 => egui::Key::F14,
        KeyCode::F15 => egui::Key::F15,
        KeyCode::F16 => egui::Key::F16,
        KeyCode::F17 => egui::Key::F17,
        KeyCode::F18 => egui::Key::F18,
        KeyCode::F19 => egui::Key::F19,
        KeyCode::F20 => egui::Key::F20,
        _ => return None,
    };
    Some(key)
}

/// Converts [`bevy_math::Vec2`] into [`egui::Pos2`].
#[inline(always)]
pub fn vec2_into_egui_pos2(vec: bevy_math::Vec2) -> egui::Pos2 {
    egui::Pos2::new(vec.x, vec.y)
}

/// Converts [`bevy_math::Vec2`] into [`egui::Vec2`].
#[inline(always)]
pub fn vec2_into_egui_vec2(vec: bevy_math::Vec2) -> egui::Vec2 {
    egui::Vec2::new(vec.x, vec.y)
}

/// Converts [`bevy_math::Rect`] into [`egui::Rect`].
#[inline(always)]
pub fn rect_into_egui_rect(rect: bevy_math::Rect) -> egui::Rect {
    egui::Rect {
        min: vec2_into_egui_pos2(rect.min),
        max: vec2_into_egui_pos2(rect.max),
    }
}

/// Converts [`egui::Pos2`] into [`bevy_math::Vec2`].
#[inline(always)]
pub fn egui_pos2_into_vec2(pos: egui::Pos2) -> bevy_math::Vec2 {
    bevy_math::Vec2::new(pos.x, pos.y)
}

/// Converts [`egui::Vec2`] into [`bevy_math::Vec2`].
#[inline(always)]
pub fn egui_vec2_into_vec2(pos: egui::Vec2) -> bevy_math::Vec2 {
    bevy_math::Vec2::new(pos.x, pos.y)
}

/// Converts [`egui::Rect`] into [`bevy_math::Rect`].
#[inline(always)]
pub fn egui_rect_into_rect(rect: egui::Rect) -> bevy_math::Rect {
    bevy_math::Rect {
        min: egui_pos2_into_vec2(rect.min),
        max: egui_pos2_into_vec2(rect.max),
    }
}

pub(crate) trait QueryHelper<'w> {
    type QueryData: bevy_ecs::query::QueryData;

    fn get_some(&self, entity: Entity) -> Option<ROQueryItem<'_, Self::QueryData>>;

    fn get_some_mut(&mut self, entity: Entity) -> Option<QueryItem<'_, Self::QueryData>>;
}

impl<'w, D: QueryData, F: QueryFilter> QueryHelper<'w> for Query<'w, '_, D, F> {
    type QueryData = D;

    fn get_some(&self, entity: Entity) -> Option<ROQueryItem<'_, Self::QueryData>> {
        match self.get(entity) {
            Ok(item) => Some(item),
            Err(QueryEntityError::EntityDoesNotExist(_)) => None,
            err => {
                err.unwrap();
                unreachable!()
            }
        }
    }

    fn get_some_mut(&mut self, entity: Entity) -> Option<QueryItem<'_, Self::QueryData>> {
        match self.get_mut(entity) {
            Ok(item) => Some(item),
            Err(QueryEntityError::EntityDoesNotExist(_)) => None,
            err => {
                err.unwrap();
                unreachable!()
            }
        }
    }
}
