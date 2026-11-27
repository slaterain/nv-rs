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

#[test]
fn a_reply_taken_later_sees_the_commands_before_it_and_keeps_to_its_caller() {
    let draw = || Cmd::DrawIndexedPrimitive {
        device: 0x1000,
        kind: 4,
        base_vertex: 0,
        min_index: 0,
        vertices: 3,
        start_index: 0,
        count: 1,
    };
    let rows = |reply: Option<Reply>| match reply {
        Some(Reply::Rows(Some(r))) => r.bytes,
        other => panic!("{other:?}"),
    };
    let runners: [fn() -> Stream; 3] = [
        || Stream::direct(Box::new(Log::default())),
        || {
            // The upload before the read-back is slow: its reply is still on its way when the query asks.
            let log = Log {
                slow_upload: Some(Duration::from_millis(30)),
                ..Log::default()
            };
            Stream::threaded(Box::new(log), Settings::default())
        },
        || {
            let tiny = Settings {
                chunk_commands: 2,
                chunk_bytes: 300,
                ..Settings::default()
            };
            Stream::threaded(Box::new(Log::default()), tiny)
        },
    ];
    let case = move |i: usize, make: fn() -> Stream| {
        let mut stream = make();
        assert!(stream.take_later().is_none(), "{i}: none asked for");
        // Two draws, the read-back (the backend answers rows of its draw count), three more draws and a
        // query (it answers the draw count): the query's reply is its own, the read-back's keeps its place.
        stream.submit(draw());
        stream.submit(draw());
        stream.submit(Cmd::UploadSurface {
            surface: 0x7000,
            row: 4,
            rows: 1,
            data: Data::Bytes(&[1, 2, 3, 4]),
        });
        stream.submit_later(Cmd::GetRenderTargetData {
            device: 0x1000,
            source: 0x2000,
            destination: 0x7000,
            pitch: 8,
        });
        assert!(stream.later_waiting(), "{i}");
        for _ in 0..3 {
            stream.submit(draw());
        }
        match stream.submit(Cmd::QueryGetData {
            query: 0x8000,
            flags: 1,
        }) {
            Reply::Query(Some((0, 5))) => {}
            other => panic!("{i}: {other:?}"),
        }
        assert_eq!(rows(stream.take_later()), vec![2; 8], "{i}");
        assert!(
            !stream.later_waiting() && stream.take_later().is_none(),
            "{i}: taken once"
        );
        // Taken before any other sync command: waited for.
        stream.submit_later(Cmd::GetRenderTargetData {
            device: 0x1000,
            source: 0x2000,
            destination: 0x7000,
            pitch: 4,
        });
        stream.submit(draw());
        assert_eq!(rows(stream.take_later()), vec![5; 4], "{i}");
    };
    for (i, make) in runners.into_iter().enumerate() {
        // On a thread of its own, waited for a bounded time: a reply lost fails the test (review of
        // #1037), not hangs it.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            case(i, make);
            let _ = tx.send(());
        });
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(()) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                panic!("{i}: failed (its thread's panic above)")
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                panic!("{i}: no answer in 20 s: a reply lost")
            }
        }
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
    // Each upload passes chunk_bytes, so the recording side handed chunks over as it went (#510's
    // Threaded::record must hand over at the chunk's limits, as Threaded::submit does).
    assert!(most > 0, "nothing handed over before finish");
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
        Cmd::RecordNext {
            device: d,
            id: 5,
            forget: 4,
            check: 1,
        },
        Cmd::DrawItem {
            device: d,
            id: 5,
            check: 0,
            constants: w(&p.registers),
        },
        Cmd::ForgetRecords {
            device: d,
            ids: w(&p.handles),
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

/// The per-draw digests name objects by first use too: the same drawing with every object elsewhere has the same
/// digest at its draw (the gate of 2026-11-21 saw only two buffers' handles differ), and a real difference (the buffer
/// bound where the texture was) still gives another. The counts list each command with its bytes.
#[test]
fn draw_digests_and_counts_see_objects_by_first_use() {
    let a = Objects {
        device: 0x2000_0010,
        texture: 0x2000_0020,
        levels: [0x2000_0100, 0x2000_0108],
        buffer: 0x2000_0030,
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
    let da = wire::draw_digests(&ra[..]).unwrap();
    assert_eq!(
        da.len(),
        2,
        "the draw, then the file's end (what was destroyed)"
    );
    assert_eq!(da, wire::draw_digests(&rb[..]).unwrap());
    let sa = wire::draw_snapshot(&ra[..], 0).unwrap().expect("a draw");
    let sb = wire::draw_snapshot(&rb[..], 0).unwrap().expect("a draw");
    assert_eq!(sa.state, sb.state);
    assert!(wire::draw_snapshot(&ra[..], 1).unwrap().is_none());
    let swapped = Objects {
        bound: 0x2400_0050,
        ..b
    };
    assert_ne!(da, wire::draw_digests(&drawing(&swapped, 2)[..]).unwrap());
    assert_ne!(da, wire::draw_digests(&drawing(&b, 3)[..]).unwrap());
    assert!(wire::draw_digests(&ra[..ra.len() - 3]).is_err());

    let counts = wire::command_counts(&ra[..]).unwrap();
    let by_name: std::collections::BTreeMap<_, _> =
        counts.iter().map(|(name, n, _)| (*name, *n)).collect();
    let expected: std::collections::BTreeMap<_, _> = [
        ("CreateTexture", 1),
        ("SetRenderTarget", 1),
        ("SetTexture", 1),
        ("SetStreamSource", 2),
        ("DrawPrimitive", 1),
        ("Destroy", 1),
        ("Present", 1),
    ]
    .into_iter()
    .collect();
    assert_eq!(by_name, expected);
    let header = wire::header().len() as u64;
    assert_eq!(
        counts.iter().map(|c| c.2).sum::<u64>(),
        ra.len() as u64 - header,
        "every record's bytes, its length word included"
    );
    assert_eq!(counts, wire::command_counts(&rb[..]).unwrap());
    assert!(wire::command_counts(&b"not a stream at all"[..]).is_err());
}

/// One step of [`regrown`]'s records: a buffer made at an address, bound and drawn from, or ended.
#[derive(Clone, Copy)]
enum Step {
    Make(u32),
    Draw(u32),
    End(u32),
}

/// A record of `steps` (a dynamic vertex buffer's growth: the old one ended, a bigger one made).
fn regrown(steps: &[Step]) -> Vec<u8> {
    let device = 0x2000_0010;
    let mut cmds = Vec::new();
    let mut made = 0;
    for step in steps {
        match *step {
            Step::Make(buffer) => {
                cmds.push(Cmd::CreateBuffer {
                    device,
                    buffer,
                    index: false,
                    length: if made == 0 { 2736 } else { 144_000 },
                    usage: 0x208,
                    format: 0,
                    pool: 0,
                });
                made += 1;
            }
            Step::Draw(buffer) => {
                cmds.push(Cmd::SetStreamSource {
                    device,
                    stream: 0,
                    buffer,
                    offset: 0,
                    stride: 36,
                });
                cmds.push(Cmd::DrawPrimitive {
                    device,
                    kind: 4,
                    start: 0,
                    count: 2,
                });
            }
            Step::End(object) => cmds.push(Cmd::Destroy { object }),
        }
    }
    file_of(&cmds)
}

/// With the world's draw on the drawing thread an object the drawing side lets go of ends at the
/// join, so in the posted run's record its `Destroy` comes later and the bigger buffer the serial run
/// makes at the freed address lies elsewhere (the late gate's second difference, 2026-11-16): the
/// plain comparison stops there, the one with deferred ends finds the records alike. It still finds
/// an end earlier in B, an end missing from one record, and a use of an object after its end.
#[test]
fn records_compare_with_deferred_ends() {
    use Step::{Draw, End, Make};
    let (x, y) = (0x2400_0100u32, 0x2400_0200u32);
    let serial = regrown(&[Make(x), Draw(x), End(x), Make(x), Draw(x)]);
    let posted = regrown(&[Make(x), Draw(x), Make(y), Draw(y), End(x)]);
    let (_, plain) = wire::compare_renamed(&serial[..], &posted[..]).unwrap();
    assert_eq!(
        plain.expect("a difference").index,
        3,
        "the plain one stops at the end"
    );
    let (alike, diff, ends) =
        wire::compare_renamed_ends_deferred(&serial[..], &posted[..]).unwrap();
    assert_eq!(diff, None);
    assert_eq!((alike, ends.later), (6, 1));
    // An end earlier in B than in A.
    let (_, diff, _) = wire::compare_renamed_ends_deferred(&posted[..], &serial[..]).unwrap();
    assert!(diff
        .expect("earlier")
        .a
        .expect("why")
        .contains("earlier in B"));
    // An end in one record only.
    let kept = regrown(&[Make(x), Draw(x), Make(y), Draw(y)]);
    let (_, diff, _) = wire::compare_renamed_ends_deferred(&serial[..], &kept[..]).unwrap();
    assert!(diff
        .expect("one only")
        .a
        .expect("why")
        .contains("one record only"));
    // B draws from the ended buffer: another name there.
    let stale = regrown(&[Make(x), Draw(x), End(x), Make(y), Draw(x)]);
    let (alike, diff, _) = wire::compare_renamed_ends_deferred(&serial[..], &stale[..]).unwrap();
    assert_eq!((alike, diff.map(|d| d.index)), (4, Some(5)));
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

/// What a `Clear` clears into goes into its hash: a wrong render target, depth surface, viewport, scissor rectangle
/// or scissor test seen only by a `Clear` (set right again before the next draw) gives that draw another digest; a
/// state the `Clear` does not reach, set otherwise before it and right again after, gives the same.
#[test]
fn a_clear_is_hashed_with_what_it_clears_into() {
    let device = 0x1000;
    let clear = Cmd::Clear {
        device,
        count: 0,
        rects: None,
        flags: 7,
        colour: 0xff00_00ff,
        z: 1f32.to_bits(),
        stencil: 0,
        model: 0,
    };
    let draw = Cmd::DrawIndexedPrimitive {
        device,
        kind: 4,
        base_vertex: 0,
        min_index: 0,
        vertices: 3,
        start_index: 0,
        count: 1,
    };
    let target = |index: u32, surface: u32| Cmd::SetRenderTarget {
        device,
        index,
        surface,
    };
    let depth = |surface: u32| Cmd::SetDepthStencilSurface { device, surface };
    let viewport = |w: u32| Cmd::SetViewport {
        device,
        viewport: [0, 0, w, 100, 0, 1f32.to_bits()],
    };
    let scissor = |w: i32| Cmd::SetScissorRect {
        device,
        rect: [0, 0, w, 100],
    };
    let test = |value: u32| Cmd::SetRenderState {
        device,
        state: 174,
        value,
    };
    let pairs = [
        // Target 0 sets the viewport and the scissor rectangle too; target 1 only itself.
        ("render target 0", target(0, 0x9000), target(0, 0x9001)),
        ("render target 1", target(1, 0x9000), target(1, 0x9001)),
        ("depth surface", depth(0x9100), depth(0x9101)),
        ("viewport", viewport(50), viewport(100)),
        ("scissor rectangle", scissor(50), scissor(100)),
        ("scissor test", test(0), test(1)),
    ];
    for (what, wrong, right) in pairs {
        assert_ne!(
            digests_of(&[wrong, clear, right, draw]),
            digests_of(&[right, clear, right, draw]),
            "the {what} the clear saw"
        );
        assert_eq!(
            digests_of(&[right, clear, draw]),
            digests_of(&[right, right, clear, draw]),
            "the {what} set again before the clear"
        );
    }
    let state = |value: u32| Cmd::SetRenderState {
        device,
        state: 7,
        value,
    };
    assert_eq!(
        digests_of(&[state(1), clear, state(2), draw]),
        digests_of(&[state(2), clear, state(2), draw]),
        "a state the clear does not reach"
    );
}

/// `DrawPrimitiveUP` leaves stream 0 empty, as the device does: the stream set again to the buffer it held before
/// that draw gives the next draw another digest than leaving it out, which would draw from no buffer.
#[test]
fn a_draw_from_memory_empties_stream_0() {
    let device = 0x1000;
    let stream = Cmd::SetStreamSource {
        device,
        stream: 0,
        buffer: 0x6000,
        offset: 0,
        stride: 16,
    };
    let vertices = [0u32; 12];
    let up = Cmd::DrawPrimitiveUp {
        device,
        kind: 4,
        count: 1,
        stride: 16,
        vertices: Data::from_words(&vertices),
    };
    let draw = Cmd::DrawIndexedPrimitive {
        device,
        kind: 4,
        base_vertex: 0,
        min_index: 0,
        vertices: 3,
        start_index: 0,
        count: 1,
    };
    let set_again = digests_of(&[stream, up, stream, draw]);
    let left_out = digests_of(&[stream, up, draw]);
    assert_eq!(set_again.len(), 3, "two draws, then the destroys");
    assert_eq!(set_again[0], left_out[0], "the draw from memory itself");
    assert_ne!(set_again[1], left_out[1], "the draw after it");
    // Before the draw from memory nothing changes: the stream set twice is one state.
    assert_eq!(digests_of(&[stream, stream, up, stream, draw]), set_again);
}

/// A state that holds an object set to none is the device's default, as if never set: a sampler, a shader, the index
/// buffer or a stream set to none (where a cache on the other side skipped the set) draws alike. An object on one side
/// and none (or nothing) on the other still differ, as does an object left in place against one set to none after it,
/// a stream set to no buffer with a stride, and a set to none against an object set again after a `Reset`.
#[test]
fn a_state_set_to_no_object_is_never_set() {
    let device = 0x1000;
    let draw = Part::Draw(1).cmd();
    let present: Vec<u32> = vec![0; 14];
    let reset = Cmd::Reset {
        device,
        window: 0,
        present: Data::from_words(&present),
        back_buffers: Data::from_words(&[0x9000]),
        depth_stencil: 0,
    };
    let texture = |object: u32| Cmd::SetTexture {
        device,
        sampler: 7,
        texture: object,
    };
    let shader = |object: u32| Cmd::SetShader {
        device,
        pixel: true,
        shader: object,
    };
    let indices = |object: u32| Cmd::SetIndices {
        device,
        buffer: object,
    };
    let stream = |object: u32| Cmd::SetStreamSource {
        device,
        stream: 1,
        buffer: object,
        offset: 0,
        stride: if object == 0 { 0 } else { 16 },
    };
    for (name, set) in [
        ("texture", &texture as &dyn Fn(u32) -> Cmd<Data<'static>>),
        ("shader", &shader),
        ("indices", &indices),
        ("stream", &stream),
    ] {
        let never = digests_of(&[draw]);
        assert_eq!(
            digests_of(&[set(0), draw]),
            never,
            "{name}: none is never set"
        );
        assert_eq!(
            digests_of(&[set(0x4000), set(0), draw]),
            never,
            "{name}: an object then none"
        );
        assert_ne!(digests_of(&[set(0x4000), draw]), never, "{name}: an object");
        assert_ne!(
            digests_of(&[set(0x4000), set(0), draw]),
            digests_of(&[set(0x4000), draw]),
            "{name}: an object left in place against one set to none"
        );
        assert_ne!(
            digests_of(&[set(0x4000), draw]),
            digests_of(&[set(0x4001), draw]),
            "{name}: two objects"
        );
        assert_eq!(
            digests_of(&[set(0x4000), reset, set(0), draw]),
            digests_of(&[set(0x4000), reset, draw]),
            "{name}: none after a Reset"
        );
        assert_ne!(
            digests_of(&[set(0x4000), reset, set(0x4000), draw]),
            digests_of(&[set(0x4000), reset, set(0), draw]),
            "{name}: an object set again after a Reset"
        );
    }
    let strided = Cmd::SetStreamSource {
        device,
        stream: 1,
        buffer: 0,
        offset: 0,
        stride: 16,
    };
    assert_ne!(
        digests_of(&[strided, draw]),
        digests_of(&[draw]),
        "no buffer with a stride is not the default"
    );
}

/// The per-draw digests name objects by first use across a `Destroy`: a buffer's address used again for a new buffer
/// (record `a`) draws as a new buffer at another address does (record `b`), when the stream is set again to it; a
/// record that keeps the old binding without setting it again, whose raw handles at the draw match `a`'s, gives that
/// draw another digest.
#[test]
fn draw_digests_see_an_address_used_again_as_a_new_object() {
    let record = |device: u32, first: u32, again: u32, set_again: bool| {
        let stream = |buffer: u32| Cmd::SetStreamSource {
            device,
            stream: 0,
            buffer,
            offset: 0,
            stride: 32,
        };
        let draw = Cmd::DrawPrimitive {
            device,
            kind: 4,
            start: 0,
            count: 1,
        };
        let mut cmds = vec![stream(first), draw, Cmd::Destroy { object: first }];
        if set_again {
            cmds.push(stream(again));
        }
        cmds.push(draw);
        file_of(&cmds)
    };
    let a = wire::draw_digests(&record(0x2000_0010, 0x2000_0030, 0x2000_0030, true)[..]).unwrap();
    let b = wire::draw_digests(&record(0x2400_0010, 0x2400_0050, 0x2400_0060, true)[..]).unwrap();
    assert_eq!(a.len(), 3, "two draws, then the destroys");
    assert_eq!(a, b);
    let kept =
        wire::draw_digests(&record(0x2000_0010, 0x2000_0030, 0x2000_0030, false)[..]).unwrap();
    assert_eq!(kept[0], a[0]);
    assert_ne!(kept[1], a[1], "the old binding of the address used again");
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

/// The last Present the backend carried out, with when: on the direct runner at once; on the
/// threaded one once the render thread has carried it out (all of them after `finish`).
#[test]
fn the_last_present_carried_out_is_known_with_when() {
    for threaded in [false, true] {
        let mut s = if threaded {
            Stream::threaded(Box::new(Log::default()), Settings::default())
        } else {
            Stream::direct(Box::new(Log::default()))
        };
        assert!(s.last_present_carried_out().is_none());
        let mut before = std::time::Instant::now();
        for _ in 0..3 {
            before = std::time::Instant::now();
            s.submit(Part::Present.cmd());
        }
        s.finish();
        let (frame, at) = s.last_present_carried_out().expect("carried out");
        assert_eq!(frame, 3, "threaded {threaded}");
        assert!(at >= before, "threaded {threaded}: after its submit began");
    }
}

/// TEST DATA: a backend whose presents submit for `submit` (a blocking call, timed as one) and then work `own` more.
struct SlowSubmit {
    submit: Duration,
    own: Duration,
}

impl Backend for SlowSubmit {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        if let Cmd::Present { .. } = cmd {
            hitch::timed(hitch::Call::Submit, || std::thread::sleep(self.submit));
            std::thread::sleep(self.own);
        }
        Reply::None
    }
}

/// What the render thread spent a frame on is counted before main's wait in `Present` for that frame ends: its time
/// on the game's commands, its blocking calls by part, and its longest call by name. Main's own calls are not timed.
#[test]
fn the_render_threads_frame_is_counted_before_main_stops_waiting_for_it() {
    let _one = hitch::tests::SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    hitch::set_on(true);
    let _ = hitch::take_longest();
    let mut s = Stream::threaded(
        Box::new(SlowSubmit {
            submit: Duration::from_millis(30),
            own: Duration::from_millis(20),
        }),
        Settings::default(),
    );
    let before = hitch::snapshot();
    // One frame in flight: the second Present returns once the first is carried out.
    s.submit(Part::Present.cmd());
    s.submit(Part::Present.cmd());
    let now = hitch::snapshot();
    let d = |c: hitch::Count| now[c as usize] - before[c as usize];
    assert!(
        d(hitch::Count::RenderSubmitNs) >= 30_000_000,
        "{}",
        d(hitch::Count::RenderSubmitNs)
    );
    assert!(
        d(hitch::Count::RenderBusyNs) >= 50_000_000,
        "{}",
        d(hitch::Count::RenderBusyNs)
    );
    let [render, _] = hitch::take_longest();
    let (call, ns) = render.expect("the render thread's longest call");
    assert_eq!(call, hitch::Call::Submit);
    assert!(ns >= 30_000_000, "{ns}");
    // Main is not one of the timed threads.
    assert!(!hitch::timing());
    s.finish();
}

/// TEST DATA: a backend whose first idle turn polls for 30 ms (a blocking call, timed as one).
struct IdlePoll {
    turns: Arc<AtomicU64>,
}

impl Backend for IdlePoll {
    fn execute(&mut self, _cmd: Cmd<Data<'_>>) -> Reply {
        Reply::None
    }

    fn idle(&mut self) -> bool {
        if self.turns.fetch_add(1, Ordering::SeqCst) == 0 {
            hitch::timed(hitch::Call::Poll, || {
                std::thread::sleep(Duration::from_millis(30))
            });
        }
        false
    }
}

/// The render thread's idle work is counted whole as its idle time; its calls there are in none
/// of the parts of its time on the game's commands (so no part is larger than that time), and
/// the longest of them is still named.
#[test]
fn the_render_threads_idle_calls_stay_out_of_its_commands_parts() {
    let _one = hitch::tests::SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    hitch::set_on(true);
    let _ = hitch::take_longest();
    let turns = Arc::new(AtomicU64::new(0));
    let before = hitch::snapshot();
    let mut s = Stream::threaded(
        Box::new(IdlePoll {
            turns: Arc::clone(&turns),
        }),
        Settings::default(),
    );
    let start = std::time::Instant::now();
    while turns.load(Ordering::SeqCst) == 0 {
        assert!(start.elapsed() < Duration::from_secs(10), "no idle turn");
        std::thread::sleep(Duration::from_millis(1));
    }
    // Carried out after the idle turn ended (and handed its counts over).
    s.submit(Part::Present.cmd());
    s.finish();
    let now = hitch::snapshot();
    let d = |c: hitch::Count| now[c as usize] - before[c as usize];
    assert_eq!(
        d(hitch::Count::RenderPollNs),
        0,
        "the idle poll is in no part"
    );
    assert!(
        d(hitch::Count::RenderIdleNs) >= 30_000_000,
        "{}",
        d(hitch::Count::RenderIdleNs)
    );
    let [render, _] = hitch::take_longest();
    let (call, ns) = render.expect("the render thread's longest call");
    assert_eq!(call, hitch::Call::Poll);
    assert!(ns >= 30_000_000, "{ns}");
}

/// A file written before the kept draw records' commands were added reads as ever: its list's hash
/// ([`wire::OLDER_LIST_HASH`], a recording's header of 2026-11-20) is the hash of today's list up to
/// them, and both hashes are read.
#[test]
fn files_written_before_the_kept_records_commands_still_read() {
    let list = crate::cmd::LIST;
    let at = list
        .find("RecordNext")
        .expect("the kept records' first command");
    let older = list[..at].trim_end_matches([',', ' ']);
    assert!(older.ends_with("QueryGetData { query : Handle, flags : u32 }"));
    assert_eq!(
        wire::hash(wire::HASH_START, older.as_bytes()),
        wire::OLDER_LIST_HASH
    );
    let mut file = wire::header().to_vec();
    file[8..].copy_from_slice(&wire::OLDER_LIST_HASH.to_le_bytes());
    wire::encode(
        &Cmd::SetRenderState {
            device: 1,
            state: 7,
            value: 1,
        },
        &mut file,
    );
    let read: Vec<_> = wire::commands(&file)
        .expect("the older list is read")
        .collect::<Result<_, _>>()
        .expect("its command");
    assert_eq!(read.len(), 1);
    file[8] ^= 1;
    assert!(wire::commands(&file).is_err(), "another list is refused");
}

/// TEST DATA: a draw's own commands (a stream source, indices, two constant sets, the draw), its
/// registers `regs` (two vertex registers, then one pixel register).
fn kept_draw(regs: &[u32; 12]) -> Vec<(Cmd<()>, Vec<u8>)> {
    let bytes = |w: &[u32]| w.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>();
    vec![
        (
            Cmd::SetStreamSource {
                device: 1,
                stream: 0,
                buffer: 0x50,
                offset: 16,
                stride: 32,
            },
            Vec::new(),
        ),
        (
            Cmd::SetIndices {
                device: 1,
                buffer: 0x60,
            },
            Vec::new(),
        ),
        (
            Cmd::SetConstants {
                device: 1,
                kind: 0,
                start: 4,
                count: 2,
                data: (),
            },
            bytes(&regs[..8]),
        ),
        (
            Cmd::SetConstants {
                device: 1,
                kind: 3,
                start: 1,
                count: 1,
                data: (),
            },
            bytes(&regs[8..]),
        ),
        (
            Cmd::DrawIndexedPrimitive {
                device: 1,
                kind: 4,
                base_vertex: 0,
                min_index: 0,
                vertices: 30,
                start_index: 6,
                count: 10,
            },
            Vec::new(),
        ),
    ]
}

/// Runs `cmds` (each with its payload, when it has one) into `b`.
fn run_kept(b: &mut (impl Backend + ?Sized), cmds: &[(Cmd<()>, Vec<u8>)]) {
    for (c, data) in cmds {
        b.execute(c.map(|()| Data::Bytes(data)));
    }
}

/// The registers of [`kept_draw`] as one item's payload.
fn item_registers(regs: &[u32; 12]) -> Vec<u8> {
    regs.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// A record's commands are carried out as they come and kept; an item carries them out again with
/// its own registers: the backend gets exactly what the draw sent with those registers, and nothing
/// for an item whose registers are not the record's size or whose record was dropped.
#[test]
fn an_item_carries_out_its_records_commands_with_its_own_registers() {
    let first: [u32; 12] = std::array::from_fn(|i| i as u32);
    let second: [u32; 12] = std::array::from_fn(|i| 100 + i as u32);
    let mut plain = Log::default();
    let plain_lines = plain.lines();
    run_kept(&mut plain, &kept_draw(&first));
    run_kept(&mut plain, &kept_draw(&second));

    let log = Log::default();
    let lines = log.lines();
    let mut kept = Kept::new(log);
    kept.execute(Cmd::RecordNext {
        device: 1,
        id: 7,
        forget: 0,
        check: 0,
    });
    run_kept(&mut kept, &kept_draw(&first));
    let regs = item_registers(&second);
    kept.execute(Cmd::DrawItem {
        device: 1,
        id: 7,
        check: 0,
        constants: Data::Bytes(&regs),
    });
    assert_eq!(*lines.lock().unwrap(), *plain_lines.lock().unwrap());
    assert_eq!(kept.counts().records, 1);
    assert_eq!(kept.counts().items, 1);

    let before = lines.lock().unwrap().len();
    kept.execute(Cmd::DrawItem {
        device: 1,
        id: 7,
        check: 0,
        constants: Data::Bytes(&regs[..16]),
    });
    let ids = 7u32.to_le_bytes();
    kept.execute(Cmd::ForgetRecords {
        device: 1,
        ids: Data::Bytes(&ids),
    });
    kept.execute(Cmd::DrawItem {
        device: 1,
        id: 7,
        check: 0,
        constants: Data::Bytes(&regs),
    });
    assert_eq!(lines.lock().unwrap().len(), before, "nothing carried out");
    assert_eq!(kept.counts().mismatched, 1);
    assert_eq!(kept.counts().missing, 1);
}

/// A check run's two forms of one draw: the old form (`Form::Old`) carries out the draw's own
/// commands and leaves the item; the items (`Form::Items`) draw the item from the record kept
/// before and only keep the old form (as the game side renews its record); both give the digests a
/// run without items gives, here where the item and the old form agree.
#[test]
fn a_check_run_replays_as_its_old_form_or_as_its_items() {
    let first: [u32; 12] = std::array::from_fn(|i| i as u32);
    let second: [u32; 12] = std::array::from_fn(|i| 100 + i as u32);
    let regs = item_registers(&second);
    let run = |b: &mut dyn Backend| {
        b.execute(Cmd::RecordNext {
            device: 1,
            id: 1,
            forget: 0,
            check: 0,
        });
        run_kept(b, &kept_draw(&first));
        b.execute(Cmd::DrawItem {
            device: 1,
            id: 1,
            check: 1,
            constants: Data::Bytes(&regs),
        });
        b.execute(Cmd::RecordNext {
            device: 1,
            id: 2,
            forget: 1,
            check: 1,
        });
        run_kept(b, &kept_draw(&second));
    };
    let (mut plain, plain_digests) = StateDigest::new();
    run_kept(&mut plain, &kept_draw(&first));
    run_kept(&mut plain, &kept_draw(&second));
    for form in [kept::Form::Old, kept::Form::Items] {
        let (digest, digests) = StateDigest::new();
        let mut b = Kept::with_form(digest, form);
        run(&mut b);
        assert_eq!(b.counts().items, u64::from(form == kept::Form::Items));
        assert_eq!(b.counts().other_form, 1, "{form:?}");
        assert_eq!(b.records(), 1, "the old form renewed the record ({form:?})");
        assert_eq!(
            *digests.lock().unwrap(),
            *plain_digests.lock().unwrap(),
            "{form:?}"
        );
    }
    // An item that differs from its old form gives another digest in the items' replay only.
    let (digest, digests) = StateDigest::new();
    let mut b = Kept::with_form(digest, kept::Form::Items);
    b.execute(Cmd::RecordNext {
        device: 1,
        id: 1,
        forget: 0,
        check: 0,
    });
    run_kept(&mut b, &kept_draw(&first));
    let wrong = item_registers(&first);
    b.execute(Cmd::DrawItem {
        device: 1,
        id: 1,
        check: 1,
        constants: Data::Bytes(&wrong),
    });
    let d = digests.lock().unwrap();
    let p = plain_digests.lock().unwrap();
    assert_eq!(d[0], p[0]);
    assert_ne!(d[1], p[1], "the item's registers are the first draw's");
}

/// A record is given up at a `Present` before its draw: an item of it draws nothing.
#[test]
fn a_record_without_its_draw_is_given_up() {
    let mut kept = Kept::new(Log::default());
    kept.execute(Cmd::RecordNext {
        device: 1,
        id: 3,
        forget: 0,
        check: 0,
    });
    let cmds = kept_draw(&[0; 12]);
    run_kept(&mut kept, &cmds[..2]);
    kept.execute(Cmd::Present {
        device: 1,
        source: None,
        dest: None,
        window: 0,
        dirty: false,
        model: 0,
        frame: 1,
    });
    kept.execute(Cmd::DrawItem {
        device: 1,
        id: 3,
        check: 0,
        constants: Data::Bytes(&[0; 48]),
    });
    assert_eq!(kept.counts().cut, 1);
    assert_eq!(kept.counts().missing, 1);
    assert_eq!(kept.records(), 0);
}

/// The shape of `cmds` as the platform's side and [`Kept`] hash it ([`kept::Commands::shape`]).
fn shape_of(cmds: &[(Cmd<()>, Vec<u8>)]) -> u64 {
    let mut c = kept::Commands::default();
    for (cmd, data) in cmds {
        c.keep(cmd.map(|()| Data::Bytes(data)));
    }
    c.shape()
}

fn record_next(id: u32, forget: u32, check: u32) -> Cmd<Data<'static>> {
    Cmd::RecordNext {
        device: 1,
        id,
        forget,
        check,
    }
}

/// A shape is every command of the run with its payload, but the constants' registers (review of #1108): two
/// runs that differ in any command (the first, a middle one, a payload that is not registers) differ in shape;
/// two that differ only in their registers do not.
#[test]
fn a_runs_shape_is_every_command_but_the_registers() {
    let first: [u32; 12] = std::array::from_fn(|i| i as u32);
    let second: [u32; 12] = std::array::from_fn(|i| 100 + i as u32);
    let base = kept_draw(&first);
    assert_eq!(shape_of(&base), shape_of(&kept_draw(&second)));
    let mut other_source = base.clone();
    other_source[0].0 = Cmd::SetStreamSource {
        device: 1,
        stream: 0,
        buffer: 0x50,
        offset: 48,
        stride: 32,
    };
    assert_ne!(
        shape_of(&base),
        shape_of(&other_source),
        "the first command"
    );
    let mut other_indices = base.clone();
    other_indices[1].0 = Cmd::SetIndices {
        device: 1,
        buffer: 0x61,
    };
    assert_ne!(
        shape_of(&base),
        shape_of(&other_indices),
        "a middle command"
    );
    let transform = |m: u32| {
        let mut run = base.clone();
        run.insert(
            0,
            (
                Cmd::SetTransform {
                    device: 1,
                    state: 256,
                    matrix: (),
                },
                [m; 16].iter().flat_map(|w| w.to_le_bytes()).collect(),
            ),
        );
        run
    };
    assert_ne!(
        shape_of(&transform(1)),
        shape_of(&transform(2)),
        "a payload that is not registers"
    );
}

/// The records are the device's: a device made or reset drops them all, so an item after it draws nothing.
#[test]
fn a_reset_drops_the_records() {
    for reset in [
        Cmd::Reset {
            device: 1,
            window: 0,
            present: Data::Bytes(&[]),
            back_buffers: Data::Bytes(&[]),
            depth_stencil: 0,
        },
        Cmd::CreateDevice {
            device: 1,
            focus: 0,
            window: 0,
            behavior: 0x40,
            present: Data::Bytes(&[]),
            back_buffers: Data::Bytes(&[]),
            depth_stencil: 0,
        },
    ] {
        let log = Log::default();
        let lines = log.lines();
        let mut kept = Kept::new(log);
        kept.execute(record_next(7, 0, 0));
        run_kept(&mut kept, &kept_draw(&[0; 12]));
        assert_eq!(kept.records(), 1);
        kept.execute(reset);
        let before = lines.lock().unwrap().len();
        kept.execute(Cmd::DrawItem {
            device: 1,
            id: 7,
            check: 0,
            constants: Data::Bytes(&[0; 48]),
        });
        assert_eq!(lines.lock().unwrap().len(), before, "{}", reset.name());
        assert_eq!(
            (kept.records(), kept.counts().missing),
            (0, 1),
            "{}",
            reset.name()
        );
    }
}

/// A record keeps at most [`kept::MOST_COMMANDS`] commands, its draw included; one more and it is cut (the
/// game's side never holds a longer run).
#[test]
fn a_record_longer_than_the_most_commands_is_cut() {
    let constant = (
        Cmd::SetConstants {
            device: 1,
            kind: 0,
            start: 0,
            count: 1,
            data: (),
        },
        vec![0u8; 16],
    );
    let draw = kept_draw(&[0; 12]).pop().expect("the draw");
    for (constants, kept_it) in [
        (kept::MOST_COMMANDS - 1, true),
        (kept::MOST_COMMANDS, false),
    ] {
        let mut kept = Kept::new(Log::default());
        kept.execute(record_next(4, 0, 0));
        let mut run = vec![constant.clone(); constants];
        run.push(draw.clone());
        run_kept(&mut kept, &run);
        assert_eq!(
            (kept.records(), kept.counts().cut),
            (usize::from(kept_it), u64::from(!kept_it)),
            "{constants} constant sets"
        );
    }
}

/// A sync command inside a check record carried out as its items still reaches the backend (it answers);
/// the record's other commands do not (the item draws them).
#[test]
fn a_check_records_sync_command_is_carried_out_as_its_items() {
    let log = Log::default();
    let lines = log.lines();
    let mut kept = Kept::with_form(log, kept::Form::Items);
    kept.execute(record_next(2, 0, 1));
    let cmds = kept_draw(&[0; 12]);
    run_kept(&mut kept, &cmds[..1]);
    kept.execute(Cmd::QueryGetData {
        query: 0x70,
        flags: 1,
    });
    run_kept(&mut kept, &cmds[1..]);
    let lines = lines.lock().unwrap();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("QueryGetData"), "{lines:?}");
    assert_eq!(kept.records(), 1);
}

/// An item, the records' dropping or a reset in the middle of a record cuts it: its draw that comes later is no
/// record's.
#[test]
fn an_item_in_the_middle_of_a_record_cuts_it() {
    let mut kept = Kept::new(Log::default());
    kept.execute(record_next(5, 0, 0));
    let cmds = kept_draw(&[0; 12]);
    run_kept(&mut kept, &cmds[..2]);
    kept.execute(Cmd::DrawItem {
        device: 1,
        id: 9,
        check: 0,
        constants: Data::Bytes(&[]),
    });
    run_kept(&mut kept, &cmds[2..]);
    assert_eq!((kept.records(), kept.counts().cut), (0, 1));
}

/// The game's own [`Kept`] (`reporting`) gives its counts every 1,000 frames, and soon after an item found no
/// record (at most every 60 frames while that goes on); other ones say nothing.
#[test]
fn the_games_kept_records_report_their_counts() {
    let mut quiet = Kept::new(Log::default());
    assert!((0..1000).all(|_| quiet.presented().is_none()));
    let mut kept = Kept::new(Log::default()).reporting();
    assert!((0..999).all(|_| kept.presented().is_none()));
    let line = kept.presented().expect("the 1000th frame");
    assert!(
        line.starts_with("NV_DRAW_ITEMS backend: 0 records kept"),
        "{line}"
    );
    let missing = |k: &mut Kept<Log>| {
        k.execute(Cmd::DrawItem {
            device: 1,
            id: 3,
            check: 0,
            constants: Data::Bytes(&[]),
        });
    };
    missing(&mut kept);
    let line = kept.presented().expect("an item of no record");
    assert!(
        line.contains("1 items of no record, 0 with the wrong registers, 0 records cut"),
        "{line}"
    );
    missing(&mut kept);
    assert!(
        (0..59).all(|_| kept.presented().is_none()),
        "quiet for 60 frames"
    );
    assert!(kept
        .presented()
        .expect("then again")
        .contains("2 items of no record"));
}
