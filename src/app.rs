use std::sync::Arc;

use egui::{RichText};

use crate::{
    input::{InputWithTimestamp, InputStrings},
    input_event_recorder::{GetResultError, InputRecorder, ReadNextStatusError},
    input_player::{InputPlayer, InputReplayState, Repeating},
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AirApp {
    #[serde(skip)]
    input: Option<Arc<Vec<InputWithTimestamp>>>,

    #[serde(skip)]
    input_strings : Option<InputStrings>,

    #[serde(skip)]
    record_task: Option<InputRecorder>,

    #[serde(skip)]
    play_task: Option<InputPlayer>,

    tap_threshold_ms : u32,
    tap_threshold_distance : u32,
    delay_ms_between_loops : u32,
}

impl Default for AirApp {
    fn default() -> Self {
        Self {
            input: Default::default(),
            record_task: Default::default(),
            input_strings: Default::default(),
            play_task: Default::default(),
            tap_threshold_distance : 100,
            tap_threshold_ms : 500,
            delay_ms_between_loops : 200,
        }
    }
}

impl AirApp {
    const KEY : &'static str = "android_input_replayer";
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, Self::KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn draw_main(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(recorder) = &mut self.record_task {
            match Self::handle_recording(recorder, ctx, ui, _frame) {
                Err(_) => self.record_task = None,
                Ok(None) => (),
                Ok(Some(res)) => {
                    self.input_strings = Some(InputStrings::from_inputs(&res));
                    self.input = Some(Arc::new(res));
                    self.record_task = None;
                }
            };
        } else {
            if ui.button("Start Recording").clicked() {
                self.record_task = Some(InputRecorder::new(ctx, self.tap_threshold_distance, self.tap_threshold_ms));
                self.input = None;
            }
        }

        if let Some(player) = &mut self.play_task {
            if player.is_running() {
                if ui.button("Stop Playing").clicked() {
                    player.stop();
                    self.play_task = None;
                }
            }
        } else if let Some(input) = &self.input {
            if ui.button("Play Recording").clicked() {
                self.play_task = Some(InputPlayer::new(ctx, input.clone(), self.delay_ms_between_loops));
            }
        }
    }

    fn draw_settings(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {

        ui.vertical(|ui| {

            ui.add(egui::Slider::new(&mut self.tap_threshold_distance, 0..=1000).text("Max tap distance"))
                .on_hover_text_at_pointer("The distance between touch down and touch up must be lower than this to count as tap. Otherwise it is a swipe")
            ;

            ui.add(egui::Slider::new(&mut self.tap_threshold_ms, 0..=1000).text("Max tap MS"))
                .on_hover_text_at_pointer("The time in milliseconds between touch down and touch up must be lower than this to count as tap. Otherwise it is a swipe")
            ;

            ui.add(egui::Slider::new(&mut self.delay_ms_between_loops, 0..=10000).text("MS between loops"))
                .on_hover_text_at_pointer("The app waits this many milliconds between each repetition of the recorded inputs")
            ;
        });       
    }

    fn draw_input_strings(input_strings : &InputStrings, replay_state : Option<InputReplayState>, _ctx: &egui::Context, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        for (i, s) in input_strings.0.iter().enumerate()
        {
            let is_current = replay_state.map(
                |s| match s {
                    InputReplayState::Repeating(Repeating { repetion : _, reptetion_element: Some(idx)  }) => idx == i,
                    _ => false,
                }
            ).unwrap_or(false);

            if is_current {
                ui.add(egui::Label::new(RichText::new(s).strong().monospace())).scroll_to_me(None); 
            } else {
                ui.add(egui::Label::new(RichText::new(s).monospace())); 
            }
        }

        let is_end = replay_state.map(
            |s| match s {
                InputReplayState::Repeating(Repeating { repetion : _, reptetion_element: None  }) => true,
                _ => false,
            }
        ).unwrap_or(false);

        if is_end {
            ui.add(egui::Label::new(RichText::new("END OF INPUTS").strong().monospace())).scroll_to_me(None);
        } else {
            ui.add(egui::Label::new(RichText::new("END OF INPUTS").monospace())); 
        }
    }

    // handles recording, if finsihed return it's result, return error if something bad happend and the recordder should be destroyed
    fn handle_recording(
        recorder: &mut InputRecorder,
        _ctx: &egui::Context,
        ui: &mut egui::Ui,
        _frame: &mut eframe::Frame,
    ) -> Result<Option<Vec<InputWithTimestamp>>, ()> {
        loop {
            match recorder.read_next_status() {
                Ok(_) => (), // TODO DISPLAY
                Err(ReadNextStatusError::Empty) => break,
                Err(ReadNextStatusError::Finished) => break,
            }
        }

        if recorder.is_running() {
            if ui.button("Stop Recording").clicked() {
                recorder.stop();
            }
            Ok(None)
        } else {
            ui.label("Stopping Recording ...");
            match recorder.try_get_result() {
                Err(GetResultError::NotYetAvailable) => Ok(None),
                Err(GetResultError::AlreadyReceived) => {
                    eprint!("Querying alreay received task");
                    Err(())
                }
                Err(GetResultError::InternalError) => Err(()),
                Ok(res) => {
                    return Ok(Some(res));
                }
            }
        }
    }

    fn draw_gui_infos(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut egui::Ui,
        _frame: &mut eframe::Frame,
    ) {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {


            ui.hyperlink_to("Source code", "https://github.com/nebulark/android_input_replayer");

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("GUI created with ");
                ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                ui.label(" and ");
                ui.hyperlink_to(
                    "eframe",
                    "https://github.com/emilk/egui/tree/master/crates/eframe",
                );
                ui.label(".");
            });

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Made with ");
                ui.hyperlink_to("Rust", "https://www.rust-lang.org/");
            });


            egui::warn_if_debug_build(ui);
        });
    }
}

impl eframe::App for AirApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, Self::KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("side_panel").resizable(true).show(ctx, |ui| {
            self.draw_settings(ctx, ui, _frame);
            self.draw_gui_infos(ctx, ui, _frame);
        });

        if let Some(input_strings) = &self.input_strings
        {
            let replay_status = self.play_task.as_ref().map(|t|t.get_current_status());

            egui::TopBottomPanel::bottom("bottom_panel").resizable(true).show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui|{
                Self::draw_input_strings(input_strings, replay_status, ctx, ui, _frame);
                });
            });
        }        

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_main(ctx, ui, _frame);
        });
    }
}
