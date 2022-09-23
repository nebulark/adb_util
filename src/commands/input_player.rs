use std::{fmt::Write, process::Stdio, sync::Arc, time::Duration};

use tokio::{
    process::Command,
    sync::{oneshot::{self, error::TryRecvError}, watch},
};

use crate::input::{InputWithTimestamp, Input};

use super::NO_WINDOW_FLAGS;

pub struct InputPlayer {
    stop_send: Option<oneshot::Sender<()>>,
    status_recv: watch::Receiver<InputReplayState>,
}

#[derive(Clone, Copy, Debug)]
pub enum InputReplayState {
    NotStarted,
    Repeating(Repeating),
    Finished
}

#[derive(Clone, Copy, Debug)]
pub struct Repeating {
    pub repetion : u32,
    pub reptetion_element : Option<usize>
}

impl InputPlayer {
    pub fn new(gui_context: &egui::Context, inputs: Arc<Vec<InputWithTimestamp>>, delay_ms_between_loops : u32) -> Self {
        let (stop_send, mut stop_recv) = oneshot::channel::<()>();
        let (status_send, status_recv) = watch::channel::<InputReplayState>(InputReplayState::NotStarted);

        let gui_context_async = gui_context.clone();
        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut repetion = 0;
            'main_loop: loop {
                let mut last_millis = 0;

                for (idx, input) in inputs.iter().enumerate() {
                    match stop_recv.try_recv() {
                        Ok(_) | Err(TryRecvError::Closed) => break 'main_loop,
                        Err(TryRecvError::Empty) => (),
                    }

                    let diff = input.timestamp_milliseconds - last_millis;
                    last_millis = input.timestamp_milliseconds;

                    if diff > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(diff as u64)).await;
                    }

                    buffer.clear();
                    write!(buffer, "{}", input.input).unwrap();

                    _ = status_send.send(InputReplayState::Repeating(Repeating { repetion, reptetion_element: Some(idx) }));
                    gui_context_async.request_repaint();


                    Command::new("adb.exe")
                        .stdin(Stdio::null())
                        .arg("shell")
                        .arg("input")
                        .args(buffer.split_ascii_whitespace())
                        .creation_flags(NO_WINDOW_FLAGS)
                        .spawn()
                        .expect("adb must be installed");
                }

                // input sequence finished

                if let Some(InputWithTimestamp { input : Input::Swipe(s), timestamp_milliseconds: _}) = inputs.last()
                {
                    tokio::time::sleep(Duration::from_millis(s.milliseconds as u64)).await;
                }

                _ = status_send.send(InputReplayState::Repeating(Repeating { repetion, reptetion_element: None }));
                gui_context_async.request_repaint();

                tokio::time::sleep(Duration::from_millis(delay_ms_between_loops as u64)).await;
                repetion += 1;
            }

            if let Err(_e) = status_send.send(InputReplayState::Finished) {
                eprintln!("error confirming stop: {}", "receiver dropped");
            }
            gui_context_async.request_repaint();
        });

        Self {
            stop_send: Some(stop_send),
            status_recv,
        }
    }

    pub fn stop(&mut self) {
        if let Some(stop_send) = self.stop_send.take() {
            if let Err(_) = stop_send.send(()) {
                eprintln!("error sending stop: {}", "sender dropped");
            }
        }
    }

    pub fn get_current_status(&self) -> InputReplayState {
        self.status_recv.borrow().clone()
    }

    pub fn is_running(&self) -> bool {
        self.stop_send.is_some()
    }
}

impl Drop for InputPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
