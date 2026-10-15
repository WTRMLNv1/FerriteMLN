# FerriteMLN

Low-end-friendly voxel-engine foundation in Rust and wgpu. `cargo run -p game` starts the Phase 0 renderer; core generation and greedy meshing are testable without a GPU.

## Layout

- `voxel-core`: block data, palette chunk storage, coordinates, LOD ring index.
- `voxel-gen`: deterministic, scale-aware terrain generation.
- `voxel-mesh`: binary greedy surface extraction into direction buckets.
- `voxel-jobs`: bounded worker-pool and stale-job cancellation protocol.
- `voxel-render`: adapter capability probe and reverse-Z renderer.
- `game`: winit integration and frame metrics.
