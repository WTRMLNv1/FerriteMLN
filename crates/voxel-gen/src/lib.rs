use fastnoise_lite::{FastNoiseLite, NoiseType};
use voxel_core::*;
pub const SEA_LEVEL: i32 = 96;
pub struct Terrain {
    height: FastNoiseLite,
    continent: FastNoiseLite,
    cave: FastNoiseLite,
}
impl Terrain {
    pub fn new(seed: i32) -> Self {
        fn n(seed: i32, frequency: f32) -> FastNoiseLite {
            let mut n = FastNoiseLite::with_seed(seed);
            n.set_noise_type(Some(NoiseType::OpenSimplex2));
            n.set_frequency(Some(frequency));
            n
        }
        Self {
            height: n(seed, 0.004),
            continent: n(seed + 1, 0.0012),
            cave: n(seed + 2, 0.045),
        }
    }
    pub fn height_at(&self, x: i32, z: i32, lod: u8) -> i32 {
        let s = 1_i32 << lod;
        let x = (x * s) as f32;
        let z = (z * s) as f32;
        let plains = self.height.get_noise_2d(x, z) * 18.;
        let continents = self.continent.get_noise_2d(x, z);
        SEA_LEVEL + (plains + continents.signum() * continents.abs().powf(1.6) * 85.) as i32
    }
    pub fn column_range(&self, cx: i32, cz: i32, lod: u8) -> (i32, i32) {
        let mut lo = i32::MAX;
        let mut hi = i32::MIN;
        for z in [0, 31] {
            for x in [0, 31] {
                let h = self.height_at(cx * 32 + x, cz * 32 + z, lod);
                lo = lo.min(h);
                hi = hi.max(h.max(SEA_LEVEL));
            }
        }
        (lo, hi)
    }
    pub fn fill(&self, pos: ChunkPos, out: &mut ChunkData) {
        let scale = pos.scale();
        for z in 0..32 {
            for x in 0..32 {
                let wx = pos.x * 32 + x as i32;
                let wz = pos.z * 32 + z as i32;
                let h = self.height_at(wx, wz, pos.lod);
                for y in 0..32 {
                    let wy = (pos.y * 32 + y as i32) * scale;
                    let id = if wy > h {
                        if wy <= SEA_LEVEL { WATER } else { AIR }
                    } else if pos.lod <= 1
                        && wy < h - 4
                        && self.cave.get_noise_3d(
                            (wx * scale) as f32,
                            wy as f32,
                            (wz * scale) as f32,
                        ) > 0.68
                    {
                        AIR
                    } else if wy == h {
                        if h < SEA_LEVEL + 2 {
                            SAND
                        } else if h > 170 {
                            SNOW
                        } else {
                            GRASS
                        }
                    } else if wy > h - 4 {
                        DIRT
                    } else {
                        STONE
                    };
                    out.set(x, y, z, id)
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic() {
        let t = Terrain::new(7);
        let p = ChunkPos {
            x: 3,
            y: 2,
            z: -9,
            lod: 0,
        };
        let mut a = ChunkData::uniform(AIR);
        let mut b = ChunkData::uniform(AIR);
        t.fill(p, &mut a);
        t.fill(p, &mut b);
        for y in 0..32 {
            assert_eq!(a.get(5, y, 9), b.get(5, y, 9))
        }
    }
}
