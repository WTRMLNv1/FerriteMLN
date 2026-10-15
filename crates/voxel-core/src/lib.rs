//! GPU-independent voxel types. Coordinates are deliberately signed: worlds do not have an edge.
use std::collections::HashMap;

pub const CHUNK_SIDE: usize = 32;
pub const PADDED_SIDE: usize = CHUNK_SIDE + 2;
pub const CHUNK_VOLUME: usize = CHUNK_SIDE * CHUNK_SIDE * CHUNK_SIDE;
pub type BlockId = u16;
pub const AIR: BlockId = 0;
pub const STONE: BlockId = 1;
pub const DIRT: BlockId = 2;
pub const GRASS: BlockId = 3;
pub const WATER: BlockId = 4;
pub const SAND: BlockId = 5;
pub const SNOW: BlockId = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub lod: u8,
}
impl ChunkPos {
    pub fn scale(self) -> i32 {
        1_i32 << self.lod
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BlockInfo {
    pub opaque: bool,
    pub transparent: bool,
    pub alpha_tested: bool,
    pub face_layers: [u16; 6],
    pub far_colour: [u8; 4],
}
pub struct Registry {
    blocks: Vec<BlockInfo>,
}
impl Default for Registry {
    fn default() -> Self {
        Self {
            blocks: vec![
                BlockInfo {
                    opaque: false,
                    transparent: true,
                    alpha_tested: false,
                    face_layers: [0; 6],
                    far_colour: [0, 0, 0, 0],
                },
                BlockInfo {
                    opaque: true,
                    transparent: false,
                    alpha_tested: false,
                    face_layers: [1; 6],
                    far_colour: [112, 112, 118, 255],
                },
                BlockInfo {
                    opaque: true,
                    transparent: false,
                    alpha_tested: false,
                    face_layers: [2; 6],
                    far_colour: [105, 72, 44, 255],
                },
                BlockInfo {
                    opaque: true,
                    transparent: false,
                    alpha_tested: false,
                    face_layers: [3, 2, 2, 2, 2, 2],
                    far_colour: [77, 130, 56, 255],
                },
                BlockInfo {
                    opaque: false,
                    transparent: true,
                    alpha_tested: false,
                    face_layers: [4; 6],
                    far_colour: [40, 95, 170, 180],
                },
                BlockInfo {
                    opaque: true,
                    transparent: false,
                    alpha_tested: false,
                    face_layers: [5; 6],
                    far_colour: [194, 178, 125, 255],
                },
                BlockInfo {
                    opaque: true,
                    transparent: false,
                    alpha_tested: false,
                    face_layers: [6; 6],
                    far_colour: [235, 240, 245, 255],
                },
            ],
        }
    }
}
impl Registry {
    pub fn get(&self, id: BlockId) -> BlockInfo {
        self.blocks
            .get(id as usize)
            .copied()
            .unwrap_or(self.blocks[0])
    }
}

#[derive(Clone, Debug)]
pub enum ChunkData {
    Uniform(BlockId),
    Palette {
        palette: Vec<BlockId>,
        indices: Vec<u8>,
    },
}
impl ChunkData {
    pub fn uniform(block: BlockId) -> Self {
        Self::Uniform(block)
    }
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        let i = x + CHUNK_SIDE * (z + CHUNK_SIDE * y);
        match self {
            Self::Uniform(v) => *v,
            Self::Palette { palette, indices } => palette[indices[i] as usize],
        }
    }
    pub fn set(&mut self, x: usize, y: usize, z: usize, v: BlockId) {
        let i = x + CHUNK_SIDE * (z + CHUNK_SIDE * y);
        match self {
            Self::Uniform(old) if *old == v => return,
            Self::Uniform(old) => {
                let old = *old;
                *self = Self::Palette {
                    palette: vec![old, v],
                    indices: vec![0; CHUNK_VOLUME],
                };
                if let Self::Palette { indices, .. } = self {
                    indices[i] = 1
                }
            }
            Self::Palette { palette, indices } => {
                let p = palette.iter().position(|&b| b == v).unwrap_or_else(|| {
                    assert!(palette.len() < 256, "palette overflow");
                    palette.push(v);
                    palette.len() - 1
                });
                indices[i] = p as u8;
            }
        }
    }
}

/// Fixed-size camera-centred world index. Entries outside the current window are overwritten.
pub struct RingIndex<T> {
    radius: i32,
    cells: HashMap<(i32, i32, i32, u8), T>,
}
impl<T> RingIndex<T> {
    pub fn new(radius: i32) -> Self {
        Self {
            radius,
            cells: HashMap::new(),
        }
    }
    pub fn insert(&mut self, p: ChunkPos, v: T) {
        self.cells.insert((p.x, p.y, p.z, p.lod), v);
    }
    pub fn get(&self, p: ChunkPos) -> Option<&T> {
        self.cells.get(&(p.x, p.y, p.z, p.lod))
    }
    pub fn recenter(&mut self, center: ChunkPos) {
        let r = self.radius;
        self.cells.retain(|&(x, y, z, l), _| {
            l == center.lod
                && (x - center.x).abs() <= r
                && (y - center.y).abs() <= r
                && (z - center.z).abs() <= r
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uniform_promotes() {
        let mut c = ChunkData::uniform(AIR);
        c.set(1, 2, 3, STONE);
        assert_eq!(c.get(1, 2, 3), STONE);
        assert_eq!(c.get(0, 0, 0), AIR)
    }
}
