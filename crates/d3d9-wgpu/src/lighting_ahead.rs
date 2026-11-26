//! The new lighting's objects made ahead on a thread of the device's own (port-added). While the
//! player can switch the new lighting while playing (`crate::scene_light::allow_switch`), what its
//! parts draw with is made off the frame: the ambient occlusion's pipelines and the sun shadows'
//! thin casters' pipeline at the device's start, the occlusion's pipeline for each screen-sized
//! target the game makes, the counter's and the note's drawing once the window's frame is known;
//! the first time a part is switched on, the sun shadows' maps, the occlusion's half-resolution
//! textures and the multisampled depth surfaces made readable. The device asks for a [`Job`] and
//! takes what is made at its next `Present` ([`Worker::take`]); a part runs from the frame after
//! what it needs is there, so no frame waits for the driver at a switch. Pipelines are made with
//! the device's pipeline cache.

use crate::{frame_rate, scene_light, sun_shadow};
use std::collections::HashSet;
use std::sync::mpsc;

/// What the thread makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Job {
    /// The sun shadows' thin casters' pipeline ([`sun_shadow::thin_pipeline`]).
    Thin,
    /// What the ambient occlusion needs ([`scene_light::Need`]).
    Ao(scene_light::Need),
    /// The sun shadows' maps ([`sun_shadow::Gpu::new`]), with the characters' layers or not.
    SunMaps(bool),
    /// The counter's and the notes' drawing for the window format, the notes for a glyph height
    /// ([`frame_rate::Overlay`]).
    Overlay(wgpu::TextureFormat, u32),
    /// A multisampled depth surface made again, readable: the surface's texture id, its size,
    /// format and samples.
    Depth(u64, u32, u32, wgpu::TextureFormat, u32),
}

/// What a [`Job`] made.
pub(crate) enum Made {
    Thin(wgpu::RenderPipeline),
    Ao(scene_light::Ahead),
    /// The maps and how long they took to make.
    SunMaps(Box<sun_shadow::Gpu>, std::time::Duration),
    Overlay(Box<frame_rate::Overlay>),
    Depth(wgpu::Texture, wgpu::TextureView),
    /// wgpu refused it (said in the log once by the device).
    Refused(String),
}

/// The thread and what it was asked for and has not answered yet.
pub(crate) struct Worker {
    jobs: mpsc::Sender<Job>,
    made: mpsc::Receiver<(Job, Made)>,
    asked: HashSet<Job>,
}

/// The notes the window shows after a switch (all four states').
fn notes() -> [&'static str; 4] {
    scene_light::Parts::EVERY.map(scene_light::Parts::note)
}

/// Makes `job` on the calling thread (the worker's), with the driver's `cache`.
fn make(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: Option<&wgpu::PipelineCache>,
    job: Job,
) -> Made {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let made = match job {
        Job::Thin => Made::Thin(sun_shadow::thin_pipeline(device, cache)),
        Job::Ao(need) => Made::Ao(scene_light::make(device, cache, need)),
        Job::SunMaps(characters) => {
            let started = std::time::Instant::now();
            let gpu = sun_shadow::Gpu::new(device, characters);
            Made::SunMaps(Box::new(gpu), started.elapsed())
        }
        Job::Overlay(format, glyph) => {
            let mut o = frame_rate::Overlay::new(device, format, cache);
            o.prepare_notes(device, queue, glyph, &notes());
            Made::Overlay(Box::new(o))
        }
        Job::Depth(_, width, height, format, samples) => {
            crate::made_here::texture();
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: width.max(1),
                    height: height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                ..Default::default()
            });
            Made::Depth(texture, view)
        }
    };
    match crate::device::block_on(scope.pop()) {
        None => made,
        Some(e) => Made::Refused(e.to_string()),
    }
}

impl Worker {
    /// The thread, for `device` and `queue` (none when it cannot be started: the device then makes
    /// what it needs itself, as without the switch).
    pub(crate) fn start(
        device: wgpu::Device,
        queue: wgpu::Queue,
        cache: Option<wgpu::PipelineCache>,
    ) -> Option<Worker> {
        let (jobs, asked) = mpsc::channel::<Job>();
        let (tx, made) = mpsc::channel();
        std::thread::Builder::new()
            .name("nv-wgpu-lighting".into())
            .spawn(move || {
                // Ends with the device (its sender dropped).
                for job in asked {
                    let m = make(&device, &queue, cache.as_ref(), job);
                    if tx.send((job, m)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Worker {
            jobs,
            made,
            asked: HashSet::new(),
        })
    }

    /// Asks for `job` unless it is on its way.
    pub(crate) fn ask(&mut self, job: Job) {
        if self.asked.insert(job) && self.jobs.send(job).is_err() {
            self.asked.remove(&job);
        }
    }

    /// Whether a job `of` this kind is on its way.
    pub(crate) fn waits_for(&self, of: impl Fn(&Job) -> bool) -> bool {
        self.asked.iter().any(of)
    }

    /// What is made by now (never waited for).
    pub(crate) fn take(&mut self) -> Vec<(Job, Made)> {
        let done: Vec<(Job, Made)> = self.made.try_iter().collect();
        for (job, _) in &done {
            self.asked.remove(job);
        }
        done
    }

    /// Tests: waits until everything asked for is made.
    pub(crate) fn wait(&mut self) -> Vec<(Job, Made)> {
        let mut done = Vec::new();
        while !self.asked.is_empty() {
            let Ok((job, m)) = self.made.recv() else {
                break;
            };
            self.asked.remove(&job);
            done.push((job, m));
        }
        done
    }
}
