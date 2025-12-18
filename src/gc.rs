//! _Nuclear Alabaster Chainsaw_ utilizes a state of the art strategy for automatically
//! managing it's memory at runtime: **YOU** 🫵 (Yes, we consider you to be state of
//! the art. You should feel special).
//!

use std::time::{Duration, Instant};
use egui_sdl2::egui;
use sdl2::event::{Event, WindowEvent};
use crate::vm::{ExternObject, Heap, HeapValue, Object, Runtime};

mod ui;

const FULL_TITLE: &'static str = r#" _   _            _
| \ | |_   _  ___| | ___  __ _ _ __
|  \| | | | |/ __| |/ _ \/ _` | '__|
| |\  | |_| | (__| |  __/ (_| | |
|_| \_|\__,_|\___|_|\___|\__,_|_|  _
     / \  | | __ _| |__   __ _ ___| |_ ___ _ __
    / _ \ | |/ _` | '_ \ / _` / __| __/ _ \ '__|
   / ___ \| | (_| | |_) | (_| \__ \ ||  __/ |
  /_/ __\_\_|\__,_|_.__/ \__,_|___/\__\___|_|
     / ___| |__   __ _(_)_ __  ___  __ ___      __
    | |   | '_ \ / _` | | '_ \/ __|/ _` \ \ /\ / /
    | |___| | | | (_| | | | | \__ \ (_| |\ V  V /
     \____|_| |_|\__,_|_|_| |_|___/\__,_| \_/\_/
"#;

pub fn gc_app(runtime: &mut Runtime) {
    // Look for an instance of an SDL context in the runtime's globals.
    let sdl = runtime
        .global_values()
        .filter_map(|value| value.try_as_extern())
        .filter_map(|addr| runtime.heap.get_extern(addr))
        .find_map(|obj| obj.try_borrow::<sdl2::Sdl>())
        .cloned()
        .unwrap_or_else(|| sdl2::init().expect("failed to initialize SDL context"));

    let video = runtime
        .global_values()
        .filter_map(|value| value.try_as_extern())
        .filter_map(|addr| runtime.heap.get_extern(addr))
        .find_map(|obj| obj.try_borrow::<sdl2::VideoSubsystem>())
        .cloned()
        .unwrap_or_else(|| sdl.video().expect("failed to get video subsystem for SDL"));

    let window = video
        .window("Nuclear Alabaster Chainsaw - GC", 800, 600)
        .build()
        .expect("failed to create window");

    let (mut event_pump, addr) = runtime
        .globals
        .iter()
        .filter_map(|value| value.try_as_extern())
        .filter_map(|addr| runtime.heap.try_take_extern(addr).zip(Some(addr)))
        .find_map(|(obj, addr)| obj.into_obj::<sdl2::EventPump>().zip(Some(addr)))
        .unwrap_or_else(|| {
            dbg!("creating new event pump");
            (
                Box::new(
                    sdl.event_pump()
                        .expect("failed to create event pump for SDL"),
                ),
                u32::MAX,
            )
        });

    let mut app = GcApp::new(window, runtime);

    while app.running {
        for event in event_pump.poll_iter() {
            app.handle_event(&event);
        }

        app.update();
        std::thread::sleep(Duration::from_secs_f64(1.0 / 30.0));
    }

    app.shutdown();

    if addr != u32::MAX {
        runtime.heap.insert(addr, *event_pump);
    }
}

pub struct GcMetrics {
    pub total_cycles: usize,
    pub total_garbage_collected: usize,
}

impl Default for GcMetrics {
    fn default() -> Self {
        Self {
            total_cycles: 0,
            total_garbage_collected: 0,
        }
    }
}

/// `egui` application allowing the user/player to manage _Nuclear Alabaster
/// Chainsaw_'s heap.
pub struct GcApp<'r> {
    egui: egui_sdl2::EguiCanvas,
    heap: &'r mut Heap,
    running: bool,
    sweeping: bool,
    active_object: usize,
    marked: Vec<bool>,
    metrics: &'r mut GcMetrics,
    sweep_time: Instant,
}

impl<'r> GcApp<'r> {
    /// Create a new garbage collection application. This will attempt to reuse an existing SDL
    /// context if the runtime has already created one. If it can't find one, then it'll initialize
    /// a new one.  
    pub fn new(window: sdl2::video::Window, runtime: &'r mut Runtime) -> Self {
        let egui = egui_sdl2::EguiCanvas::new(window);
        let size = runtime.heap.size();

        Self {
            egui,
            heap: &mut runtime.heap,
            running: true,
            active_object: 0,
            marked: vec![false; size],
            metrics: &mut runtime.gc_metrics,
            sweeping: false,
            sweep_time: Instant::now(),
        }
    }

    pub fn shutdown(&mut self) {
        self.egui.destroy();
    }

    pub fn handle_event(&mut self, event: &Event) {
        let res = self.egui.on_event(event);

        if !res.consumed {
            match event {
                Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                } => {
                    self.running = false;
                }
                _ => {}
            }
        }
    }

    pub fn update(&mut self) {
        self.egui.run(|ctx| {
            if self.sweeping {
                let total_time = 1.6; // 2 seconds.
                let time = self.sweep_time.elapsed().as_secs_f64();
                ui::freeing_garbage(ctx, time, total_time, 0.6);

                if time >= total_time {
                    self.running = false;
                }
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                if self.sweeping {
                    ui.disable();
                }

                ui.horizontal(|ui| {
                    egui::Frame::group(ui.style())
                        .corner_radius(0)
                        .show(ui, |ui| {
                            ui.vertical(|ui: &mut egui::Ui| {
                                for (addr, entry) in self.heap.objects().enumerate() {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .radio(addr == self.active_object, "")
                                            .on_hover_text("View object")
                                            .clicked()
                                        {
                                            self.active_object = addr;
                                        }

                                        ui.checkbox(&mut self.marked[addr], "")
                                            .on_hover_text("Mark this object as not garbage");

                                        ui.label(
                                            egui::RichText::new(format!("0x{:0>6x}", addr))
                                                .color(egui::Color32::WHITE),
                                        );

                                        let value = match entry {
                                            HeapValue::Free { next } => *next,
                                            HeapValue::Object(object) => {
                                                (object as *const Object).addr()
                                            }
                                            HeapValue::Extern(object) => {
                                                (object as *const ExternObject).addr()
                                            }
                                        };

                                        let color = if self.marked[addr] {
                                            egui::Color32::YELLOW
                                        } else {
                                            egui::Color32::LIGHT_GRAY
                                        };

                                        ui.label(
                                            egui::RichText::new(format!("0x{:0>6x}", value))
                                                .color(color),
                                        );
                                    });
                                }

                                ui.horizontal(|ui| {
                                    ui.add_space(ui.available_height());
                                });
                            });
                        });

                    egui::Frame::group(ui.style())
                        .corner_radius(0)
                        .show(ui, |ui| {
                            ui.separator();

                            ui.vertical(|ui| {
                                ui.set_width(180.0);

                                ui.label("Contents");
                                ui.separator();

                                let object =
                                    self.heap.objects().nth(self.active_object).expect("bug");
                                match object {
                                    HeapValue::Free { .. } => {
                                        ui.label(
                                            egui::RichText::new("<the void>")
                                                .color(egui::Color32::LIGHT_GRAY),
                                        );
                                    }
                                    HeapValue::Object(object) => {
                                        ui.vertical(|ui| {
                                            for (_field_id, value) in object.data.iter() {
                                                ui.horizontal(|ui| {
                                                    ui::draw_object_field(ui, *value);
                                                });
                                            }
                                        });
                                    }
                                    HeapValue::Extern(extern_object) => {
                                        ui.horizontal(|ui| {
                                            for bit in extern_object.value_addr().to_le_bytes() {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{:0>2x}",
                                                        bit
                                                    ))
                                                    .color(egui::Color32::LIGHT_GRAY),
                                                );
                                            }
                                        });  
                                    },
                                }
                            });
                        });

                    ui.vertical(|ui| {
                        egui::Frame::group(ui.style())
                            .corner_radius(0)
                            .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("Instructions").heading());
                                    ui.separator();
                                    ui.label("♥ Select the objects you'd like to keep.");
                                    ui.label("♥ Unselected objects are assumed to be garbage and will be freed at the end of this cycle.");
                                    ui.label("♥ Carefully inspect the contents of each object to sus out if they can be freed");
                                    ui.label("♥ Worst case scenario, the program will SegFault.");
                                    ui.label("♥ Click the finish cycle button to resume the program.");

                                    if ui.button("Finish Cycle").clicked() {
                                        println!("Finishing the GC cycle");
                                        self.sweeping = true;
                                        self.metrics.total_garbage_collected += self.heap.sweep(&self.marked);
                                        self.sweep_time = Instant::now();
                                    }
                                });
                            });

                        egui::Frame::group(ui.style())
                            .corner_radius(0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("GC Stats").heading());
                                ui.separator();
                                ui.label(format!("Cycles survived: {}", self.metrics.total_cycles));
                                ui.label(format!("Total garbage collected: {}", self.metrics.total_garbage_collected));
                            });

                        egui::Frame::group(ui.style())
                            .corner_radius(0)
                            .show(ui, |ui| {
                                ui.separator();
                                ui.label(egui::RichText::new(FULL_TITLE)
                                    .size(10.0)
                                    .monospace()
                                    .background_color(egui::Color32::TRANSPARENT).color(egui::Color32::GREEN));
                            });
                    });
                });
            });
        });

        self.egui.clear([255, 255, 255, 255]);
        self.egui.paint();
        self.egui.present();
    }
}
