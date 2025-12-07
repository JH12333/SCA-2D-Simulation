# SCA-2D-Simulation

## Project Overview
This project implements an interactive **2D Space Colonization Algorithm (SCA)** inspired by *Modeling Trees with a Space Colonization Algorithm* (Runions, Lane & Prusinkiewicz, 2007). The idea is to let branches compete for **attractor points** in space rather than using recursive splitting.

The code base is split into two crates:

- **`sim-core`**: a library crate that contains the simulation data structures and algorithms:
  - `Tree` and `TreeNode` for the growing tree.
  - `AttractorSet` for attractor points in space.
  - `InfluenceBuffer` as a per-node accumulator.
  - High-level phases: `attraction_phase`, `growth_phase`, and `kill_phase`.
  - A configurable `Config` struct controlling radii, k-th-nearest indices, step length, tropism, and spawn tools.
- **`sim-view`**: an eframe/egui application that visualizes the tree and attractors and exposes runtime controls:
  - A central canvas that displays the tree and attractor cloud.
  - Side/top panels for simulation parameters, run control, and status.
  - Basic interaction tools for spawning roots and attractors.

---

## What Works

### Core SCA Logic

The following pieces of the Space Colonization Algorithm are implemented and working:

- **Attraction phase**  
  - Each alive attractor queries its *k*-th nearest node using `Tree::find_kth_nearest_nodes`.
  - If the squared distance is within `influence_radius²`, a normalized direction from the node to the attractor is accumulated into an `InfluenceBuffer`.
  - Attractors record which node (if any) currently “owns” them.

- **Growth phase**  
  - For each node that was influenced in the previous step, the average direction is computed.
  - A configurable **tropism** vector (e.g., gravity-like bias) is added, and the result is normalized.
  - A new child node is proposed at `old_pos + dir * step_len`.
  - If there is already a child very close to the proposed position, the candidate is skipped to avoid local clutter (via `Tree::has_child_near`).
  - Otherwise, a new node is added to the tree; newly created node ids are tracked for visualization.

- **Kill phase**  
  - Each alive attractor again queries its *k*-th nearest node.
  - If the distance is within `kill_radius`, the attractor is marked dead and stops participating in future steps.

The algorithm is therefore structured as a clean **Attract → Grow → Kill** loop.

### Viewer and Interaction

The `sim-view` crate provides a functional visualization and control surface:

- **Run control**
  - Start / pause continuous simulation.
  - Step-by-step advancement using a single button.
  - Reset: rebuilds a fresh tree and a new attractor cloud.
  - Clear: removes all nodes and attractors, leaving a blank canvas.

- **Configuration panel**
  - `attract_from_kn` and `kill_from_kn` (k-nearest indices).
  - `influence_radius` and `kill_radius`.
  - `step_len` for growth.
  - `tropism.x` and `tropism.y` for directional bias.
  - Spawn controls:
    - `spawn_attractors` (number of attractors to create in one click).
    - `spawn_rect_half_extents` for rectangular spawn areas.
    - `spawn_oval_radii` for oval spawn areas.
  - *Note:* Larger values of `attract_from_kn` / `kill_from_kn` increase the cost of each nearest-neighbor query, so small values are recommended for large trees.

- **Spawn tools**
  - **Root tool**: add new root nodes at the clicked position.
  - **Rect attractors tool**: spawn attractors in a rectangle centered at the click point.
  - **Oval attractors tool**: spawn attractors in an oval around the click point.
  - A small overlay indicates the current spawn area (rectangle or oval) under the mouse cursor.

- **Camera and visualization**
  - Pan by dragging on the central canvas.
  - Zoom with the scroll wheel, centered around the mouse position.
  - Tree edges are drawn as line segments between parent and child nodes.
  - Nodes are drawn as filled circles; nodes added in the most recent step are highlighted.
  - Alive attractors are drawn as small red dots.
  - A status bar shows:
    - Number of nodes.
    - Number of alive attractors.
    - Target step interval and measured last step duration.

For moderate tree sizes (up to roughly ten thousand nodes), the viewer runs smoothly and allows interactive exploration of SCA behavior.

---

## What Does Not Work as Intended / Known Failure Modes

Even though the basic loop runs, there are several edge cases where the behavior is not fully satisfactory.

### 1. Attractors that never get killed

There are scenarios where some attractors keep pulling the tree in a similar direction indefinitely but are never removed by the kill phase:

- **Overlapping children mitigated but root cause remains**  
  - The function `Tree::has_child_near` (with a corresponding test `has_child_near_detects_close_child`) was introduced to skip creating new children extremely close to existing ones.
  - This alleviates obvious visual artifacts such as multiple nodes stacking on top of each other when the growth direction keeps repeating.
  - However, the underlying logical issue — attractors that never enter the kill radius — still exists.

These non-converging attractors fall into two broad categories:

#### a. Parameter regime issues

- If the growth step length `step_len` is too large relative to the kill radius `kill_radius`, branches can “jump over” the region where attractors would be killed.
- In such a configuration, an attractor may keep influencing the same node or its descendants, but the new nodes never actually pass through the kill zone.
- This is essentially a parameter mismatch: with reasonable ratios between step length and kill radius, the problem can often be avoided.

#### b. Symmetric cancellation in the algorithm

- When two attractors are placed symmetrically around a node, their attraction vectors can cancel out in the average, producing a growth direction along the midline.
- In that case, the node does not move meaningfully closer to either attractor, and both attractors may remain outside `kill_radius` forever.
- **tropism** is a way to break such symmetries: adding a small directional bias can help the tree “choose” a direction and eventually enter the kill zone.
- This is treated as a modeling limitation rather than a bug; no additional logic is implemented to detect or resolve perfectly symmetric configurations.

These behaviors are important to note because they highlight regimes where the current implementation may not converge or may require careful parameter tuning.

---

## What Is Not Implemented / Known Limitations

### 1. Pipe-model radius update for branches

The original design planned to implement a **pipe-model** for branch thickness, where node radii are updated based on downstream subtree size (e.g., cumulative flow or number of terminal tips). This would produce more realistic tapering of branches.

In the current implementation:

- Each `TreeNode` carries a `radius` field, but this value is not dynamically updated by a pipe model.
- Newly created nodes simply inherit the radius of their parent.

The data model in `sim-core` is already structured in a way that would make a pipe-model feasible (the tree is stored as an array with explicit parent/child links). The main practical obstacle is the viewer:

- egui’s built-in line stroke (`egui::Stroke` and `egui::Shape::line_segment`) uses a **constant thickness** along the entire segment.
- A true pipe-model visualization requires segments that are thicker at the base and thinner at the tip.
- Achieving this effect would require custom geometry (e.g., building tapered quads or triangle meshes) instead of relying on the built-in line primitive.

As a result, the pipe-model is **not implemented** in this iteration, although the code is structured to accommodate it later.

### 2. Performance at high node counts

For very large trees (on the order of **> 10,000 nodes**), the viewer can start to lose smoothness:

- Rendering many lines and circles every frame becomes relatively expensive in egui.
- The current implementation uses **simple O(N × M)** style loops for attraction and k-nearest queries over all nodes, without spatial acceleration structures.

The simulation logic in `sim-core` is intentionally written in a data-parallel style:

- The attraction phase operates per attractor.
- The growth phase operates per influenced node.
- The kill phase again operates per attractor.

This makes the algorithm a good candidate for **parallelization** (for example, using Rayon). However, the current version remains **single-threaded**, and no parallel execution is implemented yet.

A reasonable next step would be:

- Introduce parallel iterators for phases that are embarrassingly parallel.
- Optionally add spatial indexing (e.g., grid or k-d tree) for k-nearest queries.
- Decouple simulation stepping from frame rendering so that heavy steps can run off the UI thread.

### 3. Direction constraints and visualization features

Several planned features are not implemented:

- **No hard-coded branching angle constraints**  
  - Growth directions are determined purely by the averaged influence vector plus optional tropism.
  - There is no explicit constraint on the angle between a new branch and its parent (e.g., no maximum deviation or hard-coded branching pattern).  
  - More biologically-inspired or stylized branching patterns would require additional angle-based logic.

- **No per-branch level computation for display**  
  - The tree does not currently track explicit depth/level for each branch.
  - All nodes are rendered with the same visual treatment (apart from simple radius scaling and “new node” highlighting).
  - Depth-based styling (e.g., color or thickness by level) is therefore not available.

- **No user control over color palette**  
  - Colors for nodes, new nodes, edges, and attractors are fixed in code.
  - The viewer does not expose any settings for users to customize these colors.

- **Planned local vs. global k-nearest evaluation not implemented**  
  - Both the growth and kill phases currently query the k-th nearest node **over the entire tree**, rather than within a local neighborhood.  
  - For the attraction phase, a more meaningful design would be **local** evaluation: for each attractor, consider only tree nodes within its influence radius, sort them by distance, and treat the farthest node in that local set as the fallback when `k` exceeds the local count. In that scheme, only nodes that actually “compete” for the attractor (i.e., lie within its influence region) can win and grow. The current **global** evaluation breaks this semantic: a node that is not even within the attractor’s influence radius can still be chosen as the k-th neighbor and receive growth credit, even though it does not satisfy the geometric growth condition.  
  - For the kill phase, a local k-NN variant would not change behavior in a meaningful way: if we restrict to nodes within the kill radius and fall back to the farthest local node, the attractor is still removed whenever any node is in range. This is effectively equivalent to simply killing based on the nearest node, so the current global k-based lookup is acceptable here.  
  - *Note:* the `k` parameter itself is not part of the original SCA algorithm; it is introduced here mainly to add visual variety and improve visual interest (even if it slightly violates the natural “strongest wins” rule). Conceptually, it is similar to using a k-nearest-neighbor / Voronoi-style construction on the tree nodes.

---

## Lessons Learned

Several lessons emerged during the development of this project:

1. **Core vs. UI separation pays off**  
   Splitting the project into a `sim-core` library and a `sim-view` application made it much easier to:
   - Write unit tests for the SCA phases and data structures.
   - Reason about correctness and performance without being entangled with GUI concerns.
   - Consider adding parallelism later without changing the viewer.

2. **Index-based trees are simple and effective**  
   Representing the tree as a flat `Vec<TreeNode>` with `NodeId = usize` indices proved to be:
   - Easy to traverse (parent and children links are explicit).
   - Friendly to borrowing rules.
   - Reasonably cache-friendly for the inner loops.

3. **Scalability and convergence need to be considered early**  
   The basic SCA algorithm works well for small to medium trees, but:
   - The cost of naive nearest-neighbor queries becomes significant for large node counts.
   - Without parallelization and acceleration structures, stepping can become slow as the tree grows.
   - Certain parameter regimes (e.g., step length larger than kill radius) and symmetric attractor configurations can lead to non-converging attractors that are never killed.
   The implementation uses tropism and `has_child_near` to mitigate some of these issues, but they also highlight the importance of careful parameter choices and algorithm design.

4. **Early testing**  
   - Running `cargo clippy` regularly helps to identify small logical errors and style issues early on, thus reducing subsequent debugging time.
   - Unit tests should be written as early as possible once a component becomes stable.

---

## AI Usage

- **egui panels and view transforms**  
  The layout of egui panels and the design of world-to-screen / screen-to-world coordinate transforms were developed with the help of ChatGPT, and then integrated and adjusted manually.

- **Documentation comments (Rustdoc)**  
  Many Rustdoc-style comments (`///` and `//!`) were initially drafted by ChatGPT.  
  These comments were then manually reviewed, edited, and corrected to match the actual implementation.

- **Unit tests**  
  After the main functionality was implemented and stabilized, unit tests for core components (such as `Tree`, `InfluenceBuffer`, and the SCA phases) were added based on ChatGPT-generated test templates.  
  The final tests were adapted to the project’s concrete types and behavior and verified against the running code.

---

## How to Build and Run

### Prerequisites

- Rust (stable toolchain, installed via `rustup`).
- The project uses `eframe`, `egui`, `glam`, and `rand` as dependencies; `cargo` will fetch them automatically.

### Commands

From the repository root:

```bash
# Run all tests (most tests target the core; a smaller subset covers the viewer)
cargo test

# Run clippy
cargo clippy

# Run the viewer application (much faster in release mode)
cargo run --release