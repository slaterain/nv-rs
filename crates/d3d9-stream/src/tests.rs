//! The stream against a backend that logs: the threaded runner carries out the same commands in
//! the same order as the direct one, a sync command sees every command before it, at most one
//! frame is in flight, a backend's panic stops the recording side, the queue stays bounded, files
//! replay, and a tee feeds both backends. Two producers ([`Stream::cut`]) are carried out, recorded
//! and digested in the serial order, and main never waits for the render thread while a slot is
//! open (a wait that would freeze the game fails the test after a time limit instead of hanging).

use crate::cmd::NAMES;
use crate::*;
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// TEST DATA: a backend that logs each command as it prints, keeps the levels it is given,
/// counts its draws and presents, and can be slow or refuse a command.
#[derive(Default)]
struct Log {
    lines: Arc<Mutex<Vec<String>>>,
    levels: HashMap<Handle, Vec<u8>>,
    draws: u32,
    presents: Arc<AtomicU64>,
    slow_present: Option<Duration>,
    slow_upload: Option<Duration>,
    refuse: Option<&'static str>,
    /// Added to its query answers (a shadow that answers otherwise).
    offset: u32,
}

impl Log {
    fn lines(&self) -> Arc<Mutex<Vec<String>>> {
        Arc::clone(&self.lines)
    }
}

impl Backend for Log {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        self.lines.lock().unwrap().push(format!("{cmd:?}"));
        if self.refuse == Some(cmd.name()) {
            panic!("the test backend refuses {}", cmd.name());
        }
        match cmd {
            Cmd::CreateDevice { .. } => Reply::Device {
                created: true,
                native: None,
            },
            Cmd::UploadSurface { surface, data, .. } => {
                if let Some(d) = self.slow_upload {
                    std::thread::sleep(d);
                }
                self.levels.insert(surface, data.bytes().into_owned());
                Reply::None
            }
            Cmd::DrawIndexedPrimitive { .. } => {
                self.draws += 1;
                Reply::None
            }
            Cmd::Present { .. } => {
                if let Some(d) = self.slow_present {
                    std::thread::sleep(d);
                }
                self.presents.fetch_add(1, Ordering::SeqCst);
                Reply::None
            }
            Cmd::ReadSurface { surface, size } => Reply::Bytes(
                self.levels
                    .get(&surface)
                    .filter(|b| b.len() == size as usize)
                    .cloned(),
            ),
            Cmd::GetRenderTargetData { pitch, .. } => Reply::Rows(Some(Rows {
                row: pitch,
                bytes: vec![self.draws as u8; pitch as usize],
            })),
            Cmd::QueryGetData { .. } => Reply::Query(Some((0, self.draws + self.offset))),
            _ => Reply::None,
        }
    }
}

/// A small random number source (xorshift), so a run is the same every time.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
}

/// TEST DATA: `frames` frames shaped like the game's (constants, textures, states, buffers,
/// draws; now and then an upload, a large upload, a read-back, a query, a creation and a
/// destruction), submitted to `stream`; the replies as they print.
fn play(stream: &mut Stream, frames: u32) -> Vec<String> {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let mut replies = Vec::new();
    let present: Vec<u32> = (0..14).collect();
    replies.push(format!(
        "{:?}",
        stream.submit(Cmd::CreateDevice {
            device: 0x1000,
            focus: 0x10,
            window: 0x20,
            behavior: 0x54,
            present: Data::from_words(&present),
            back_buffers: Data::from_words(&[0x2000]),
            depth_stencil: 0x3000,
        })
    ));
    for frame in 0..frames {
        stream.submit(Cmd::BeginScene {
            device: 0x1000,
            model: 0,
        });
        let draws = 40 + rng.next() % 40;
        for _ in 0..draws {
            let constants: Vec<u32> = (0..4 * (1 + rng.next() % 8)).map(|_| rng.next()).collect();
            let kind = (rng.next() % 6) as u8;
            // Booleans are one word a register.
            let per = if kind % 3 == 2 { 1 } else { 4 };
            stream.submit(Cmd::SetConstants {
                device: 0x1000,
                kind,
                start: rng.next() % 200,
                count: constants.len() as u32 / per,
                data: Data::from_words(&constants),
            });
            stream.submit(Cmd::SetTexture {
                device: 0x1000,
                sampler: rng.next() % 8,
                texture: 0x4000 + 8 * (rng.next() % 64),
            });
            stream.submit(Cmd::SetSamplerState {
                device: 0x1000,
                sampler: rng.next() % 8,
                kind: 1 + rng.next() % 13,
                value: rng.next(),
            });
            stream.submit(Cmd::SetRenderState {
                device: 0x1000,
                state: rng.next() % 210,
                value: rng.next(),
            });
            let matrix: Vec<u32> = (0..16).map(|_| rng.next()).collect();
            stream.submit(Cmd::SetTransform {
                device: 0x1000,
                state: 256,
                matrix: Data::from_words(&matrix),
            });
            stream.submit(Cmd::SetStreamSource {
                device: 0x1000,
                stream: 0,
                buffer: 0x5000 + 8 * (rng.next() % 32),
                offset: 0,
                stride: 32,
            });
            stream.submit(Cmd::SetIndices {
                device: 0x1000,
                buffer: 0x6000 + 8 * (rng.next() % 32),
            });
            stream.submit(Cmd::DrawIndexedPrimitive {
                device: 0x1000,
                kind: 4,
                base_vertex: 0,
                min_index: 0,
                vertices: rng.next() % 1000,
                start_index: 0,
                count: rng.next() % 500,
            });
            match rng.next() % 64 {
                0 => {
                    let seed = rng.next() as u8;
                    let fill = move |offset: u32, into: &mut [u8]| {
                        for (i, b) in into.iter_mut().enumerate() {
                            *b = (offset as usize + i) as u8 ^ seed;
                        }
                    };
                    stream.submit(Cmd::UploadBuffer {
                        buffer: 0x5000,
                        offset: 16,
                        flags: 0x2000,
                        data: Data::Read {
                            len: 1 + rng.next() % 3000,
                            read: &fill,
                        },
                    });
                }
                1 => {
                    let size = 64 * 64 * 4 + rng.next() % 200_000;
                    let level: Vec<u8> = (0..size).map(|i| (i ^ frame) as u8).collect();
                    let surface = 0x7000 + 8 * (rng.next() % 4);
                    stream.submit(Cmd::UploadSurface {
                        surface,
                        row: size,
                        rows: 1,
                        data: Data::Bytes(&level),
                    });
                    replies.push(format!(
                        "{:?}",
                        stream.submit(Cmd::ReadSurface { surface, size })
                    ));
                }
                2 => {
                    stream.submit(Cmd::QueryIssue {
                        query: 0x8000,
                        flags: 1,
                    });
                    replies.push(format!(
                        "{:?}",
                        stream.submit(Cmd::QueryGetData {
                            query: 0x8000,
                            flags: 1,
                        })
                    ));
                }
                3 => {
                    let tokens: Vec<u32> = (0..rng.next() % 300).map(|_| rng.next()).collect();
                    stream.submit(Cmd::CreateShader {
                        device: 0x1000,
                        shader: 0x9000,
                        pixel: true,
                        tokens: Data::from_words(&tokens),
                    });
                    stream.submit(Cmd::Destroy { object: 0x9000 });
                }
                4 => {
                    replies.push(format!(
                        "{:?}",
                        stream.submit(Cmd::GetRenderTargetData {
                            device: 0x1000,
                            source: 0x2000,
                            destination: 0x7000,
                            pitch: 16,
                        })
                    ));
                }
                _ => {}
            }
        }
        stream.submit(Cmd::EndScene {
            device: 0x1000,
            model: 0,
        });
        stream.submit(Cmd::Present {
            device: 0x1000,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame: 0,
        });
    }
    replies
}

/// The log and the replies of [`play`] through `make`'s stream.
fn run(make: impl FnOnce(Box<dyn Backend>) -> Stream, frames: u32) -> (Vec<String>, Vec<String>) {
    let log = Log::default();
    let lines = log.lines();
    let mut stream = make(Box::new(log));
    let replies = play(&mut stream, frames);
    drop(stream);
    let lines = lines.lock().unwrap().clone();
    (lines, replies)
}

#[test]
fn threaded_carries_out_the_same_commands_in_the_same_order_as_direct() {
    let (direct, direct_replies) = run(Stream::direct, 60);
    assert!(direct.len() > 20_000, "{} commands", direct.len());
    assert!(direct_replies.len() > 10);
    let tiny = Settings {
        chunk_commands: 3,
        chunk_bytes: 300,
        ..Settings::default()
    };
    let no_frame_in_flight = Settings {
        frames_in_flight: 0,
        check: true,
        ..Settings::default()
    };
    for settings in [Settings::default(), tiny, no_frame_in_flight] {
        let (threaded, replies) = run(|b| Stream::threaded(b, settings.clone()), 60);
        assert_eq!(threaded.len(), direct.len(), "{settings:?}");
        if let Some(i) = (0..direct.len()).find(|&i| threaded[i] != direct[i]) {
            panic!(
                "{settings:?}: command {i} differs:\n  direct   {}\n  threaded {}",
                direct[i], threaded[i]
            );
        }
        assert_eq!(replies, direct_replies, "{settings:?}");
    }
}

#[test]
fn presents_are_numbered_from_one() {
    let (lines, _) = run(Stream::direct, 3);
    let frames: Vec<&String> = lines.iter().filter(|l| l.starts_with("Present")).collect();
    assert_eq!(frames.len(), 3);
    for (i, l) in frames.iter().enumerate() {
        assert!(l.contains(&format!("frame: {}", i + 1)), "{l}");
    }
}

#[test]
fn a_sync_command_sees_every_command_before_it() {
    let log = Log {
        slow_upload: Some(Duration::from_millis(2)),
        ..Log::default()
    };
    let mut stream = Stream::threaded(Box::new(log), Settings::default());
    let mut last = Vec::new();
    for i in 0..20u8 {
        last = vec![i; 4096];
        stream.submit(Cmd::UploadSurface {
            surface: 0x7000,
            row: 64,
            rows: 64,
            data: Data::Bytes(&last),
        });
    }
    match stream.submit(Cmd::ReadSurface {
        surface: 0x7000,
        size: 4096,
    }) {
        Reply::Bytes(Some(b)) => assert_eq!(b, last),
        other => panic!("{other:?}"),
    }
}

/// Presents left to carry out each time `Present` returns, with a backend whose presents take
/// 20 ms.
fn ahead(frames_in_flight: u64) -> Vec<u64> {
    let log = Log {
        slow_present: Some(Duration::from_millis(20)),
        ..Log::default()
    };
    let done = Arc::clone(&log.presents);
    let settings = Settings {
        frames_in_flight,
        ..Settings::default()
    };
    let mut stream = Stream::threaded(Box::new(log), settings);
    (0..6)
        .map(|_| {
            stream.submit(Cmd::Present {
                device: 1,
                source: None,
                dest: None,
                window: 0,
                dirty: false,
                model: 0,
                frame: 0,
            });
            u64::from(stream.presents()) - done.load(Ordering::SeqCst)
        })
        .collect()
}

#[test]
fn at_most_one_frame_is_in_flight() {
    let one = ahead(1);
    assert!(one.iter().all(|&n| n <= 1), "{one:?}");
    assert!(
        one.contains(&1),
        "the game goes on while a present is carried out: {one:?}"
    );
    let none = ahead(0);
    assert!(none.iter().all(|&n| n == 0), "{none:?}");
}

#[test]
fn a_backend_panic_stops_the_recording_side_with_its_message() {
    let log = Log {
        refuse: Some("SetFvf"),
        ..Log::default()
    };
    let mut stream = Stream::threaded(Box::new(log), Settings::default());
    stream.submit(Cmd::SetFvf { device: 1, fvf: 2 });
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        stream.submit(Cmd::QueryGetData { query: 3, flags: 1 });
    }));
    let message = crate::runner::panic_message(&*stopped.expect_err("the recording side stops"));
    assert!(
        message.contains("render thread stopped") && message.contains("refuses SetFvf"),
        "{message}"
    );
    // And it ends without waiting for the stopped render thread.
    drop(stream);
}

#[test]
fn the_queue_stays_bounded_while_the_render_thread_is_behind() {
    let log = Log {
        slow_upload: Some(Duration::from_millis(1)),
        ..Log::default()
    };
    let settings = Settings {
        max_queued_bytes: 1 << 20,
        max_queued_chunks: 8,
        chunk_bytes: 64 << 10,
        ..Settings::default()
    };
    let mut stream = Stream::threaded(Box::new(log), settings);
    let level = vec![7u8; 100_000];
    let mut most = 0;
    for _ in 0..100 {
        stream.submit(Cmd::UploadSurface {
            surface: 1,
            row: 1000,
            rows: 100,
            data: Data::Bytes(&level),
        });
        let (bytes, chunks) = stream.queued();
        most = most.max(bytes);
        assert!(chunks <= 9, "{chunks} chunks waiting");
    }
    assert!(most <= (1 << 20) + 2 * 100_000, "{most} bytes waiting");
    stream.finish();
    assert_eq!(stream.queued(), (0, 0));
}

#[test]
fn dropping_the_stream_carries_out_what_was_recorded() {
    let log = Log::default();
    let lines = log.lines();
    let mut stream = Stream::threaded(Box::new(log), Settings::default());
    for i in 0..10_000 {
        stream.submit(Cmd::SetRenderState {
            device: 1,
            state: 7,
            value: i,
        });
    }
    drop(stream);
    assert_eq!(lines.lock().unwrap().len(), 10_000);
}

/// TEST DATA: payloads of the sizes the calls give them, filled with distinct words.
struct Payloads {
    present: [u32; 14],
    handles: [u32; 3],
    rect: [u32; 4],
    other_rect: [u32; 4],
    matrix: [u32; 16],
    ramp: Vec<u8>,
    registers: [u32; 12],
    vertices: [u32; 9],
    material: [u32; 17],
    light: [u32; 26],
    cube: [u32; 6],
    tokens: [u32; 5],
    elements: [u32; 4],
    level: [u32; 2],
}

impl Payloads {
    fn new() -> Payloads {
        fn words<const N: usize>(from: u32) -> [u32; N] {
            std::array::from_fn(|i| from.wrapping_add(i as u32).wrapping_mul(0x0101_0101))
        }
        Payloads {
            present: words(1),
            handles: words(20),
            rect: words(30),
            other_rect: words(40),
            matrix: words(50),
            ramp: (0..0x600u32).map(|i| (i * 3) as u8).collect(),
            registers: words(70),
            vertices: words(100),
            material: words(110),
            light: words(130),
            cube: words(160),
            tokens: words(170),
            elements: words(180),
            level: words(190),
        }
    }
}

/// One of every command, in the list's order; `big` the payload of the buffer's upload.
fn every_command<'a>(p: &'a Payloads, big: Data<'a>) -> Vec<Cmd<Data<'a>>> {
    let w = |words: &'a [u32]| Data::from_words(words);
    let d = 0x1000;
    vec![
        Cmd::CreateDevice {
            device: d,
            focus: -2,
            window: 0x1234_5678_9abc,
            behavior: 0x54,
            present: w(&p.present),
            back_buffers: w(&p.handles),
            depth_stencil: 3,
        },
        Cmd::Reset {
            device: d,
            window: 9,
            present: w(&p.present),
            back_buffers: w(&p.handles[..1]),
            depth_stencil: 0,
        },
        Cmd::Present {
            device: d,
            source: Some(w(&p.rect)),
            dest: None,
            window: 5,
            dirty: true,
            model: 1,
            frame: 1, // as the stream numbers its first present
        },
        Cmd::BeginScene {
            device: d,
            model: 0,
        },
        Cmd::EndScene {
            device: d,
            model: 0x8876_086c,
        },
        Cmd::Clear {
            device: d,
            count: 1,
            rects: Some(w(&p.other_rect)),
            flags: 7,
            colour: 0xff00_00ff,
            z: 1f32.to_bits(),
            stencil: 3,
            model: 0,
        },
        Cmd::SetRenderTarget {
            device: d,
            index: 1,
            surface: 4,
        },
        Cmd::SetDepthStencilSurface {
            device: d,
            surface: 0,
        },
        Cmd::SetViewport {
            device: d,
            viewport: [1, 2, 3, 4, 5, 6],
        },
        Cmd::SetTransform {
            device: d,
            state: 256,
            matrix: w(&p.matrix),
        },
        Cmd::SetScissorRect {
            device: d,
            rect: [-1, 2, -3, 4],
        },
        Cmd::SetClipPlane {
            device: d,
            index: 2,
            plane: [9, 8, 7, 6],
        },
        Cmd::SetGammaRamp {
            device: d,
            swap_chain: 0,
            flags: 1,
            ramp: Data::Bytes(&p.ramp),
        },
        Cmd::SetRenderState {
            device: d,
            state: 7,
            value: 1,
        },
        Cmd::SetTextureStageState {
            device: d,
            stage: 1,
            kind: 2,
            value: 3,
        },
        Cmd::SetSamplerState {
            device: d,
            sampler: 257,
            kind: 5,
            value: 2,
        },
        Cmd::SetTexture {
            device: d,
            sampler: 3,
            texture: 0x4000,
        },
        Cmd::SetVertexDeclaration {
            device: d,
            declaration: 0x4100,
        },
        Cmd::SetFvf {
            device: d,
            fvf: 0x152,
        },
        Cmd::SetShader {
            device: d,
            pixel: true,
            shader: 0x4200,
        },
        Cmd::SetConstants {
            device: d,
            kind: 3,
            start: 2,
            count: 3,
            data: w(&p.registers),
        },
        Cmd::SetStreamSource {
            device: d,
            stream: 1,
            buffer: 0x4300,
            offset: 16,
            stride: 32,
        },
        Cmd::SetIndices {
            device: d,
            buffer: 0x4400,
        },
        Cmd::DrawPrimitive {
            device: d,
            kind: 5,
            start: 1,
            count: 2,
        },
        Cmd::DrawIndexedPrimitive {
            device: d,
            kind: 4,
            base_vertex: -7,
            min_index: 1,
            vertices: 2,
            start_index: 3,
            count: 4,
        },
        Cmd::DrawPrimitiveUp {
            device: d,
            kind: 4,
            count: 1,
            stride: 12,
            vertices: w(&p.vertices),
        },
        Cmd::SetMaterial {
            device: d,
            material: w(&p.material),
        },
        Cmd::SetLight {
            device: d,
            index: 1,
            light: w(&p.light),
        },
        Cmd::LightEnable {
            device: d,
            index: 1,
            enable: 1,
        },
        Cmd::SetSoftwareVertexProcessing {
            device: d,
            software: 0,
        },
        Cmd::UpdateTexture {
            device: d,
            source: 1,
            destination: 2,
        },
        Cmd::StretchRect {
            device: d,
            source: 1,
            source_rect: None,
            destination: 2,
            destination_rect: Some(w(&p.rect)),
            filter: 2,
        },
        Cmd::ColorFill {
            device: d,
            surface: 1,
            rect: Some(w(&p.other_rect)),
            colour: 5,
        },
        Cmd::CreateTexture {
            device: d,
            texture: 0x4000,
            levels: w(&p.handles),
            width: 256,
            height: 128,
            usage: 0,
            format: 21,
            pool: 1,
        },
        Cmd::CreateCubeTexture {
            device: d,
            texture: 0x4500,
            surfaces: w(&p.cube),
            edge: 64,
            usage: 1,
            format: 22,
            pool: 0,
        },
        Cmd::CreateSurface {
            device: d,
            surface: 0x4600,
            depth: true,
            width: 1,
            height: 2,
            format: 75,
            multisample: 0,
            quality: 0,
            flag: 1,
        },
        Cmd::CreateOffscreenSurface {
            device: d,
            surface: 0x4700,
            width: 3,
            height: 4,
            format: 21,
            pool: 2,
        },
        Cmd::CreateBuffer {
            device: d,
            buffer: 0x4300,
            index: false,
            length: 4096,
            usage: 0x208,
            format: 0x152,
            pool: 0,
        },
        Cmd::CreateShader {
            device: d,
            shader: 0x4200,
            pixel: false,
            tokens: w(&p.tokens),
        },
        Cmd::CreateDeclaration {
            device: d,
            declaration: 0x4100,
            elements: w(&p.elements),
        },
        Cmd::CreateQuery {
            device: d,
            query: 0x4800,
            kind: 9,
        },
        Cmd::Destroy { object: 0x4800 },
        Cmd::UploadSurface {
            surface: 0x4600,
            row: 4,
            rows: 2,
            data: w(&p.level),
        },
        Cmd::UploadBuffer {
            buffer: 0x4300,
            offset: 8,
            flags: 0x1000,
            data: big,
        },
        Cmd::SetLod {
            texture: 0x4000,
            lod: 2,
            model: 1,
        },
        Cmd::QueryIssue {
            query: 0x4800,
            flags: 2,
        },
        Cmd::ReadSurface {
            surface: 0x4600,
            size: 8,
        },
        Cmd::GetRenderTargetData {
            device: d,
            source: 1,
            destination: 2,
            pitch: 0,
        },
        Cmd::QueryGetData {
            query: 0x4800,
            flags: 0,
        },
    ]
}

#[test]
fn every_command_is_listed_once_and_well_formed() {
    let p = Payloads::new();
    let cmds = every_command(&p, Data::Bytes(&[1, 2, 3]));
    let names: Vec<&str> = cmds.iter().map(|c| c.name()).collect();
    assert_eq!(names, NAMES);
    for c in &cmds {
        assert_eq!(c.check(), Ok(()), "{}", c.name());
    }
    let sync: Vec<&str> = cmds
        .iter()
        .filter(|c| c.is_sync())
        .map(|c| c.name())
        .collect();
    assert_eq!(
        sync,
        [
            "CreateDevice",
            "ReadSurface",
            "GetRenderTargetData",
            "QueryGetData"
        ]
    );
}

/// A payload of another size than its call gives it is refused: by the check, on submitting in a
/// debug build, and on reading a file.
#[test]
fn a_payload_of_the_wrong_size_is_refused() {
    let short = [0u32; 15];
    let bad = Cmd::SetTransform {
        device: 1,
        state: 256,
        matrix: Data::from_words(&short),
    };
    assert!(bad
        .check()
        .unwrap_err()
        .contains("matrix is 60 bytes, not 64"));
    let booleans = [1u32; 3];
    let ok = Cmd::SetConstants {
        device: 1,
        kind: 2,
        start: 0,
        count: 3,
        data: Data::from_words(&booleans),
    };
    assert_eq!(ok.check(), Ok(()));
    let mut bytes = wire::header().to_vec();
    wire::encode(&bad, &mut bytes);
    assert!(matches!(
        wire::commands(&bytes).unwrap().next(),
        Some(Err(wire::WireError::Payload { .. }))
    ));
    if cfg!(debug_assertions) {
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Stream::direct(Box::new(Log::default())).submit(bad);
        }));
        assert!(refused.is_err());
    }
}

/// A file being written that the test reads afterwards.
#[derive(Clone, Default)]
struct File(Arc<Mutex<Vec<u8>>>);

impl Write for File {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn every_command_reads_back_as_written_and_replays() {
    let p = Payloads::new();
    let fill = |offset: u32, into: &mut [u8]| {
        for (i, b) in into.iter_mut().enumerate() {
            *b = (offset as usize + i) as u8;
        }
    };
    let big = Data::Read {
        len: 70_000,
        read: &fill,
    };
    let cmds = every_command(&p, big);
    let log = Log::default();
    let lines = log.lines();
    let mut stream = Stream::direct(Box::new(log));
    let file = File::default();
    stream.record_to(Box::new(file.clone())).unwrap();
    for c in &cmds {
        stream.submit(*c);
    }
    drop(stream);
    let bytes = file.0.lock().unwrap().clone();
    let read: Vec<String> = wire::commands(&bytes)
        .unwrap()
        .map(|c| format!("{:?}", c.unwrap()))
        .collect();
    let written = lines.lock().unwrap().clone();
    assert_eq!(read.len(), NAMES.len());
    same(&read, &written, "read from the file");
    // Replayed into another backend: the same commands.
    let mut again = Log::default();
    let replayed = again.lines();
    assert_eq!(
        wire::replay(&bytes, &mut again).unwrap(),
        NAMES.len() as u64
    );
    same(&replayed.lock().unwrap(), &written, "replayed");
    // And through a chunk (the render thread's way).
    let mut c = crate::chunk::Chunk::default();
    for cmd in &cmds {
        c.push(*cmd);
    }
    let chunked: Vec<String> = (0..c.len()).map(|i| format!("{:?}", c.get(i))).collect();
    same(&chunked, &written, "through a chunk");
}

/// `NV_REPEATABLE`'s record: every command as `record_to` writes it, except the window handles
/// (written as 0) and the query reads (left out); the backend still gets them all as they are.
#[test]
fn a_repeatable_record_leaves_out_windows_and_query_reads() {
    let p = Payloads::new();
    let cmds = every_command(&p, Data::Bytes(&[7; 16]));
    let log = Log::default();
    let lines = log.lines();
    let mut stream = Stream::direct(Box::new(log));
    let file = File::default();
    stream.record_repeatable(Box::new(file.clone())).unwrap();
    for c in &cmds {
        stream.submit(*c);
    }
    stream.finish_recording();
    let bytes = file.0.lock().unwrap().clone();
    drop(stream);
    let read: Vec<Cmd<Data<'_>>> = wire::commands(&bytes)
        .unwrap()
        .map(|c| c.unwrap())
        .collect();
    assert_eq!(lines.lock().unwrap().len(), NAMES.len());
    assert_eq!(
        read.len(),
        NAMES.len() - 1,
        "only the query read is left out"
    );
    let mut windows = 0;
    for c in &read {
        match c {
            Cmd::QueryGetData { .. } => panic!("a query read was written"),
            Cmd::CreateDevice { focus, window, .. } => {
                assert_eq!((*focus, *window), (0, 0));
                windows += 1;
            }
            Cmd::Reset { window, .. } | Cmd::Present { window, .. } => {
                assert_eq!(*window, 0);
                windows += 1;
            }
            _ => {}
        }
    }
    assert_eq!(windows, 3);
}

#[test]
fn a_damaged_file_is_refused() {
    assert_eq!(
        wire::commands(b"not a stream").err(),
        Some(wire::WireError::NotAStream)
    );
    let mut bytes = wire::header().to_vec();
    wire::encode(&Cmd::SetFvf { device: 1, fvf: 2 }, &mut bytes);
    let whole = bytes.clone();
    bytes.truncate(bytes.len() - 1);
    let mut cmds = wire::commands(&bytes).unwrap();
    assert!(matches!(
        cmds.next(),
        Some(Err(wire::WireError::Truncated { .. }))
    ));
    assert!(cmds.next().is_none());
    let mut unknown = whole.clone();
    unknown[16 + 4] = 0xff;
    assert!(matches!(
        wire::commands(&unknown).unwrap().next(),
        Some(Err(wire::WireError::UnknownCommand { tag: 0xff, .. }))
    ));
    assert_eq!(wire::commands(&whole).unwrap().count(), 1);
    // Written with another command list: refused before any command is read.
    let mut other = whole.clone();
    other[8] ^= 1;
    assert!(matches!(
        wire::commands(&other).err(),
        Some(wire::WireError::OtherList { .. })
    ));
}

/// The list's hash covers each command's place and fields: two commands of the same layout
/// (SetVertexDeclaration and SetFvf) swapped would give another hash.
#[test]
fn the_list_hash_covers_names_order_and_fields() {
    let list = crate::cmd::LIST;
    let a = list.find("SetVertexDeclaration").unwrap();
    let b = list.find("SetFvf").unwrap();
    assert!(a < b);
    let mut swapped = list.to_string();
    swapped.replace_range(b..b + "SetFvf".len(), "SetVertexDeclaration");
    swapped.replace_range(a..a + "SetVertexDeclaration".len(), "SetFvf");
    assert_ne!(
        wire::hash(wire::HASH_START, swapped.as_bytes()),
        wire::LIST_HASH
    );
    assert_eq!(
        wire::hash(wire::HASH_START, list.as_bytes()),
        wire::LIST_HASH
    );
    assert!(
        list.contains("SetConstants") && list.contains("optional"),
        "{list}"
    );
}

#[test]
fn a_tee_feeds_both_and_the_primary_answers() {
    let primary = Log::default();
    let shadow = Log {
        offset: 1000,
        ..Log::default()
    };
    let (a, b) = (primary.lines(), shadow.lines());
    let tee = Tee::new(Box::new(primary), Box::new(shadow));
    let mut stream = Stream::threaded(Box::new(tee), Settings::default());
    let replies = play(&mut stream, 5);
    drop(stream);
    let (_, alone) = run(Stream::direct, 5);
    assert_eq!(replies, alone, "the primary's answers");
    assert_eq!(*a.lock().unwrap(), *b.lock().unwrap());
    assert!(a.lock().unwrap().iter().any(|l| l.contains("frame: 5")));
}

#[test]
fn a_tee_counts_the_query_results_both_give_alike_and_otherwise() {
    let shadow = Log {
        offset: 2,
        ..Log::default()
    };
    let mut tee = Tee::new(Box::new(Log::default()), Box::new(shadow));
    let get = Cmd::QueryGetData { query: 7, flags: 1 };
    // Both answer 0 draws + offset: 0 and 2, different.
    assert!(matches!(tee.execute(get), Reply::Query(Some((0, 0)))));
    assert_eq!(tee.query_results(), (0, 1));
    let alike = Log::default();
    let mut tee = Tee::new(Box::new(Log::default()), Box::new(alike));
    tee.execute(get);
    tee.execute(Cmd::SetFvf { device: 1, fvf: 2 });
    assert_eq!(tee.query_results(), (1, 0));
}

#[test]
fn a_shadow_that_panics_is_left_out() {
    let shadow = Log {
        refuse: Some("SetFvf"),
        ..Log::default()
    };
    let shadow_lines = shadow.lines();
    let primary = Log::default();
    let lines = primary.lines();
    let mut tee = Tee::new(Box::new(primary), Box::new(shadow));
    for fvf in 0..3 {
        tee.execute(Cmd::SetFvf { device: 1, fvf });
    }
    assert!(!tee.shadow_running());
    assert_eq!(lines.lock().unwrap().len(), 3);
    assert_eq!(shadow_lines.lock().unwrap().len(), 1);
}

/// Two logs are the same (the first difference printed, not the whole logs).
fn same(a: &[String], b: &[String], what: &str) {
    if let Some(i) = (0..a.len().max(b.len())).find(|&i| a.get(i) != b.get(i)) {
        let short =
            |s: Option<&String>| s.map_or("(none)".to_string(), |s| s.chars().take(300).collect());
        panic!(
            "{what}: line {i} differs:\n  {}\n  {}",
            short(a.get(i)),
            short(b.get(i))
        );
    }
}

/// The recording side's cost a command, against a backend that does nothing: carried out at once
/// (direct) and recorded for the render thread (threaded), over frames shaped like the town's
/// (about 7,000 commands: 640 draws of 11 calls, constants of 4 registers); then the same with a
/// cut each frame, a quarter of the draws through its slot on the same thread (as stepped).
/// `cargo test --release -p d3d9-stream cost_of_recording -- --ignored --nocapture`.
#[test]
#[ignore]
fn cost_of_recording() {
    struct Nothing;
    impl Backend for Nothing {
        fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
            std::hint::black_box(&cmd);
            Reply::None
        }
    }
    let registers: Vec<u32> = (0..16).collect();
    let frame = |stream: &mut Stream, cut: bool| {
        let (slot_from, slot_to) = if cut { (320, 480) } else { (640, 640) };
        for i in 0..slot_from {
            town_draw(i, &registers, |c| {
                stream.submit(c);
            });
        }
        if cut {
            let mut slot = stream.cut();
            for i in slot_from..slot_to {
                town_draw(i, &registers, |c| {
                    slot.submit(c);
                });
            }
            slot.close();
        }
        for i in slot_to..640 {
            town_draw(i, &registers, |c| {
                stream.submit(c);
            });
        }
        stream.submit(Cmd::Present {
            device: 1,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame: 0,
        });
    };
    let frames = 300;
    let commands = f64::from(frames * (640 * 11 + 1));
    for cut in [false, true] {
        for threaded in [false, true] {
            let mut stream = if threaded {
                Stream::threaded(Box::new(Nothing), Settings::default())
            } else {
                Stream::direct(Box::new(Nothing))
            };
            frame(&mut stream, cut);
            let t = std::time::Instant::now();
            for _ in 0..frames {
                frame(&mut stream, cut);
            }
            let ns = t.elapsed().as_nanos() as f64 / commands;
            eprintln!(
                "{}{}: {ns:.1} ns a command, {:.3} ms a frame of 7,041",
                if threaded { "threaded" } else { "direct" },
                if cut { ", a cut each frame" } else { "" },
                ns * 7041.0 / 1e6
            );
        }
    }
}

/// [`cost_of_recording`]'s draw `i`: its 11 calls through `submit`.
fn town_draw(i: u32, registers: &[u32], mut submit: impl FnMut(Cmd<Data<'_>>)) {
    for kind in [0u8, 0, 0, 3, 3] {
        submit(Cmd::SetConstants {
            device: 1,
            kind,
            start: i % 200,
            count: 4,
            data: Data::from_words(registers),
        });
    }
    submit(Cmd::SetSamplerState {
        device: 1,
        sampler: i % 8,
        kind: 5,
        value: i,
    });
    submit(Cmd::SetTexture {
        device: 1,
        sampler: i % 8,
        texture: 0x4000 + i,
    });
    submit(Cmd::SetRenderState {
        device: 1,
        state: 7,
        value: i,
    });
    submit(Cmd::SetStreamSource {
        device: 1,
        stream: 0,
        buffer: 0x5000 + i,
        offset: 0,
        stride: 32,
    });
    submit(Cmd::SetIndices {
        device: 1,
        buffer: 0x6000 + i,
    });
    submit(Cmd::DrawIndexedPrimitive {
        device: 1,
        kind: 4,
        base_vertex: 0,
        min_index: 0,
        vertices: 100,
        start_index: 0,
        count: 50,
    });
}

/// [`cost_of_recording`]'s chunk part alone: the commands pushed into a chunk and the chunk
/// emptied every 1,024, on one thread.
#[test]
#[ignore]
fn cost_of_pushing() {
    let registers: Vec<u32> = (0..16).collect();
    let mut c = crate::chunk::Chunk::default();
    let rounds = 300 * 7041 / 11;
    let t = std::time::Instant::now();
    for i in 0..rounds as u32 {
        for kind in [0u8, 0, 0, 3, 3] {
            c.push(Cmd::SetConstants {
                device: 1,
                kind,
                start: i % 200,
                count: 4,
                data: Data::from_words(&registers),
            });
        }
        for _ in 0..6 {
            c.push(Cmd::SetRenderState {
                device: 1,
                state: 7,
                value: i,
            });
        }
        if c.len() >= 1024 {
            std::hint::black_box(&c);
            c.clear();
        }
    }
    let ns = t.elapsed().as_nanos() as f64 / (rounds * 11) as f64;
    eprintln!("pushing: {ns:.1} ns a command");
}

/// The stream can live in state the game's threads share (behind their lock), and a reply can
/// cross from the render thread.
#[test]
fn the_stream_and_its_replies_move_between_threads() {
    fn moves<T: Send>() {}
    moves::<Stream>();
    moves::<Reply>();
    moves::<Tee>();
}

/// A backend that refuses every command and panics again when dropped.
struct Bomb;

impl Backend for Bomb {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        panic!("the bomb refuses {}", cmd.name());
    }
}

impl Drop for Bomb {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            panic!("the bomb goes off in its drop");
        }
    }
}

#[test]
fn a_shadow_that_panics_in_its_drop_too_is_left_out() {
    let primary = Log::default();
    let lines = primary.lines();
    let mut tee = Tee::new(Box::new(primary), Box::new(Bomb));
    for fvf in 0..3 {
        tee.execute(Cmd::SetFvf { device: 1, fvf });
    }
    assert!(!tee.shadow_running());
    assert_eq!(lines.lock().unwrap().len(), 3);
}

/// A render thread stopped by its backend stops the recording side at its next command, not only
/// at the next wait.
#[test]
fn the_next_command_after_a_backend_panic_stops() {
    let log = Log {
        refuse: Some("SetFvf"),
        ..Log::default()
    };
    let mut stream = Stream::threaded(Box::new(log), Settings::default());
    stream.submit(Cmd::SetFvf { device: 1, fvf: 2 });
    stream.flush();
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for value in 0..10_000 {
            stream.submit(Cmd::SetRenderState {
                device: 1,
                state: 7,
                value,
            });
            std::thread::sleep(Duration::from_millis(1));
        }
    }));
    let message = crate::runner::panic_message(&*stopped.expect_err("the recording side stops"));
    assert!(message.contains("refuses SetFvf"), "{message}");
}

/// A stopped render thread stops a caller that only hands over (`flush`), with nothing recorded
/// since: a poll that never records would otherwise spin for ever.
#[test]
fn a_flush_after_the_render_thread_stopped_stops_too() {
    let log = Log {
        refuse: Some("SetFvf"),
        ..Log::default()
    };
    let mut stream = Stream::threaded(Box::new(log), Settings::default());
    stream.submit(Cmd::SetFvf { device: 1, fvf: 2 });
    stream.flush();
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for _ in 0..10_000 {
            stream.flush();
            std::thread::sleep(Duration::from_millis(1));
        }
    }));
    let message = crate::runner::panic_message(&*stopped.expect_err("the flush stops"));
    assert!(message.contains("refuses SetFvf"), "{message}");
}

/// A file of `cmds`, as `NV_D3D9_RECORD` writes one.
fn file_of(cmds: &[Cmd<Data<'_>>]) -> Vec<u8> {
    let mut bytes = wire::header().to_vec();
    for c in cmds {
        wire::encode(c, &mut bytes);
    }
    bytes
}

/// The objects of [`drawing`]: the device, a texture, its two levels, a buffer, the buffer bound
/// after the first one's `Destroy`, the level made the render target, and the texture bound.
#[derive(Clone, Copy)]
struct Objects {
    device: u32,
    texture: u32,
    levels: [u32; 2],
    buffer: u32,
    again: u32,
    target: u32,
    bound: u32,
}

/// A small drawing over `o`'s objects, its draw `count` primitives.
fn drawing(o: &Objects, count: u32) -> Vec<u8> {
    let device = o.device;
    file_of(&[
        Cmd::CreateTexture {
            device,
            texture: o.texture,
            levels: Data::from_words(&o.levels),
            width: 4,
            height: 4,
            usage: 0,
            format: 21,
            pool: 1,
        },
        Cmd::SetRenderTarget {
            device,
            index: 0,
            surface: o.target,
        },
        Cmd::SetTexture {
            device,
            sampler: 0,
            texture: o.bound,
        },
        Cmd::SetStreamSource {
            device,
            stream: 0,
            buffer: o.buffer,
            offset: 0,
            stride: 32,
        },
        Cmd::DrawPrimitive {
            device,
            kind: 4,
            start: 0,
            count,
        },
        Cmd::Destroy { object: o.buffer },
        Cmd::SetStreamSource {
            device,
            stream: 0,
            buffer: o.again,
            offset: 0,
            stride: 32,
        },
        Cmd::Present {
            device,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame: 1,
        },
    ])
}

/// Two records of the same drawing with every object at other addresses (a game heap laid out
/// otherwise, ADR-0014 stage 2) compare alike once handles are renamed by first use, the handles in
/// a texture's levels too, and an address used again after its object's `Destroy` is a new object.
/// A real difference (another draw, another level as the render target, the buffer bound where the
/// texture was) is found at its command, and so is a file that ends first.
#[test]
fn records_compare_with_handles_renamed() {
    let a = Objects {
        device: 0x2000_0010,
        texture: 0x2000_0020,
        levels: [0x2000_0100, 0x2000_0108],
        buffer: 0x2000_0030,
        // The freed buffer's address handed out again to the next one.
        again: 0x2000_0030,
        target: 0x2000_0108,
        bound: 0x2000_0020,
    };
    let b = Objects {
        device: 0x2400_0010,
        texture: 0x2400_0040,
        levels: [0x2400_0300, 0x2400_0310],
        buffer: 0x2400_0050,
        again: 0x2400_0060,
        target: 0x2400_0310,
        bound: 0x2400_0040,
    };
    let (ra, rb) = (drawing(&a, 2), drawing(&b, 2));
    assert_ne!(ra, rb, "the addresses differ");
    assert_eq!(wire::compare_renamed(&ra[..], &rb[..]), Ok((8, None)));
    // Another draw.
    let (alike, diff) = wire::compare_renamed(&ra[..], &drawing(&b, 3)[..]).unwrap();
    let diff = diff.expect("a difference");
    assert_eq!((alike, diff.index), (4, 4));
    assert!(diff.a.expect("a's").contains("count: 2"));
    // The other level as the render target: the levels' handles are renamed with the texture.
    let other_level = Objects {
        target: 0x2400_0300,
        ..b
    };
    let found = wire::compare_renamed(&ra[..], &drawing(&other_level, 2)[..]).unwrap();
    assert_eq!(found.0, 1);
    // The buffer bound where the texture was.
    let swapped = Objects {
        bound: 0x2400_0050,
        ..b
    };
    let found = wire::compare_renamed(&ra[..], &drawing(&swapped, 2)[..]).unwrap();
    assert_eq!(found.0, 2);
    // A file that ends first, one cut inside a record, and one that is no stream.
    let (alike, diff) = wire::compare_renamed(&ra[..], &file_of(&[])[..]).unwrap();
    assert_eq!((alike, diff.map(|d| (d.index, d.b))), (0, Some((0, None))));
    assert!(wire::compare_renamed(&ra[..], &rb[..rb.len() - 10]).is_err());
    assert!(wire::compare_renamed(&b"not a stream at all"[..], &ra[..]).is_err());
}

/// TEST DATA: a backend with idle work for its first `turns` idle calls, counting them.
struct Idler {
    turns: u64,
    calls: Arc<AtomicU64>,
}

impl Backend for Idler {
    fn execute(&mut self, _cmd: Cmd<Data<'_>>) -> Reply {
        Reply::None
    }

    fn idle(&mut self) -> bool {
        let n = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        n < self.turns
    }
}

/// The render thread gives a backend's idle work a turn whenever nothing is queued, again after
/// [`IDLE_POLL`] while it says it has more, and then sleeps until the next command.
#[test]
fn the_render_thread_gives_idle_work_turns_until_it_is_done() {
    let calls = Arc::new(AtomicU64::new(0));
    let mut s = Stream::threaded(
        Box::new(Idler {
            turns: 5,
            calls: Arc::clone(&calls),
        }),
        Settings::default(),
    );
    s.submit(Cmd::SetFvf { device: 1, fvf: 0 });
    s.finish();
    // Five turns at least come by themselves, a millisecond or so apart (the first when the
    // render thread starts with nothing queued), then no more while nothing comes.
    let settled = || {
        let start = std::time::Instant::now();
        let mut last = calls.load(Ordering::Relaxed);
        loop {
            std::thread::sleep(IDLE_POLL * 20);
            let now = calls.load(Ordering::Relaxed);
            if now == last && now >= 5 {
                return now;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "idle turns never settle"
            );
            last = now;
        }
    };
    let before = settled();
    // The next command, and the queue empty again: one more turn, and no more.
    s.submit(Cmd::SetFvf { device: 1, fvf: 1 });
    s.finish();
    assert_eq!(settled(), before + 1);
}

// --- Two producers (`Stream::cut`) ---

/// TEST DATA: one command of a frame, owning its payload; or main's flush.
#[derive(Clone)]
enum Part {
    State(u32, u32),
    Texture(u32, u32),
    Constants(u32, Vec<u32>),
    Upload(u32, Vec<u32>),
    Declaration(u32, Vec<u32>),
    Draw(u32),
    Issue(u32),
    Read(u32),
    Present,
    /// [`Stream::flush`] (main's only): no command.
    Flush,
}

impl Part {
    fn cmd(&self) -> Cmd<Data<'_>> {
        match self {
            Part::State(state, value) => Cmd::SetRenderState {
                device: 0x1000,
                state: *state,
                value: *value,
            },
            Part::Texture(sampler, texture) => Cmd::SetTexture {
                device: 0x1000,
                sampler: *sampler,
                texture: *texture,
            },
            Part::Constants(start, words) => Cmd::SetConstants {
                device: 0x1000,
                kind: 0,
                start: *start,
                count: words.len() as u32 / 4,
                data: Data::from_words(words),
            },
            Part::Upload(buffer, words) => Cmd::UploadBuffer {
                buffer: *buffer,
                offset: 0,
                flags: 0,
                data: Data::from_words(words),
            },
            Part::Declaration(declaration, words) => Cmd::CreateDeclaration {
                device: 0x1000,
                declaration: *declaration,
                elements: Data::from_words(words),
            },
            Part::Draw(count) => Cmd::DrawIndexedPrimitive {
                device: 0x1000,
                kind: 4,
                base_vertex: 0,
                min_index: 0,
                vertices: 3 * count,
                start_index: 0,
                count: *count,
            },
            Part::Issue(query) => Cmd::QueryIssue {
                query: *query,
                flags: 1,
            },
            Part::Read(query) => Cmd::QueryGetData {
                query: *query,
                flags: 1,
            },
            Part::Present => Cmd::Present {
                device: 0x1000,
                source: None,
                dest: None,
                window: 0,
                dirty: false,
                model: 0,
                frame: 0,
            },
            Part::Flush => unreachable!("a flush is no command"),
        }
    }
}

/// Main's part `p` through `stream`: its reply as it prints (a flush hands over what is recorded).
fn send(stream: &mut Stream, p: &Part) -> String {
    match p {
        Part::Flush => {
            stream.flush();
            "flushed".to_string()
        }
        p => format!("{:?}", stream.submit(p.cmd())),
    }
}

/// TEST DATA: `frames` frames, each in three parts: main's commands before the cut (a query read
/// first: main's sync call after the last frame's slot, then states and draws), the drawing side's
/// (a creation, then states, textures, constants, uploads and draws, with a query issued and read
/// back between them: a sync call whose answer counts the draws before it; none in some frames,
/// two in others, and every fifth slot empty), and main's after the cut (more states and draws, a
/// large upload, the present). Some states repeat a value already set.
fn three_part_frames(frames: u32) -> Vec<[Vec<Part>; 3]> {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut list = Vec::new();
    for frame in 0..frames {
        let mut part = |draws: u32, uploads: bool| {
            let mut v = Vec::new();
            for _ in 0..draws {
                v.push(Part::State(7 + rng.next() % 4, rng.next() % 3));
                v.push(Part::Texture(
                    rng.next() % 4,
                    0x4000 + 8 * (rng.next() % 16),
                ));
                let words: Vec<u32> = (0..4 * (1 + rng.next() % 4)).map(|_| rng.next()).collect();
                v.push(Part::Constants(rng.next() % 32, words));
                if uploads && rng.next() % 4 == 0 {
                    let words: Vec<u32> = (0..16 + rng.next() % 64).map(|_| rng.next()).collect();
                    v.push(Part::Upload(0x6000 + 4 * (rng.next() % 8), words));
                }
                v.push(Part::Draw(1 + rng.next() % 50));
            }
            v
        };
        let mut before = vec![Part::Read(0x8001)];
        before.extend(part(5, false));
        let mut draw = Vec::new();
        if frame % 5 != 4 {
            draw.push(Part::Declaration(0x7000 + frame, vec![frame; 8]));
            draw.extend(part(10, true));
            for _ in 0..[1, 0, 2, 1][frame as usize % 5] {
                draw.push(Part::Issue(0x8000));
                draw.push(Part::Read(0x8000));
                draw.extend(part(10, true));
            }
        }
        let mut after = part(10, false);
        after.push(Part::Upload(0x6100, (0..20_000).collect()));
        after.push(Part::Present);
        list.push([before, draw, after]);
    }
    list
}

/// The draws in `frames`.
fn draws_in(frames: &[[Vec<Part>; 3]]) -> usize {
    frames
        .iter()
        .flatten()
        .flatten()
        .filter(|p| matches!(p, Part::Draw(_)))
        .count()
}

/// The frames through `stream` in the serial order, all from main: the backend's log is this.
/// Every reply, in order.
fn serial(stream: &mut Stream, frames: &[[Vec<Part>; 3]]) -> Vec<String> {
    let mut replies = Vec::new();
    for parts in frames {
        for p in parts.iter().flatten() {
            replies.push(send(stream, p));
        }
    }
    replies
}

/// The frames with two producers, as the game's layer makes them: main submits its part before
/// the cut, cuts, and submits its part after the cut while a drawing thread submits the middle
/// part through the slot at the same time (each side yielding at random), every call under the
/// device lock (a mutex both take for each call, as the layer's does); at the next frame main
/// waits for the slot as the layer does (`cut_waiter`, the lock let go of) and makes its sync
/// call. Every reply, in the serial order.
fn two_producers(stream: &mut Stream, frames: &[[Vec<Part>; 3]]) -> Vec<String> {
    let device = Mutex::new(());
    let device = &device;
    let mut replies = Vec::new();
    let mut rng = Rng(0x1234_5678_9abc_def1);
    std::thread::scope(|scope| {
        for parts in frames {
            if let Some(waiter) = stream.cut_waiter() {
                waiter.wait();
            }
            for p in &parts[0] {
                let _lock = device.lock().unwrap();
                replies.push(send(stream, p));
            }
            let mut slot = {
                let _lock = device.lock().unwrap();
                stream.cut()
            };
            let pause = rng.next() % 3;
            let draw = scope.spawn(move || {
                let mut answers = Vec::new();
                let mut rng = Rng(u64::from(pause) + 7);
                for p in &parts[1] {
                    if rng.next() % 16 == 0 {
                        std::thread::yield_now();
                    }
                    let _lock = device.lock().unwrap();
                    answers.push(format!("{:?}", slot.submit(p.cmd())));
                }
                slot.close();
                answers
            });
            let mut after = Vec::new();
            for p in &parts[2] {
                if rng.next() % 8 == 0 {
                    std::thread::yield_now();
                }
                let _lock = device.lock().unwrap();
                after.push(send(stream, p));
            }
            replies.extend(draw.join().unwrap());
            replies.extend(after);
        }
    });
    stream.finish();
    replies
}

/// The frames as with stepped threads, where the drawing job runs only once main waits for it:
/// main submits its part before the cut, cuts, and submits its whole part after the cut before the
/// drawing side records anything; then main takes the cut's waiter (the slot still open), lets the
/// drawing side go and waits for the slot as the layer does, and the slot is settled. Every reply,
/// in the serial order.
fn stepped(stream: &mut Stream, frames: &[[Vec<Part>; 3]]) -> Vec<String> {
    let mut replies = Vec::new();
    std::thread::scope(|scope| {
        for parts in frames {
            for p in &parts[0] {
                replies.push(send(stream, p));
            }
            let mut slot = stream.cut();
            let (go, told) = std::sync::mpsc::channel::<()>();
            let draw = scope.spawn(move || {
                told.recv().expect("main lets the drawing side go");
                let answers: Vec<String> = parts[1]
                    .iter()
                    .map(|p| format!("{:?}", slot.submit(p.cmd())))
                    .collect();
                slot.close();
                answers
            });
            let after: Vec<String> = parts[2].iter().map(|p| send(stream, p)).collect();
            let waiter = stream
                .cut_waiter()
                .expect("the slot is open: the drawing side has not begun");
            assert!(!waiter.is_closed());
            go.send(()).unwrap();
            waiter.wait();
            assert!(!stream.cut_open(), "the slot is closed and settled");
            replies.extend(draw.join().unwrap());
            replies.extend(after);
        }
    });
    stream.finish();
    replies
}

/// TEST DATA: frames for [`stepped`]: after the cut, main submits a burst of uploads past a small
/// queue's limit (a cell loading), a few commands a flush hands over into the full queue, and two
/// Presents; the slot has a sync call between its draws; main's first command of a frame is a sync
/// call (after the last frame's slot).
fn stepped_frames(frames: u32) -> Vec<[Vec<Part>; 3]> {
    (0..frames)
        .map(|frame| {
            let before = vec![Part::Read(0x8001), Part::State(7, frame), Part::Draw(3)];
            let mut draw = vec![Part::Declaration(0x7000 + frame, vec![frame; 8])];
            for i in 0..12 {
                draw.extend([Part::State(8, i), Part::Draw(1 + i)]);
            }
            draw.extend([Part::Issue(0x8000), Part::Read(0x8000), Part::Draw(2)]);
            let mut after = Vec::new();
            for i in 0..40 {
                after.extend([
                    Part::Upload(0x6000 + 4 * (i % 8), vec![i; 512]),
                    Part::Draw(1),
                ]);
            }
            after.extend([Part::State(9, frame), Part::Draw(4), Part::Flush]);
            after.extend([
                Part::Present,
                Part::State(10, frame),
                Part::Draw(5),
                Part::Present,
            ]);
            [before, draw, after]
        })
        .collect()
}

/// Runs `body` on a thread of its own and fails when it has not ended within half a minute: a wait
/// that would freeze the game (main waiting for the render thread, which waits at a cut's mark for
/// the drawing side, which waits for main) fails the test instead of hanging the run.
fn within<T: Send + 'static>(what: &str, body: impl FnOnce() -> T + Send + 'static) -> T {
    let (done, told) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)));
    });
    match told.recv_timeout(Duration::from_secs(30)) {
        Ok(Ok(v)) => v,
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(_) => panic!("{what}: still waiting after 30 s: a freeze"),
    }
}

/// A flag threads wait for.
#[derive(Default)]
struct Latch(Mutex<bool>, std::sync::Condvar);

impl Latch {
    fn set(&self) {
        *self.0.lock().unwrap() = true;
        self.1.notify_all();
    }

    fn wait(&self) {
        let mut set = self.0.lock().unwrap();
        while !*set {
            set = self.1.wait(set).unwrap();
        }
    }
}

/// TEST DATA: a [`Log`] that holds the render thread at a gate: at a `SetRenderState` of state 99
/// (logged first) it says it arrived, and waits until the test opens the gate.
struct Gated {
    log: Log,
    arrived: Arc<Latch>,
    open: Arc<Latch>,
}

impl Backend for Gated {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        let gate = matches!(cmd, Cmd::SetRenderState { state: 99, .. });
        let reply = self.log.execute(cmd);
        if gate {
            self.arrived.set();
            self.open.wait();
        }
        reply
    }
}

/// [`Gated`]: the backend, the latch saying the render thread is at the gate, and the one that
/// opens it.
fn gated(log: Log) -> (Box<dyn Backend>, Arc<Latch>, Arc<Latch>) {
    let (arrived, open) = (Arc::new(Latch::default()), Arc::new(Latch::default()));
    let backend = Gated {
        log,
        arrived: Arc::clone(&arrived),
        open: Arc::clone(&open),
    };
    (Box::new(backend), arrived, open)
}

fn settings_to_try() -> Vec<Settings> {
    vec![
        Settings::default(),
        Settings {
            chunk_commands: 3,
            chunk_bytes: 300,
            check: true,
            ..Settings::default()
        },
        // No frame in flight and a queue past its limit at once: while a cut is open, main must
        // not wait for the render thread (which waits at the cut for the drawing side).
        Settings {
            frames_in_flight: 0,
            max_queued_bytes: 4 << 10,
            max_queued_chunks: 2,
            chunk_commands: 7,
            ..Settings::default()
        },
    ]
}

#[test]
fn two_producers_are_carried_out_in_the_serial_order() {
    let frames = three_part_frames(40);
    let log = Log::default();
    let lines = log.lines();
    let mut s = Stream::direct(Box::new(log));
    let serial_replies = serial(&mut s, &frames);
    drop(s);
    let serial_lines = lines.lock().unwrap().clone();
    assert!(
        serial_lines.len() > 4_000,
        "{} commands",
        serial_lines.len()
    );
    type Make = Box<dyn Fn(Box<dyn Backend>) -> Stream>;
    let mut runs: Vec<(String, Make)> = vec![("direct".into(), Box::new(Stream::direct))];
    for settings in settings_to_try() {
        let name = format!("threaded {settings:?}");
        runs.push((
            name,
            Box::new(move |b| Stream::threaded(b, settings.clone())),
        ));
    }
    for (name, make) in runs {
        let log = Log::default();
        let lines = log.lines();
        let mut s = make(Box::new(log));
        let replies = two_producers(&mut s, &frames);
        drop(s);
        same(&lines.lock().unwrap(), &serial_lines, &name);
        same(&replies, &serial_replies, &format!("{name}: the replies"));
    }
}

/// As with stepped threads, the drawing side records nothing until main has submitted its whole
/// part after the cut: a burst past the queue's limit, a flush into the full queue, and two
/// Presents with no frame in flight. None of it may wait for the render thread, which waits at the
/// cut's mark for the slot (in the game the drawing side needs the device lock main holds in each
/// call: the game would freeze). Then main waits for the slot as the layer does, and its sync call
/// after it answers every draw; the commands are carried out in the serial order on every runner.
#[test]
fn main_never_waits_for_the_render_thread_while_a_slot_is_open() {
    let small = Settings {
        frames_in_flight: 0,
        max_queued_bytes: 4 << 10,
        max_queued_chunks: 2,
        chunk_commands: 7,
        ..Settings::default()
    };
    let runs = [
        None,
        Some(Settings {
            frames_in_flight: 0,
            ..Settings::default()
        }),
        Some(small.clone()),
        Some(Settings {
            chunk_commands: 3,
            chunk_bytes: 300,
            check: true,
            ..small
        }),
    ];
    for settings in runs {
        let name = format!("{settings:?}");
        within(&name.clone(), move || {
            let frames = stepped_frames(3);
            let log = Log::default();
            let lines = log.lines();
            let mut s = Stream::direct(Box::new(log));
            let want_replies = serial(&mut s, &frames);
            drop(s);
            let want = lines.lock().unwrap().clone();
            let log = Log::default();
            let lines = log.lines();
            let mut s = match settings {
                None => Stream::direct(Box::new(log)),
                Some(settings) => Stream::threaded(Box::new(log), settings),
            };
            let replies = stepped(&mut s, &frames);
            drop(s);
            same(&lines.lock().unwrap(), &want, &name);
            same(&replies, &want_replies, &format!("{name}: the replies"));
        });
    }
}

/// The render thread a frame behind (held at a gate before the first cut's mark) while slots close
/// and the next cuts are made: a closed slot of several chunks with no sync call, an empty one, and
/// an open one whose sync call waits for the render thread wait on the queue behind each other,
/// with main's commands between them. Once the gate opens, everything is carried out in the serial
/// order: the slot's sync call answers the draws before it, main's after the slot every draw.
#[test]
fn closed_slots_wait_in_order_for_a_render_thread_behind() {
    let several = Settings {
        chunk_commands: 3,
        ..Settings::default()
    };
    for settings in [Settings::default(), several] {
        within("the render thread behind", move || behind(settings));
    }
}

/// [`closed_slots_wait_in_order_for_a_render_thread_behind`] with `settings`.
fn behind(settings: Settings) {
    let a = vec![Part::State(7, 1), Part::Draw(2), Part::State(99, 0)];
    let b: Vec<Part> = (0..10)
        .flat_map(|i| [Part::State(8, i), Part::Draw(1 + i)])
        .collect();
    let c = vec![Part::State(7, 2), Part::Draw(3)];
    let c2 = vec![Part::State(7, 3), Part::Draw(4)];
    let d1: Vec<Part> = (0..6)
        .flat_map(|i| [Part::Texture(0, 0x4000 + 8 * i), Part::Draw(5 + i)])
        .collect();
    let d2 = vec![Part::State(8, 0), Part::Draw(7)];
    let e: Vec<Part> = (0..8)
        .flat_map(|i| [Part::State(9, i), Part::Draw(1)])
        .collect();
    let (slot_read, main_read) = (Part::Read(0x8000), Part::Read(0x8001));
    // The serial order: A, slot 1 (B), C, slot 2 (empty), C2, slot 3 (D1, its read, D2), E, and
    // main's read after slot 3.
    let order: Vec<Part> = [
        &a[..],
        &b[..],
        &c[..],
        &c2[..],
        &d1[..],
        std::slice::from_ref(&slot_read),
        &d2[..],
        &e[..],
        std::slice::from_ref(&main_read),
    ]
    .concat();
    let (want, want_replies, want_file) = {
        let file = File::default();
        let log = Log::default();
        let lines = log.lines();
        let mut s = Stream::direct(Box::new(log));
        s.record_to(Box::new(file.clone())).unwrap();
        let replies: Vec<String> = order.iter().map(|p| send(&mut s, p)).collect();
        drop(s);
        let lines = lines.lock().unwrap().clone();
        let bytes = file.0.lock().unwrap().clone();
        (lines, replies, bytes)
    };
    let want_slot = &want_replies[a.len() + b.len() + c.len() + c2.len() + d1.len()];
    let want_main = want_replies.last().unwrap();

    let file = File::default();
    let log = Log::default();
    let lines = log.lines();
    let (backend, arrived, open) = gated(log);
    let mut s = Stream::threaded(backend, settings);
    s.record_to(Box::new(file.clone())).unwrap();
    for p in &a {
        s.submit(p.cmd());
    }
    // Cut 1 hands A over: the render thread carries it out and stops at the gate, before the mark.
    let mut slot = s.cut();
    arrived.wait();
    for p in &b {
        slot.submit(p.cmd());
    }
    slot.close();
    for p in &c {
        s.submit(p.cmd());
    }
    s.cut().close();
    for p in &c2 {
        s.submit(p.cmd());
    }
    let mut slot = s.cut();
    let (ready, told) = std::sync::mpsc::channel();
    let draw = std::thread::spawn(move || {
        for p in &d1 {
            slot.submit(p.cmd());
        }
        ready.send(()).unwrap();
        let answer = format!("{:?}", slot.submit(slot_read.cmd()));
        for p in &d2 {
            slot.submit(p.cmd());
        }
        slot.close();
        answer
    });
    for p in &e {
        s.submit(p.cmd());
    }
    told.recv().unwrap();
    assert_eq!(
        lines.lock().unwrap().len(),
        a.len(),
        "the render thread waits at the gate"
    );
    assert_eq!(
        s.queued_slots().len(),
        3,
        "the three cuts' slots wait on the queue"
    );
    open.set();
    let slot_answer = draw.join().unwrap();
    if let Some(waiter) = s.cut_waiter() {
        waiter.wait();
    }
    assert!(!s.cut_open());
    let main_answer = send(&mut s, &main_read);
    s.finish();
    drop(s);
    same(&lines.lock().unwrap(), &want, "the log");
    assert_eq!(
        &slot_answer, want_slot,
        "the slot's read: the draws before it"
    );
    assert_eq!(
        &main_answer, want_main,
        "main's read after the slot: every draw"
    );
    assert!(*file.0.lock().unwrap() == want_file, "the record differs");
}

#[test]
fn a_record_of_two_producers_is_the_serial_file() {
    let frames = three_part_frames(12);
    let record = |two: bool, make: &dyn Fn() -> Stream| {
        let file = File::default();
        let mut s = make();
        s.record_to(Box::new(file.clone())).unwrap();
        if two {
            two_producers(&mut s, &frames);
        } else {
            serial(&mut s, &frames);
        }
        drop(s);
        let bytes = file.0.lock().unwrap().clone();
        bytes
    };
    let direct = || Stream::direct(Box::new(Log::default()));
    let threaded = || Stream::threaded(Box::new(Log::default()), Settings::default());
    let want = record(false, &direct);
    assert!(want.len() > 100_000);
    assert!(record(true, &direct) == want, "direct: the file differs");
    assert!(
        record(true, &threaded) == want,
        "threaded: the file differs"
    );
}

/// `finish_recording` (`NV_REPEATABLE`'s end) right after a slot closed, with no command since:
/// main's commands held for the slot are written, and the file is the serial one.
#[test]
fn finish_recording_writes_main_s_commands_held_for_a_closed_slot() {
    let file_of = |two: Option<bool>| {
        let file = File::default();
        let mut s = match two {
            Some(true) => Stream::threaded(Box::new(Log::default()), Settings::default()),
            _ => Stream::direct(Box::new(Log::default())),
        };
        s.record_to(Box::new(file.clone())).unwrap();
        s.submit(Part::State(7, 1).cmd());
        if two.is_some() {
            let mut slot = s.cut();
            s.submit(Part::State(7, 3).cmd());
            s.submit(Part::Draw(1).cmd());
            slot.submit(Part::State(7, 2).cmd());
            slot.close();
        } else {
            for p in [Part::State(7, 2), Part::State(7, 3), Part::Draw(1)] {
                s.submit(p.cmd());
            }
        }
        s.finish_recording();
        let bytes = file.0.lock().unwrap().clone();
        drop(s);
        bytes
    };
    let want = file_of(None);
    for threaded in [false, true] {
        assert!(
            file_of(Some(threaded)) == want,
            "threaded {threaded}: the file differs"
        );
    }
}

/// The direct runner shares its backend with a slot only while the cut is open: once main sees the
/// slot closed, the backend is its own again (each call without the shared lock).
#[test]
fn the_direct_runner_has_its_backend_alone_again_after_a_cut() {
    let mut s = Stream::direct(Box::new(Log::default()));
    let mut slot = s.cut();
    assert!(s.backend_shared());
    slot.submit(Part::Draw(1).cmd());
    slot.close();
    assert!(s.backend_shared(), "until main sees the slot closed");
    s.settle();
    assert!(!s.backend_shared());
    drop(s.cut());
    s.submit(Part::Draw(2).cmd());
    assert!(!s.backend_shared(), "a dropped slot, seen at the next call");
}

#[test]
fn the_state_digests_of_two_producers_are_the_serial_ones() {
    let frames = three_part_frames(20);
    let (digest, serial_digests) = StateDigest::new();
    let mut s = Stream::direct(Box::new(digest));
    serial(&mut s, &frames);
    drop(s);
    let want = serial_digests.lock().unwrap().clone();
    assert_eq!(
        want.len(),
        draws_in(&frames) + 1,
        "a digest a draw, and the destroys'"
    );
    for threaded in [false, true] {
        let (digest, digests) = StateDigest::new();
        let mut s = if threaded {
            Stream::threaded(Box::new(digest), Settings::default())
        } else {
            Stream::direct(Box::new(digest))
        };
        two_producers(&mut s, &frames);
        drop(s);
        assert!(
            *digests.lock().unwrap() == want,
            "threaded {threaded}: the digests differ"
        );
    }
}

/// The digests of `cmds` carried out one by one (the destroys' value last).
fn digests_of(cmds: &[Cmd<Data<'_>>]) -> Vec<u64> {
    let (mut d, list) = StateDigest::new();
    for &c in cmds {
        d.execute(c);
    }
    drop(d);
    let v = list.lock().unwrap().clone();
    v
}

/// The digest sees a draw's state, not the commands that made it: a state set again to its value
/// changes nothing; any other change to what a draw sees changes that draw's digest, none before.
#[test]
fn a_state_digest_changes_with_what_a_draw_sees_only() {
    let digests = |parts: &[Part]| digests_of(&parts.iter().map(Part::cmd).collect::<Vec<_>>());
    let base = vec![
        Part::State(7, 1),
        Part::Constants(4, vec![1, 2, 3, 4, 5, 6, 7, 8]),
        Part::Draw(3),
        Part::Texture(0, 0x4000),
        Part::Draw(3),
    ];
    let want = digests(&base);
    assert_eq!(want.len(), 3);
    assert_ne!(want[0], want[1], "the texture changes the second draw");
    // The same state set twice, a constant register set again to its value: the same digests.
    let mut again = base.clone();
    again.insert(1, Part::State(7, 1));
    again.insert(4, Part::Constants(5, vec![5, 6, 7, 8]));
    assert_eq!(digests(&again), want);
    // Another value, another register, an upload before a draw: the digests differ from there.
    for (at, part) in [
        (1, Part::State(7, 2)),
        (1, Part::Constants(6, vec![0, 0, 0, 0])),
        (4, Part::Upload(0x6000, vec![1])),
    ] {
        let mut other = base.clone();
        other.insert(at, part);
        let got = digests(&other);
        let first = if at < 3 { 0 } else { 1 };
        assert_eq!(got[..first], want[..first]);
        assert_ne!(got[first], want[first], "inserted at {at}");
    }
}

/// A new render target 0 sets the viewport and the scissor rectangle to its whole size, and
/// `Reset` every state to the device's, so what was set before them is not what a draw after
/// them sees. The query reads and the windows' handles change no digest; the destroys count as a
/// set, in the last value.
#[test]
fn a_state_digest_resets_as_the_device_does() {
    let viewport = Cmd::SetViewport {
        device: 0x1000,
        viewport: [0, 0, 100, 100, 0, 1f32.to_bits()],
    };
    let scissor = Cmd::SetScissorRect {
        device: 0x1000,
        rect: [0, 0, 100, 100],
    };
    let target = |index: u32| Cmd::SetRenderTarget {
        device: 0x1000,
        index,
        surface: 0x9000 + index,
    };
    let parts = [Part::Draw(1), Part::State(7, 1), Part::Read(0x8000)];
    let (draw, state, read) = (parts[0].cmd(), parts[1].cmd(), parts[2].cmd());
    for set in [viewport, scissor] {
        assert_ne!(
            digests_of(&[set, target(0), draw]),
            digests_of(&[set, target(0), set, draw]),
            "set before a target change: {}",
            set.name()
        );
        assert_ne!(
            digests_of(&[target(0), set, target(0), draw]),
            digests_of(&[target(0), set, draw]),
            "the same target set again: {}",
            set.name()
        );
        assert_eq!(
            digests_of(&[set, target(1), draw]),
            digests_of(&[set, target(1), set, draw]),
            "another target index leaves {} alone",
            set.name()
        );
    }
    let present: Vec<u32> = vec![0; 14];
    let reset = |window: isize| Cmd::Reset {
        device: 0x1000,
        window,
        present: Data::from_words(&present),
        back_buffers: Data::from_words(&[0x9000]),
        depth_stencil: 0,
    };
    assert_ne!(
        digests_of(&[state, reset(5), state, draw]),
        digests_of(&[state, reset(5), draw]),
        "a state set before Reset is not set after it"
    );
    assert_eq!(
        digests_of(&[reset(5), draw]),
        digests_of(&[reset(6), draw]),
        "the window's handle"
    );
    assert_eq!(
        digests_of(&[state, read, draw, read]),
        digests_of(&[state, draw]),
        "the query reads"
    );
    let destroy = |object: u32| Cmd::Destroy { object };
    assert_eq!(
        digests_of(&[destroy(1), draw, destroy(2), draw]),
        digests_of(&[draw, destroy(2), draw, destroy(1)]),
        "the destroys in another order"
    );
    assert_ne!(
        digests_of(&[destroy(1), draw]),
        digests_of(&[draw]),
        "a destroy left out"
    );
}

#[test]
fn a_dropped_slot_closes_and_main_goes_on() {
    for threaded in [false, true] {
        let log = Log::default();
        let lines = log.lines();
        let mut s = if threaded {
            Stream::threaded(Box::new(log), Settings::default())
        } else {
            Stream::direct(Box::new(log))
        };
        s.submit(Part::State(7, 1).cmd());
        let mut slot = s.cut();
        s.submit(Part::State(7, 3).cmd());
        slot.submit(Part::State(7, 2).cmd());
        drop(slot);
        assert!(!s.cut_open(), "threaded {threaded}");
        s.submit(Part::Draw(1).cmd());
        s.finish();
        let got: Vec<String> = lines.lock().unwrap().clone();
        let want: Vec<String> = [
            Part::State(7, 1),
            Part::State(7, 2),
            Part::State(7, 3),
            Part::Draw(1),
        ]
        .iter()
        .map(|p| format!("{:?}", p.cmd()))
        .collect();
        same(&got, &want, &format!("threaded {threaded}"));
        let (cut, _) = s.last_cut_carried_out().expect("the cut was carried out");
        assert_eq!(cut, 1);
    }
}

/// The stream dropped while a slot is open (in the game the drawing job is joined first): the
/// drawing side is not left waiting. What it records once the stream went on without it is
/// dropped, its sync call answers nothing and closing it returns; main's commands after the cut
/// are carried out.
#[test]
fn a_slot_left_open_when_the_stream_goes_never_waits() {
    within("a slot after its stream", || {
        let log = Log::default();
        let lines = log.lines();
        let mut s = Stream::threaded(Box::new(log), Settings::default());
        s.submit(Part::Draw(1).cmd());
        let mut slot = s.cut();
        slot.submit(Part::Draw(2).cmd());
        s.submit(Part::Draw(3).cmd());
        drop(s);
        slot.submit(Part::Draw(4).cmd());
        assert!(matches!(slot.submit(Part::Read(0x8000).cmd()), Reply::None));
        slot.close();
        let want: Vec<String> = [Part::Draw(1), Part::Draw(3)]
            .iter()
            .map(|p| format!("{:?}", p.cmd()))
            .collect();
        same(&lines.lock().unwrap(), &want, "main's commands");
    });
}

/// ... and a slot waiting for its sync call's answer as the stream goes gets it: the render thread
/// carries out what was handed over before it goes on without the slot.
#[test]
fn a_slot_waiting_for_an_answer_as_the_stream_goes_gets_it() {
    within("a slot's answer as its stream goes", || {
        let log = Log::default();
        let lines = log.lines();
        let (backend, arrived, open) = gated(log);
        let mut s = Stream::threaded(backend, Settings::default());
        s.submit(Part::State(99, 0).cmd());
        let mut slot = s.cut();
        arrived.wait();
        let draw = std::thread::spawn(move || {
            slot.submit(Part::Draw(2).cmd());
            let answer = format!("{:?}", slot.submit(Part::Read(0x8000).cmd()));
            slot.submit(Part::Draw(4).cmd());
            slot.close();
            answer
        });
        // The read is on the queue (the slot's only chunk before its close) ...
        while s.queued_slots() != [1] {
            std::thread::yield_now();
        }
        s.submit(Part::Draw(3).cmd());
        // ... when the stream goes (its drop waits for the render thread, held at the gate).
        let dropping = std::thread::spawn(move || drop(s));
        std::thread::sleep(Duration::from_millis(20));
        open.set();
        dropping.join().unwrap();
        let answer = draw.join().unwrap();
        assert_eq!(answer, format!("{:?}", Reply::Query(Some((0, 1)))));
        let line = |p: Part| format!("{:?}", p.cmd());
        let got = lines.lock().unwrap().clone();
        let mut want = vec![
            line(Part::State(99, 0)),
            line(Part::Draw(2)),
            line(Part::Read(0x8000)),
        ];
        // The slot's last draw comes too when the slot closed before the render thread went on.
        if got.len() == 5 {
            want.push(line(Part::Draw(4)));
        }
        want.push(line(Part::Draw(3)));
        same(&got, &want, "the log");
    });
}

/// A panic of the render thread's own (its slot bookkeeping, not the backend) stops both producers
/// with its message, as a backend's panic does: neither waits for it forever.
#[test]
fn a_panic_at_a_mark_stops_both_producers_with_its_message() {
    within("a panic at a mark", || {
        let mut s = Stream::threaded(Box::new(Log::default()), Settings::default());
        s.fail_at_next_mark();
        s.submit(Part::Draw(1).cmd());
        let mut slot = s.cut();
        let draw = std::thread::spawn(move || {
            slot.submit(Part::Draw(2).cmd());
            slot.submit(Part::Read(0x8000).cmd());
        });
        let why = "a test's panic in the render thread's slot bookkeeping";
        let drawing = crate::runner::panic_message(&*draw.join().expect_err("the drawing side"));
        assert!(drawing.contains(why), "{drawing}");
        let main = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.finish()));
        let main = crate::runner::panic_message(&*main.expect_err("main stops"));
        assert!(main.contains(why), "{main}");
    });
}

#[test]
#[should_panic(expected = "main's sync call")]
fn a_sync_call_of_mains_while_a_slot_is_open_panics() {
    let mut s = Stream::threaded(Box::new(Log::default()), Settings::default());
    let _slot = s.cut();
    s.submit(Part::Read(0x8000).cmd());
}

#[test]
#[should_panic(expected = "a cut is open")]
fn a_second_cut_before_the_slot_closes_panics() {
    let mut s = Stream::direct(Box::new(Log::default()));
    let _slot = s.cut();
    let _again = s.cut();
}
