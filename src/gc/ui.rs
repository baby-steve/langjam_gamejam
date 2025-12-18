use egui_sdl2::egui;

use crate::vm::Value;

pub fn draw_object_field(ui: &mut egui::Ui, value: Value) {
    let as_u64 = value.to_u64();
    let bits = as_u64.to_le_bytes();
    let as_u32s = [
        u32::from_le_bytes([bits[0], bits[1], bits[2], bits[3]]),
        u32::from_le_bytes([bits[4], bits[5], bits[6], bits[7]]),
    ];

    let as_u16s = [
        u16::from_le_bytes([bits[0], bits[1]]),
        u16::from_le_bytes([bits[2], bits[3]]),
        u16::from_le_bytes([bits[4], bits[5]]),
        u16::from_le_bytes([bits[6], bits[7]]),
    ];

    for (n, bit) in bits.iter().enumerate() {
        ui.label(egui::RichText::new(format!("{:0>2x}", bit)).color(egui::Color32::LIGHT_GRAY))
            .on_hover_ui(|ui| {
                ui.set_width(300.0);
                ui.columns_const(|[col_1, col_2]| {
                    col_1.label("integer 8-bit");
                    col_2.label(bit.to_string());

                    let as_u16 = as_u16s[(0.125 * n as f32).floor() as usize];
                    col_1.label("integer 16-bit");
                    col_2.label(as_u16.to_string());

                    let as_u32 = as_u32s[(0.25 * n as f32).floor() as usize];
                    col_1.label("integer 32-bit");
                    col_2.label(as_u32.to_string());

                    col_1.label("integer 64-bit");
                    col_2.label(as_u64.to_string());

                    col_1.label("float 64-bit");
                    col_2.label(f64::from_bits(as_u64).to_string());
                });
            });
    }
}

pub fn freeing_garbage(ctx: &egui::Context, elapsed: f64, total_time: f64, pause: f64) {
    egui::Window::new("Collecting Garbage")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fade_in(true)
        .title_bar(false)
        .show(ctx, |ui| {
            let done = if (elapsed + pause) >= total_time {
                "DONE!"
            } else {
                ""
            };

            ui.label(
                egui::RichText::new(format!("Freeing garbage... {done}"))
                    .strong()
                    .size(15.0),
            );

            ui.set_width(500.0);

            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let elapsed = elapsed.clamp(0.0, total_time - pause);
                    let cells = 59.0;
                    let percent_complete = elapsed / total_time;
                    let lhs = cells * percent_complete;

                    ui.label(
                        egui::RichText::new("♥".repeat(lhs as usize))
                            .color(egui::Color32::GREEN)
                            .monospace(),
                    );

                    ui.add_space(ui.available_width());
                });
            });
        });
}
