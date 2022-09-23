use core::fmt;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use crate::input_event::{InputEvent, InputEventInfo, TouchType};

#[derive(Clone, Copy)]
pub struct InputWithTimestamp {
    pub input: Input,
    pub timestamp_milliseconds: u32,
}

impl Display for InputWithTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:6} {}", self.timestamp_milliseconds, self.input)
    }
}

impl FromStr for InputWithTimestamp {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (first, second) = s.trim().split_once(' ').ok_or(())?;
        let timestamp_milliseconds = first.parse().map_err(|_| ())?;
        let input = second.trim().parse().map_err(|_| ())?;
        Ok(Self {
            input,
            timestamp_milliseconds,
        })
    }
}

#[derive(Clone, Copy)]
pub enum Input {
    Tap(Tap),
    Swipe(Swipe),
    Key(Key),
}

impl Display for Input {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Input::Tap(i) => write!(f, "{:6} {}", "tap", i),
            Input::Swipe(i) => write!(f, "{:6} {}", "swipe", i),
            Input::Key(i) => write!(f, "{:6} {}", "keyevent", i),
        }
    }
}

impl FromStr for Input {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (first, second) = s.trim().split_once(' ').ok_or(())?;

        let res = match first.trim() {
            "tap" => Self::Tap(second.parse()?),
            "swipe" => Self::Swipe(second.parse()?),
            "keyevent" => Self::Key(second.parse()?),
            _ => return Err(()),
        };
        Ok(res)
    }
}

#[derive(Clone, Copy)]
pub struct Tap {
    pub x: i32,
    pub y: i32,
}

impl Display for Tap {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:4} {:4}", self.x, self.y)
    }
}

impl FromStr for Tap {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (first, second) = s.trim().split_once(' ').ok_or(())?;
        Ok(Self {
            x: first.trim().parse().map_err(|_| ())?,
            y: second.trim().parse().map_err(|_| ())?,
        })
    }
}

#[derive(Clone, Copy)]
pub struct Swipe {
    pub x: [i32; 2],
    pub y: [i32; 2],
    pub milliseconds: u32,
}

impl Display for Swipe {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:4} {:4} {:4} {:4} {:4}",
            self.x[0], self.y[0], self.x[1], self.y[1], self.milliseconds
        )
    }
}

impl FromStr for Swipe {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.trim().split_ascii_whitespace();

        let x0 = iter.next().ok_or(())?.parse().map_err(|_| ())?;
        let y0 = iter.next().ok_or(())?.parse().map_err(|_| ())?;
        let x1 = iter.next().ok_or(())?.parse().map_err(|_| ())?;
        let y1 = iter.next().ok_or(())?.parse().map_err(|_| ())?;
        let ms = iter.next().ok_or(())?.parse().map_err(|_| ())?;

        Ok(Self {
            x: [x0, x1],
            y: [y0, y1],
            milliseconds: ms,
        })
    }
}

#[derive(Clone, Copy)]
pub enum Key {
    Power,
    Back,
    Home,
    Menu,
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Key::Power => write!(f, "KEYCODE_POWER"),
            Key::Back => write!(f, "KEYCODE_BACK"),
            Key::Home => write!(f, "KEYCODE_HOME"),
            Key::Menu => write!(f, "KEYCODE_MENU"),
        }
    }
}

impl FromStr for Key {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = match s.trim() {
            "KEYCODE_POWER" => Self::Power,
            "KEYCODE_BACK" => Self::Back,
            "KEYCODE_HOME" => Self::Home,
            "KEYCODE_MENU" => Self::Menu,
            _ => return Err(()),
        };

        Ok(res)
    }
}

pub fn convert_events_to_input(
    inputs: &[InputEventInfo],
    tap_threshold_distance : u32,
    tap_threshold_ms : u32,
) -> Vec<InputWithTimestamp> {
    struct DownInput {
        x : i32,
        y : i32,
        time : u32,
    }

    let first_time_stamp = match inputs.get(0) {
        Some(x) => x.timestamp_milliseconds,
        None => return Vec::new(),
    };

    let mut result = Vec::new();

    // we cann only track on finger, so only track touch input while slot 0 is active
    let mut is_slot_0_active = true;

    let mut down : Option<DownInput> = None;
    let mut last_x = 0;
    let mut last_y = 0;


    for e in inputs.iter() {
        let relative_time_stamp = e.timestamp_milliseconds - first_time_stamp;
        match e.event {
            InputEvent::AbsMtSlot(slot) => is_slot_0_active = slot == 0,
            InputEvent::AbsMtPosX(x) if is_slot_0_active => {
                last_x = x;
            }
            InputEvent::AbsMtPosY(y) if is_slot_0_active => {
                last_y = y;               
            }
            InputEvent::BtnTouch(t) if is_slot_0_active => match t {
                TouchType::Up => {
                    if let Some(d) = down.take() {

                        let distance_moved = (d.x).abs_diff(last_x) + (d.y).abs_diff(last_y);
                        let down_dur_ms = relative_time_stamp - d.time;

                        let is_swipe = distance_moved > tap_threshold_distance || down_dur_ms > tap_threshold_ms;

                        if is_swipe {
                            result.push(InputWithTimestamp {
                                timestamp_milliseconds: d.time,
                                input: Input::Swipe(Swipe {
                                    milliseconds: down_dur_ms,
                                    x: [d.x, last_x],
                                    y: [d.y, last_y],
                                }),
                            })
                        } else {
                            result.push(InputWithTimestamp {
                                timestamp_milliseconds: d.time,
                                input: Input::Tap(Tap { x : d.x, y : d.y, }),
                            })
                        }
                    }
                }
                TouchType::Down => {
                    down = Some(DownInput {x: last_x, y:last_y, time: relative_time_stamp});
                }
            },
            InputEvent::KeyPower(t) if t == TouchType::Down => {
                result.push(InputWithTimestamp {
                    timestamp_milliseconds: relative_time_stamp,
                    input: Input::Key(Key::Power),
                });
            }
            _ => (),
        }
    }

    result
}










pub fn serialize_inputs<T: std::io::Write>(inputs: &[InputWithTimestamp], writer: &mut T) {
    for i in inputs {
        writeln!(writer, "{i}").expect("should not fail");
    }
}

pub fn serialize_inputs_fmt<T: std::fmt::Write>(inputs: &[InputWithTimestamp], writer: &mut T) {
    for i in inputs {
        writeln!(writer, "{i}").expect("should not fail");
    }
}

pub fn deser_inputs_fmt<T: std::io::BufRead>(
    reader: &mut T,
) -> Result<Vec<InputWithTimestamp>, ()> {
    let mut res = Vec::new();
    let mut linebuf = String::new();
    loop {
        linebuf.clear();
        let len = reader.read_line(&mut linebuf).map_err(|_| ())?;

        if len == 0 {
            break;
        }

        res.push(linebuf.parse()?);
    }

    Ok(res)
}

pub struct InputStrings(pub Vec<String>);

impl InputStrings{
    pub fn from_inputs(inputs: &[InputWithTimestamp]) -> Self 
    {
        let mut res = Vec::new();

        for (index, input) in inputs.iter().enumerate() {
            res.push(format!("{:3} {}", index, input)); 
        }
        Self(res)
    }
}

