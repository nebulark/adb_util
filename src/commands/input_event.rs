use std::fmt::Display;

#[derive(Clone, Copy)]
pub struct InputEventInfo {
    pub timestamp_milliseconds: u32,
    pub event_nr: i32,
    pub event: InputEvent,
}

impl Display for InputEventInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] ev{} - {}",
            self.timestamp_milliseconds, self.event_nr, self.event
        )
    }
}

#[derive(Clone, Copy)]
pub enum InputEvent {
    BtnTouch(TouchType),
    AbsMtTrackingId(i32),
    AbsMtSlot(i32),
    AbsMtPosX(i32),
    AbsMtPosY(i32),
    KeyPower(TouchType),
}

impl Display for InputEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputEvent::BtnTouch(t) => write!(f, "Touch ({})", t),
            InputEvent::AbsMtTrackingId(i) => write!(f, "TrackId({})", i),
            InputEvent::AbsMtSlot(i) => write!(f, "Slot({})", i),
            InputEvent::AbsMtPosX(d) => write!(f, "PosX({})", d),
            InputEvent::AbsMtPosY(d) => write!(f, "PosY({})", d),
            InputEvent::KeyPower(t) => write!(f, "Power ({})", t),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TouchType {
    Up,
    Down,
}

impl Display for TouchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TouchType::Up => write!(f, "up"),
            TouchType::Down => write!(f, "down"),
        }
    }
}
