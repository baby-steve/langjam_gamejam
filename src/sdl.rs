use sdl2::{event::Event, rect::FRect};

use crate::vm::{Runtime, Value};

/// Register SDL related functions.
pub fn register_sdl_functions(runtime: &mut Runtime) {
    runtime.register_function("init_sdl", 0, |args| {
        let sdl = match sdl2::init() {
            Ok(sdl) => sdl,
            Err(err) => {
                todo!("{err} (need proper error handling)");
            }
        };

        let obj = args
            .heap
            .alloc_extern(sdl)
            .expect("bug: cannot alloc external object");
        Value::ExternObject(obj)
    });

    runtime.register_function("init_video", 1, |args| {
        let sdl = match args.stack.pop().unwrap() {
            Value::ExternObject(addr) => {
                let obj = args.heap.get_extern(addr).unwrap();
                obj.try_borrow::<sdl2::Sdl>().unwrap()
            }
            _ => todo!("expected external object"),
        };

        let video = sdl.video().unwrap();
        let obj = args.heap.alloc_extern(video).unwrap();
        Value::ExternObject(obj)
    });

    runtime.register_function("create_window", 4, |args| {
        let Value::Number(height) = args.stack.pop().unwrap() else {
            todo!("Not a number");
        };

        let Value::Number(width) = args.stack.pop().unwrap() else {
            todo!("Not a number");
        };

        let Value::String(title_addr) = args.stack.pop().unwrap() else {
            todo!("Not a string");
        };

        let video = match args.stack.pop().unwrap() {
            Value::ExternObject(addr) => {
                let obj = args.heap.get_extern(addr).unwrap();
                obj.try_borrow::<sdl2::VideoSubsystem>().unwrap()
            }
            _ => todo!("expected external object"),
        };

        let title = args.strings.get(title_addr);
        let window_res = video
            .window(&title, width as u32, height as u32)
            .position_centered()
            .build();

        match window_res {
            Ok(window) => {
                let obj = args.heap.alloc_extern(window).unwrap();
                Value::ExternObject(obj)
            }
            Err(_) => todo!("Failed to create window (need real errors)"),
        }
    });

    runtime.register_function("into_canvas", 1, |args| {
        let obj = match args.stack.pop().unwrap() {
            Value::ExternObject(addr) => args.heap.take_extern(addr),
            _ => todo!("expected external object"),
        };

        let window = obj.into_obj::<sdl2::video::Window>().unwrap();
        let canvas = window.into_canvas().build().unwrap();

        let obj = args.heap.alloc_extern(canvas).unwrap();
        Value::ExternObject(obj)
    });

    runtime.register_function("create_event_pump", 1, |args| {
        let sdl = match args.stack.pop().unwrap() {
            Value::ExternObject(addr) => {
                let obj = args.heap.get_extern(addr).unwrap();
                obj.try_borrow::<sdl2::Sdl>().unwrap()
            }
            _ => todo!("expected external object"),
        };

        let event_pump = sdl.event_pump().unwrap();
        let obj = args.heap.alloc_extern(event_pump).unwrap();
        Value::ExternObject(obj)
    });

    // Event pump functions.
    runtime.register_function("poll_event", 1, |mut args| {
        println!("Calling poll_event");

        let value = args.stack.pop().unwrap();
        let event_pump = match value {
            Value::ExternObject(addr) => {
                let obj = args.heap.get_extern_mut(addr).unwrap();
                obj.try_borrow_mut::<sdl2::EventPump>().unwrap()
            }
            _ => todo!("expected external object"),
        };

        let Some(event) = event_pump.poll_event() else {
            return Value::Nil;
        };

        match event {
            Event::KeyUp { keycode, .. } => {
                let Some(object_addr) = args.heap.alloc() else {
                    // Out of memory. Trigger a garbage collection cycle.
                    *args.needs_gc = true;
                    // Restore the stack's pre-call state to prevent bad things from
                    // happening when this function gets called again.
                    args.stack.push(value);
                    return Value::Nil;
                };

                let kind_id = args.field_id("kind");
                let kind_value = args.strings.intern("keyup".into());

                let keycode_id = args.field_id("keycode");
                let keycode_value = args.strings.intern(keycode.unwrap().to_string());

                let object = args.heap.get_mut(object_addr).unwrap();
                object.data.insert(kind_id, Value::String(kind_value));
                object.data.insert(keycode_id, Value::String(keycode_value));

                return Value::Object(object_addr);
            }

            // Ignore unsupported events.
            _ => return Value::Nil,
        }
    });

    // Canvas related functions.
    runtime.register_function("set_draw_color", 4, |args| {
        let b = args.stack.pop().unwrap().as_number() as u8;
        let g = args.stack.pop().unwrap().as_number() as u8;
        let r = args.stack.pop().unwrap().as_number() as u8;
        let canvas = match args.stack.pop().unwrap() {
            Value::ExternObject(addr) => {
                let obj = args.heap.get_extern_mut(addr).unwrap();
                obj.try_borrow_mut::<sdl2::render::Canvas<sdl2::video::Window>>()
                    .unwrap()
            }
            _ => todo!("expected external object"),
        };

        canvas.set_draw_color((r, g, b));

        Value::Nil
    });

    runtime.register_function("draw_rect", 5, |args| {
        let h = args.stack.pop().unwrap().as_number() as f32;
        let w = args.stack.pop().unwrap().as_number() as f32;
        let y = args.stack.pop().unwrap().as_number() as f32;
        let x = args.stack.pop().unwrap().as_number() as f32;
        let canvas = match args.stack.pop().unwrap() {
            Value::ExternObject(addr) => {
                let obj = args.heap.get_extern_mut(addr).unwrap();
                obj.try_borrow_mut::<sdl2::render::Canvas<sdl2::video::Window>>()
                    .unwrap()
            }
            _ => todo!("expected external object"),
        };

        canvas.draw_frect(FRect::new(x, y, w, h)).unwrap();

        Value::Nil
    });

    runtime.register_function("fill_rect", 5, |args| {
        let h = args.stack.pop().unwrap().as_number() as f32;
        let w = args.stack.pop().unwrap().as_number() as f32;
        let y = args.stack.pop().unwrap().as_number() as f32;
        let x = args.stack.pop().unwrap().as_number() as f32;
        let canvas = match args.stack.pop().unwrap() {
            Value::ExternObject(addr) => {
                let obj = args.heap.get_extern_mut(addr).unwrap();
                obj.try_borrow_mut::<sdl2::render::Canvas<sdl2::video::Window>>()
                    .unwrap()
            }
            _ => todo!("expected external object"),
        };

        canvas.fill_frect(FRect::new(x, y, w, h)).unwrap();

        Value::Nil
    });

    runtime.register_function("clear", 1, |args| {
        let canvas = match args.stack.pop().unwrap() {
            Value::ExternObject(addr) => {
                let obj = args.heap.get_extern_mut(addr).unwrap();
                obj.try_borrow_mut::<sdl2::render::Canvas<sdl2::video::Window>>()
                    .unwrap()
            }
            _ => todo!("expected external object"),
        };

        canvas.clear();

        Value::Nil
    });

    runtime.register_function("present", 1, |args| {
        let canvas = match args.stack.pop().unwrap() {
            Value::ExternObject(addr) => {
                let obj = args.heap.get_extern_mut(addr).unwrap();
                obj.try_borrow_mut::<sdl2::render::Canvas<sdl2::video::Window>>()
                    .unwrap()
            }
            _ => todo!("expected external object"),
        };

        canvas.present();

        Value::Nil
    });
}
