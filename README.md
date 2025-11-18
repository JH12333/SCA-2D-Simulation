# SCA-2D-Simulation

## Project Overview
This project implements an interactive **2D Space Colonization Algorithm (SCA)** inspired by *Modeling Trees with a Space Colonization Algorithm* (Runions, Lane & Prusinkiewicz, 2007). The idea is to let branches compete for **attractor points** in space rather than using recursive splitting.

Users operate on a fixed-size canvas with a **movable single seed** and **attractor points**. Attractors can be created once from preset shapes and also added dynamically with the mouse during runtime. The simulation advances in **two-second ticks**: on each tick it executes multiple evolution steps and then refreshes the display. Controls include start/stop/reset and real-time parameter tuning. Basic runtime metrics (node/edge counts, timing) are displayed, with lightweight logging.

---

## UI Components

### SCA Controls
- **Preset shapes** for attractor generation: rectangle / circle / annulus  
- **Attractor count (N)**
- **Influence radius** `d_i` (float)
- **Kill radius** `d_k` (float)
- **Growth step** `d_s` (float)
- **Tropism** `g` (vec(float, float))

### Simulation Controls
- **Start**
- **Stop**
- **Reset**

### Visualization
- **Canvas** for tree growth (updates every **two seconds**)
- **Logging** of aggregated stats (e.g., every 10 steps)

---

## SCA Workflow

> Each attractor affects its **nearest tree node** (within `d_i`).  
> A tree node may be influenced by **multiple attractors** in the same tick.

1. **Initialization**
   - Create the initial seed node (movable).
   - Distribute attractor points (from a chosen shape) or prepare to add them dynamically.

2. **Attraction Phase**
   - For each attractor:
     1) Find the nearest node.  
     2) If distance ≤ `d_i`, mark that node as attracted and accumulate the unit vector from node → attractor.

3. **Growth Phase**
   - For each attracted node:
     1) Compute growth direction `v_dir = normalize(normalize(sum(normalize(direction_vectors)))+g)`.  
     2) Create a **candidate child** at `new_pos = current_pos + d_s * v_dir`.  
   - **Sort candidates by parent node ID (ascending)** and then **submit all children at once**, connecting edges `(parent → child)`.  
     *(This keeps IDs stable.)*

4. **Update Phase**
   - After all children are submitted, remove any attractor whose distance to **any new node** is ≤ `d_k` (kill radius).

5. **Iteration & Termination**
   - Repeat **Attract → Grow → Update** on each two-second tick until:
     - all attractors are removed, or
     - a maximum iteration limit is reached, or
     - the user stops the simulation, or
     - **stall** is detected (e.g., zero kills for `stall_window` consecutive steps).

---

## Variations (Optional)
- **k-Nearest association per attractor**  
  Each attractor may be bound to its *k-th* nearest competing node.
  - **Global evaluation**: search over all nodes.
  - **Local evaluation**: restrict to nodes within the `d_i`/`d_k` neighborhoods.
