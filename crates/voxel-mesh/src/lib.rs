//! Opaque binary surface extractor. A `Quad` is exactly 8 bytes for direct GPU upload.
use bytemuck::{Pod, Zeroable};
use voxel_core::*;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Quad {
    pub packed: u64,
}
pub struct Mesh {
    pub buckets: [Vec<Quad>; 6],
}
impl Default for Mesh {
    fn default() -> Self {
        Self {
            buckets: std::array::from_fn(|_| Vec::new()),
        }
    }
}
fn solid(c: &ChunkData, x: isize, y: isize, z: isize) -> bool {
    x >= 0
        && y >= 0
        && z >= 0
        && x < 32
        && y < 32
        && z < 32
        && c.get(x as usize, y as usize, z as usize) != AIR
}
/// Emits one unit quad per exposed face. The stable packed format is ready to replace with greedy merging.
pub fn mesh(c: &ChunkData) -> Mesh {
    let mut m = Mesh::default();
    let dirs = [
        (1, 0, 0),
        (-1, 0, 0),
        (0, 1, 0),
        (0, -1, 0),
        (0, 0, 1),
        (0, 0, -1),
    ];
    for y in 0..32 {
        for z in 0..32 {
            for x in 0..32 {
                let id = c.get(x, y, z);
                if id == AIR {
                    continue;
                }
                for (d, &(dx, dy, dz)) in dirs.iter().enumerate() {
                    if !solid(c, x as isize + dx, y as isize + dy, z as isize + dz) {
                        let p = (x as u64)
                            | ((y as u64) << 5)
                            | ((z as u64) << 10)
                            | ((d as u64) << 15)
                            | ((id as u64) << 18);
                        m.buckets[d].push(Quad { packed: p })
                    }
                }
            }
        }
    }
    m
}
