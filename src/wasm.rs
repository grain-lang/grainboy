use anyhow::Result;

struct HostState {
    pub quads: Vec<crate::gpu::QuadVertex>,
}
impl HostState {
    pub fn new() -> Self {
        Self { quads: vec![] }
    }
}

#[allow(unused)]
pub struct App {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub module_filepath: Option<String>,
    store: wasmtime::Store<HostState>,
    instance: wasmtime::Instance,
    run: wasmtime::TypedFunc<(), ()>,
    start: Option<wasmtime::TypedFunc<(), ()>>,
}
#[allow(unused)]
impl App {
    pub fn from_binary(bin: &[u8]) -> Result<Self> {
        let engine: wasmtime::Engine = create_engine();
        let mut store = wasmtime::Store::new(&engine, HostState::new());
        let module = wasmtime::Module::from_binary(store.engine(), bin)?;
        let instance = create_runtime(&mut store, module)?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        let mut start = None;
        if let Ok(func) = instance.get_typed_func::<(), ()>(&mut store, "_start") {
            start.insert(func);
        }
        Ok(Self {
            created_at: chrono::Utc::now(),
            module_filepath: None,
            store,
            instance,
            run,
            start,
        })
    }
    pub fn from_file(file: &str) -> Result<Self> {
        let engine: wasmtime::Engine = create_engine();
        let mut store = wasmtime::Store::new(&engine, HostState::new());
        let module = wasmtime::Module::from_file(store.engine(), file)?;
        let instance = create_runtime(&mut store, module)?;
        let mut start = None;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        if let Ok(func) = instance.get_typed_func::<(), ()>(&mut store, "_start") {
            start.insert(func);
        }
        Ok(Self {
            created_at: chrono::Utc::now(),
            module_filepath: Some(file.to_string()),
            store,
            instance,
            run,
            start,
        })
    }
    pub fn update_input(&mut self, p1_input: crate::input::UserInput) {
        if let Some(ptr) = self.instance.get_global(&mut self.store, "GRAINBOY_INPUT") {
            let ptr = ptr.get(&mut self.store).i32().unwrap() as usize;
            if let Some(mem) = self.instance.get_memory(&mut self.store, "memory") {
                let mem = mem.data_mut(&mut self.store);
                let p1_input: [u8; std::mem::size_of::<crate::input::UserInput>()] =
                    bytemuck::cast(p1_input);
                let mut i = ptr;
                for byte in p1_input {
                    mem[i] = byte;
                    i += 1;
                }
            } else {
                println!("Couldn't get memory")
            }
        } else {
            println!("Couldn't get GRAINBOY_INPUT")
        }
    }
    pub fn run(&mut self) -> Result<()> {
        if let Some(start) = self.start {
            match start.call(&mut self.store, ()) {
                Ok(_) => self.start = None,
                Err(err) => println!("{:?}", err),
            }
        }
        self.run.call(&mut self.store, ())
    }
    pub fn read_vertex_data(&self, cb: impl FnOnce(&[u8]) -> ()) {
        cb(&bytemuck::cast_slice(&self.store.data().quads))
    }
    pub fn clear_vertex_data(&mut self) {
        self.store.data_mut().quads.clear();
    }
}

#[allow(unused)]
fn create_engine() -> wasmtime::Engine {
    let mut config: wasmtime::Config = wasmtime::Config::new();
    config.wasm_tail_call(true);
    let engine: wasmtime::Engine = wasmtime::Engine::new(&config).unwrap();
    engine
}

#[allow(unused)]
fn create_runtime(
    store: &mut wasmtime::Store<HostState>,
    module: wasmtime::Module,
) -> Result<wasmtime::Instance> {
    let mut linker = wasmtime::Linker::new(store.engine());
    // ------------------------------------------------------------------------------------
    // wasi_snapshot_preview1::fd_write(a: i32, b: i32, c: i32, d: i32): i32
    // ------------------------------------------------------------------------------------
    linker.func_wrap("wasi_snapshot_preview1", "fd_write", {
        |mut caller: wasmtime::Caller<'_, HostState>, _a: i32, _b: i32, _c: i32, _d: i32| 0
    })?;
    // ------------------------------------------------------------------------------------
    // grainboy::log(ptr: u32, len: u32)
    // ------------------------------------------------------------------------------------
    linker.func_wrap("grainboy", "log", {
        |mut caller: wasmtime::Caller<'_, HostState>, ptr: u32, len: u32| {
            println!("Grainboy::debug::log -> {} {}", ptr, len);
            let mem = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => anyhow::bail!("failed to find host memory"),
            };
            let data = mem
                .data(&caller)
                .get(ptr as usize..)
                .and_then(|arr| arr.get(..len as u32 as usize));
            let string = match data {
                Some(data) => match std::str::from_utf8(data) {
                    Ok(s) => s,
                    Err(_) => anyhow::bail!("invalid utf-8"),
                },
                None => anyhow::bail!("pointer/length out of bounds"),
            };
            println!(
                "[Grainboy ({})]: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                string
            );
            Ok(())
        }
    })?;
    // ------------------------------------------------------------------------------------
    // grainboy::draw_rect(x: i32, y: i32, w: u32: h: u32, fill: u32)
    // ------------------------------------------------------------------------------------
    linker.func_wrap("grainboy", "draw_rect", {
        |mut caller: wasmtime::Caller<'_, HostState>, x: i32, y: i32, w: u32, h: u32, fill: u32| {
            use crate::gpu::QuadVertex;
            let state = caller.data_mut();
            let mut quad = QuadVertex::new([x as f32, y as f32, w as f32, h as f32]);
            quad.fill = fill;
            state.quads.push(quad);
            Ok(())
        }
    })?;
    // ------------------------------------------------------------------------------------
    // grainboy::draw_circ(x: i32, y: i32, diameter: u32, fill: u32)
    // ------------------------------------------------------------------------------------
    linker.func_wrap("grainboy", "draw_circ", {
        |mut caller: wasmtime::Caller<'_, HostState>, x: i32, y: i32, diameter: u32, fill: u32| {
            use crate::gpu::QuadVertex;
            let state = caller.data_mut();
            let mut quad = QuadVertex::new([x as f32, y as f32, diameter as f32, diameter as f32])
                .border_radius([(diameter, diameter); 4]);
            quad.fill = fill;
            state.quads.push(quad);
            Ok(())
        }
    })?;
    // ------------------------------------------------------------------------------------
    // grainboy::draw_sprite(x: i32, y: i32, w: u32, h: u32, sx: i32, sy: i32)
    // ------------------------------------------------------------------------------------
    linker.func_wrap("grainboy", "draw_sprite", {
        |mut caller: wasmtime::Caller<'_, HostState>,
         x: i32,
         y: i32,
         w: u32,
         h: u32,
         sx: u32,
         sy: u32| {
            use crate::gpu::QuadVertex;
            let state = caller.data_mut();
            let mut quad = QuadVertex::new([x as f32, y as f32, w as f32, h as f32]).tex_rect([
                sx as f32,
                (128 + sy) as f32,
                w as f32,
                h as f32,
            ]);
            state.quads.push(quad);
            Ok(())
        }
    })?;
    // ------------------------------------------------------------------------------------
    // grainboy::draw_text(x: i32, y: i32, font: u32, color: u32, text_ptr: u32, text_len: u32)
    // ------------------------------------------------------------------------------------
    linker.func_wrap("grainboy", "draw_text", {
        |mut caller: wasmtime::Caller<'_, HostState>,
         x: i32,
         y: i32,
         font: u32,
         color: u32,
         ptr: u32,
         len: u32| {
            let mem = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => anyhow::bail!("failed to find host memory"),
            };
            let data = mem
                .data(&caller)
                .get(ptr as usize..)
                .and_then(|arr| arr.get(..len as u32 as usize));
            let text = match data {
                Some(data) => match std::str::from_utf8(data) {
                    Ok(s) => s.to_string(),
                    Err(_) => anyhow::bail!("invalid utf-8"),
                },
                None => anyhow::bail!("pointer/length out of bounds"),
            };
            let state = caller.data_mut();
            let mut left = x;
            let mut top = y;
            for c in text.chars() {
                let (sx, sy, sw, sh) = get_glyph_coords(font as u8, c);
                if c == '\n' {
                    left = x;
                    top += sh as i32;
                } else {
                    use crate::gpu::QuadVertex;
                    let mut quad = QuadVertex::new([left as f32, top as f32, sw as f32, sh as f32])
                        .tex_rect([sx as f32, sy as f32, sw as f32, sh as f32]);
                    quad.tex_fill = color;
                    state.quads.push(quad);
                    left += sw as i32;
                }
            }
            Ok(())
        }
    })?;

    // ----------------------------------------------------------------------------------------
    let instance = linker.instantiate(store, &module)?;
    Ok(instance)
}

fn get_glyph_coords(font: u8, c: char) -> (u32, u32, u32, u32) {
    let (sw, sh) = match font {
        0 => (5, 5),
        1 => (5, 8),
        2 => (8, 8),
        _ => (5, 8),
    };
    let (ox, oy) = match font {
        0 => (0, 0),
        1 => (0, 32),
        2 => (0, 80),
        _ => (0, 32),
    };
    let (sx, sy) = match c {
        ' ' | '\n' | '\t' => (0, 0),
        '!' => (1, 0),
        '"' => (2, 0),
        '#' => (3, 0),
        '$' => (4, 0),
        '%' => (5, 0),
        '&' => (6, 0),
        '\'' | '’' => (7, 0),
        '(' => (8, 0),
        ')' => (9, 0),
        '*' => (10, 0),
        '+' => (11, 0),
        ',' => (12, 0),
        '-' => (13, 0),
        '.' => (14, 0),
        '/' => (15, 0),
        '0' => (0, 1),
        '1' => (1, 1),
        '2' => (2, 1),
        '3' => (3, 1),
        '4' => (4, 1),
        '5' => (5, 1),
        '6' => (6, 1),
        '7' => (7, 1),
        '8' => (8, 1),
        '9' => (9, 1),
        ':' => (10, 1),
        ';' => (11, 1),
        '<' => (12, 1),
        '=' => (13, 1),
        '>' => (14, 1),
        '?' => (15, 1),
        '@' => (0, 2),
        'A' => (1, 2),
        'B' => (2, 2),
        'C' => (3, 2),
        'D' => (4, 2),
        'E' => (5, 2),
        'F' => (6, 2),
        'G' => (7, 2),
        'H' => (8, 2),
        'I' => (9, 2),
        'J' => (10, 2),
        'K' => (11, 2),
        'L' => (12, 2),
        'M' => (13, 2),
        'N' => (14, 2),
        'O' => (15, 2),
        'P' => (0, 3),
        'Q' => (1, 3),
        'R' => (2, 3),
        'S' => (3, 3),
        'T' => (4, 3),
        'U' => (5, 3),
        'V' => (6, 3),
        'W' => (7, 3),
        'X' => (8, 3),
        'Y' => (9, 3),
        'Z' => (10, 3),
        '[' => (11, 3),
        '\\' => (12, 3),
        ']' => (13, 3),
        '^' => (14, 3),
        '_' => (15, 3),
        '`' => (0, 4),
        'a' => (1, 4),
        'b' => (2, 4),
        'c' => (3, 4),
        'd' => (4, 4),
        'e' => (5, 4),
        'f' => (6, 4),
        'g' => (7, 4),
        'h' => (8, 4),
        'i' => (9, 4),
        'j' => (10, 4),
        'k' => (11, 4),
        'l' => (12, 4),
        'm' => (13, 4),
        'n' => (14, 4),
        'o' => (15, 4),
        'p' => (0, 5),
        'q' => (1, 5),
        'r' => (2, 5),
        's' => (3, 5),
        't' => (4, 5),
        'u' => (5, 5),
        'v' => (6, 5),
        'w' => (7, 5),
        'x' => (8, 5),
        'y' => (9, 5),
        'z' => (10, 5),
        '{' => (11, 5),
        '|' => (12, 5),
        '}' => (13, 5),
        '~' => (14, 5),
        '🏠' => (15, 5),
        _ => (0, 0),
    };
    (ox + (sx * sw), oy + (sy * sh), sw, sh)
}
