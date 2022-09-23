use std::process::Stdio;

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{ChildStdout, Command},
    sync::{
        mpsc::{
            self,
            error::{TryRecvError},
        },
        oneshot,
    },
};

use crate::{
    device_entry::DeviceEntry,
    input::{InputWithTimestamp, convert_events_to_input},
    input_event::InputEventInfo,
    input_event_parser::ParsedGetEventOutput,
};

use super::NO_WINDOW_FLAGS;

#[derive(Clone, Copy)]
pub enum ReadNextStatusError {
    Empty,
    Finished,
}

#[derive(Clone, Copy)]
pub enum GetResultError {
    NotYetAvailable,
    AlreadyReceived,
    InternalError,
}

#[derive(Clone, Copy)]
pub enum ReadEventsError {
    ProcessAborted,
    ParseError,
}

#[derive(Clone, Copy)]
pub enum StatusMessage {
    RecordedInput(InputEventInfo),
    RecordingFinished,
}

pub struct InputRecorder {
    status_recv: mpsc::UnboundedReceiver<StatusMessage>,
    process_kill_send: Option<oneshot::Sender<()>>,
    result_recv: Option<oneshot::Receiver<Option<Vec<InputWithTimestamp>>>>,
}

impl InputRecorder {
    pub fn new(
        gui_context: &egui::Context,     
        tap_threshold_distance : u32,
        tap_threshold_ms : u32,
    ) -> Self {
        let (process_kill_send, process_kill_recv) = oneshot::channel::<()>();
        let (result_send, result_recv) = oneshot::channel::<Option<Vec<InputWithTimestamp>>>();
        let (status_send, status_recv) = mpsc::unbounded_channel::<StatusMessage>();

        tokio::spawn(Self::start(
            gui_context.clone(),
            status_send,
            result_send,
            process_kill_recv,
            tap_threshold_distance,
            tap_threshold_ms
        ));

        Self {
            process_kill_send: Some(process_kill_send),
            status_recv,
            result_recv: Some(result_recv),
        }
    }

    pub fn stop(&mut self) {
        if let Some(killer) = self.process_kill_send.take() {
            if let Err(_) = killer.send(()) {
                println!("error sending kill: {}", "sender dropped");
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.process_kill_send.is_some()
    }

    pub fn read_next_status(&mut self) -> Result<StatusMessage, ReadNextStatusError> {
        match self.status_recv.try_recv() {
            Ok(status) => Ok(status),
            Err(TryRecvError::Empty) => Err(ReadNextStatusError::Empty),
            Err(TryRecvError::Disconnected) => Err(ReadNextStatusError::Finished),
        }
    }

    pub fn try_get_result(&mut self) -> Result<Vec<InputWithTimestamp>, GetResultError> {
        if let Some(recv) = &mut self.result_recv {
            let res = recv.try_recv().map_err(|err| match err {
                oneshot::error::TryRecvError::Empty => GetResultError::NotYetAvailable,
                oneshot::error::TryRecvError::Closed => GetResultError::InternalError,
            })?;

            match res {
                Some(r) => Ok(r),
                None => Err(GetResultError::InternalError),
            }
        } else {
            Err(GetResultError::AlreadyReceived)
        }
    }

    async fn start(
        gui_context: egui::Context,
        status_send: mpsc::UnboundedSender<StatusMessage>,
        result_send: oneshot::Sender<Option<Vec<InputWithTimestamp>>>,
        terminate: oneshot::Receiver<()>,
        tap_threshold_distance : u32,
        tap_threshold_ms : u32,
    ) {


        let mut child = Command::new("adb.exe")
            .arg("shell")
            .arg("getevent")
            .arg("-t")
            .arg("-l")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .creation_flags(NO_WINDOW_FLAGS)
            .spawn()
            .expect("adb must be installed");

        let child_output = child.stdout.take().unwrap();
        let child_error = child.stderr.take().unwrap();

        let join_handle_read_err = tokio::spawn(async move {
            let mut stdout_reader = BufReader::new(child_error);

            let mut line_buffer = String::new();
            loop {
                line_buffer.clear();
                match stdout_reader.read_line(&mut line_buffer).await {
                    Ok(len) => {
                        if len == 0 {
                            break;
                        }
                        eprintln!("stderr: {}", &line_buffer);
                    }
                    Err(err) => {
                        println!("stderr read error: {}", err.to_string());
                        break;
                    }
                }
            }
        });

        let join_handle_read_input = tokio::spawn(record_inputs_output(
            child_output,
            gui_context.clone(),
            status_send.clone(),
        ));

        if let Err(err) = terminate.await {
            println!("terminate error kill err:{}", err.to_string());
        }

        if let Err(err) = child.kill().await {
            println!("child kill err:{}", err.to_string());
        }

        if let Err(err) = join_handle_read_err.await {
            println!("join err, err:{}", err.to_string());
        }

        let device_entry_and_input_events = match join_handle_read_input.await {
            Err(err) => {
                eprintln!("failed joining {}", err.to_string());
                None
            }
            Ok(ok) => ok.ok(),
        };

        let inputs = device_entry_and_input_events.map(
            |e| convert_events_to_input(&e.1, tap_threshold_distance, tap_threshold_ms));

        if let Err(_) = result_send.send(inputs) {
            eprintln!("failed to send result");
        }

        if let Err(_) = status_send.send(StatusMessage::RecordingFinished) {
            eprintln!("failed to send RecordingFinished");
        }
        gui_context.request_repaint();
    }
}

impl Drop for InputRecorder {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn record_inputs_output(
    stdout: ChildStdout,
    gui_context: egui::Context,
    status_sender: mpsc::UnboundedSender<StatusMessage>,
) -> Result<(Vec<DeviceEntry>, Vec<InputEventInfo>), ReadEventsError> {
    let mut stdout_reader = BufReader::new(stdout);

    let mut line_buffer = String::new();

    let mut last_device_with_event = None;
    let mut devices = Vec::new();
    let mut inputs = Vec::new();

    loop {
        line_buffer.clear();
        let n_bytes_read = stdout_reader
            .read_line(&mut line_buffer)
            .await
            .map_err(|_| ReadEventsError::ProcessAborted)?;

        if n_bytes_read == 0 {
            break;
        }

        let maybe_parsed = ParsedGetEventOutput::try_from_str(&line_buffer)
            .ok()
            .flatten();

        if let Some(parsed) = maybe_parsed {
            match parsed {
                ParsedGetEventOutput::AddDevice(dwe) => last_device_with_event = Some(dwe),
                ParsedGetEventOutput::Name(name) => {
                    if let Some(dwe) = last_device_with_event.take() {
                        devices.push(DeviceEntry {
                            device_nr: dwe.device_nr,
                            event_nr: dwe.event_nr,
                            name: name.to_string(),
                        });
                    } else {
                        eprintln!("warning: igorning name no dwe present");
                    }
                }

                ParsedGetEventOutput::Input(input) => {
                    inputs.push(input);
                    if let Err(_) = status_sender.send(StatusMessage::RecordedInput(input)) {
                        println!("receiver dropped, stopping parsing");
                        break;
                    }
                    gui_context.request_repaint();
                }
            };
        }
    }

    Ok((devices, inputs))
}
