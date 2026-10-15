use glam::Vec3;
use std::{collections::HashSet, time::Instant};
use voxel_core::{AIR, ChunkData, ChunkPos};
use voxel_gen::Terrain;
use voxel_mesh::mesh;
use voxel_render::{Renderer, VoxelVertex};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(48.0, 132.0, 72.0),
            yaw: -2.16,
            pitch: -0.38,
        }
    }
}
impl Camera {
    fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }
}
#[derive(Default)]
struct App {
    window: Option<&'static Window>,
    renderer: Option<Renderer<'static>>,
    last: Option<Instant>,
    vertices: usize,
    camera: Camera,
    keys: HashSet<KeyCode>,
}
impl App {
    fn update_camera(&mut self, seconds: f32) {
        let speed = if self.keys.contains(&KeyCode::ControlLeft) {
            35.0
        } else {
            12.0
        };
        let forward = self.camera.forward();
        let flat = Vec3::new(forward.x, 0.0, forward.z).normalize();
        let right = flat.cross(Vec3::Y).normalize();
        if self.keys.contains(&KeyCode::KeyW) {
            self.camera.position += flat * speed * seconds;
        }
        if self.keys.contains(&KeyCode::KeyS) {
            self.camera.position -= flat * speed * seconds;
        }
        if self.keys.contains(&KeyCode::KeyA) {
            self.camera.position -= right * speed * seconds;
        }
        if self.keys.contains(&KeyCode::KeyD) {
            self.camera.position += right * speed * seconds;
        }
        if self.keys.contains(&KeyCode::Space) {
            self.camera.position.y += speed * seconds;
        }
        if self.keys.contains(&KeyCode::ShiftLeft) {
            self.camera.position.y -= speed * seconds;
        }
        let look = 1.7 * seconds;
        if self.keys.contains(&KeyCode::ArrowLeft) {
            self.camera.yaw -= look;
        }
        if self.keys.contains(&KeyCode::ArrowRight) {
            self.camera.yaw += look;
        }
        if self.keys.contains(&KeyCode::ArrowUp) {
            self.camera.pitch = (self.camera.pitch + look).min(1.5);
        }
        if self.keys.contains(&KeyCode::ArrowDown) {
            self.camera.pitch = (self.camera.pitch - look).max(-1.5);
        }
        if let Some(r) = &self.renderer {
            r.set_camera(
                self.camera.position.to_array(),
                self.camera.forward().to_array(),
            );
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, e: &ActiveEventLoop) {
        if self.window.is_none() {
            let w = Box::leak(Box::new(
                e.create_window(Window::default_attributes().with_title("FerriteMLN"))
                    .unwrap(),
            ));
            let vertices = terrain_patch();
            self.vertices = vertices.len();
            self.renderer =
                Some(pollster::block_on(Renderer::new(w, &vertices)).expect("wgpu startup"));
            self.window = Some(w)
        }
    }
    fn window_event(&mut self, e: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => e.exit(),
            WindowEvent::Resized(s) => self.renderer.as_mut().unwrap().resize(s.width, s.height),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(k) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.keys.insert(k);
                        }
                        ElementState::Released => {
                            self.keys.remove(&k);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = self
                    .last
                    .map(|last| now.duration_since(last).as_secs_f32().min(0.1))
                    .unwrap_or(0.0);
                self.update_camera(dt);
                if let Some(w) = self.window {
                    w.set_title(&format!(
                        "FerriteMLN — {:.1} ms — {} vertices — WASD / arrows",
                        dt * 1000.0,
                        self.vertices
                    ));
                }
                self.last = Some(now);
                match self.renderer.as_mut().unwrap().render() {
                    Ok(()) => (),
                    Err(wgpu::SurfaceError::Lost) => {
                        let s = self.window.unwrap().inner_size();
                        self.renderer.as_mut().unwrap().resize(s.width, s.height)
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => e.exit(),
                    Err(_) => (),
                }
            }
            _ => (),
        }
    }
    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = self.window {
            w.request_redraw()
        }
    }
}
fn terrain_patch() -> Vec<VoxelVertex> {
    let terrain = Terrain::new(42);
    let mut out = Vec::new();
    for cz in -2..=2 {
        for cx in -2..=2 {
            let pos = ChunkPos {
                x: cx,
                y: 3,
                z: cz,
                lod: 0,
            };
            let mut data = ChunkData::uniform(AIR);
            terrain.fill(pos, &mut data);
            let m = mesh(&data);
            for (face, bucket) in m.buckets.iter().enumerate() {
                for quad in bucket {
                    let p = quad.packed;
                    let x = (p & 31) as f32 + cx as f32 * 32.0;
                    let y = ((p >> 5) & 31) as f32 + 96.0;
                    let z = ((p >> 10) & 31) as f32 + cz as f32 * 32.0;
                    emit_face(&mut out, [x, y, z], face, colour((p >> 18) as u16));
                }
            }
        }
    }
    out
}
fn colour(id: u16) -> [f32; 3] {
    match id {
        1 => [0.42, 0.42, 0.46],
        2 => [0.35, 0.22, 0.12],
        3 => [0.25, 0.52, 0.18],
        4 => [0.10, 0.35, 0.72],
        5 => [0.76, 0.69, 0.42],
        6 => [0.90, 0.94, 0.98],
        _ => [1.0, 0.0, 1.0],
    }
}
fn emit_face(o: &mut Vec<VoxelVertex>, p: [f32; 3], f: usize, col: [f32; 3]) {
    const C: [[[f32; 3]; 4]; 6] = [
        [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]],
        [[0., 0., 1.], [0., 1., 1.], [0., 1., 0.], [0., 0., 0.]],
        [[0., 1., 0.], [0., 1., 1.], [1., 1., 1.], [1., 1., 0.]],
        [[0., 0., 1.], [0., 0., 0.], [1., 0., 0.], [1., 0., 1.]],
        [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
        [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
    ];
    for i in [0, 1, 2, 0, 2, 3] {
        let a = C[f][i];
        o.push(VoxelVertex {
            position: [p[0] + a[0], p[1] + a[1], p[2] + a[2]],
            colour: col,
        });
    }
}
fn main() {
    tracing_subscriber::fmt::init();
    let mut app = App::default();
    EventLoop::new().unwrap().run_app(&mut app).unwrap();
}
