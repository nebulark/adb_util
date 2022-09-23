use std::str::FromStr;

use crate::input_event::{InputEvent, InputEventInfo, TouchType};

#[derive(Clone, Copy)]
pub struct DeviceWithEvent {
    pub device_nr: i32,
    pub event_nr: i32,
}

#[derive(Clone, Copy)]
pub enum ParsedGetEventOutput<'a> {
    AddDevice(DeviceWithEvent),
    Name(&'a str),
    Input(InputEventInfo),
}

impl<'a> ParsedGetEventOutput<'a> {
    fn parse_input_event_name(event_str: &str) -> Result<i32, ()> {
        // example: /dev/input/event3
        let mut ev_toks = event_str.split('/');
        if !ev_toks.next().ok_or(())?.is_empty() {
            return Err(());
        }
        if ev_toks.next().ok_or(())? != "dev" {
            return Err(());
        }
        if ev_toks.next().ok_or(())? != "input" {
            return Err(());
        }
        ev_toks
            .next()
            .ok_or(())?
            .trim_start_matches("event")
            .parse()
            .map_err(|_| ())
    }

    pub fn try_from_str(s: &'a str) -> Result<Option<Self>, ()> {
        let mut tokens = s.split_ascii_whitespace();
        let parsed = match tokens.next() {
            Some("add") => {
                // add device 2: /dev/input/event6
                if tokens.next().ok_or(())? != "device" {
                    return Err(());
                }
                let device_nr = tokens
                    .next()
                    .ok_or(())?
                    .trim_end_matches(':')
                    .parse()
                    .map_err(|_| ())?;
                let event_nr: i32 = Self::parse_input_event_name(tokens.next().ok_or(())?)?;
                Some(ParsedGetEventOutput::AddDevice(DeviceWithEvent {
                    device_nr,
                    event_nr,
                }))
            }
            Some("name:") => {
                // name:     "sec_touchscreen"
                let name = tokens.next().ok_or(())?.trim_matches('"');
                Some(ParsedGetEventOutput::Name(name))
            }
            Some("[") => {
                // [   31012.092121] /dev/input/event3: EV_ABS       ABS_MT_TOUCH_MINOR   00000005
                // [   31012.092121] /dev/input/event3: EV_KEY       BTN_TOUCH            DOWN
                // [   31012.092121] /dev/input/event3: EV_SYN       SYN_REPORT           00000000
                // [   31012.174212] /dev/input/event3: EV_ABS       ABS_MT_TOUCH_MAJOR   00000005
                let timestamp_seconds: f64 = tokens
                    .next()
                    .ok_or(())?
                    .trim_end_matches(']')
                    .parse()
                    .map_err(|_| ())?;

                let event_nr: i32 =
                    Self::parse_input_event_name(tokens.next().ok_or(())?.trim_end_matches(":"))?;
                let maybe_event = parse_input_event_name(
                    tokens.next().ok_or(())?,
                    tokens.next().ok_or(())?,
                    tokens.next().ok_or(())?,
                )?;

                maybe_event.map(|ev| {
                    ParsedGetEventOutput::Input(InputEventInfo {
                        timestamp_milliseconds: (timestamp_seconds * 1000.0).floor() as u32,
                        event_nr,
                        event: ev,
                    })
                })
            }
            _ => None,
        };

        Ok(parsed)
    }
}

impl FromStr for TouchType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DOWN" => Ok(TouchType::Down),
            "UP" => Ok(TouchType::Up),
            _ => Err(()),
        }
    }
}

fn parse_input_event_name(
    _ev_type: &str,
    ev_sub_type: &str,
    ev_value: &str,
) -> Result<Option<InputEvent>, ()> {
    // EV_ABS       ABS_MT_SLOT          00000000
    // EV_ABS       ABS_MT_TRACKING_ID   0000013e
    // EV_ABS       ABS_MT_POSITION_X    000001de
    let parsed = match ev_sub_type {
        "BTN_TOUCH" => Some(InputEvent::BtnTouch(TouchType::from_str(ev_value)?)),
        "ABS_MT_TRACKING_ID" => Some(InputEvent::AbsMtTrackingId(parse_hex_i32(ev_value)?)),
        "ABS_MT_SLOT" => Some(InputEvent::AbsMtSlot(parse_hex_i32(ev_value)?)),
        "ABS_MT_POSITION_X" => Some(InputEvent::AbsMtPosX(parse_hex_i32(ev_value)?)),
        "ABS_MT_POSITION_Y" => Some(InputEvent::AbsMtPosY(parse_hex_i32(ev_value)?)),
        "KEY_POWER" => Some(InputEvent::KeyPower(TouchType::from_str(ev_value)?)),
        _ => {
            // println!("not implemented: {} {} {}", _ev_type, ev_sub_type, ev_value);
            None
        }
    };

    Ok(parsed)
}

fn parse_hex_i32(s: &str) -> Result<i32, ()> {
    let u = u32::from_str_radix(s, 16).map_err(|_| ())?;
    Ok(u as i32)
}
