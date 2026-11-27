# Copy-move (`copymove`)

**Detects:** regions duplicated inside the same image by a pure translation (clone stamp, copy-paste).

**How.** On luma downscaled to ≤1 MP, every 16×16 block at stride 2 is described by its 4×4 cell means (via an integral image). Blocks whose cell means are explained by a plane (flat areas, sky gradients) are skipped. Features are quantised and sorted lexicographically; each block is compared with the next 12, and pairs with mean squared cell difference ≤ 4 and spatial distance ≥ 48 px vote for their (canonical, 2-px-quantised) shift. A shift is kept when it has ≥ max(150, area/4000) votes **and dominates** (≥ 2.5×) every other shift sharing ≥10 % of its blocks — periodic textures (bricks, fences) spread votes across many comparable shifts. If the two best surviving shifts are within 1.6×, nothing is reported (lattice-like texture). Matches become a mask heatmap and up to 400 source→target lines in original coordinates.

**Failure modes.** Rotated/scaled clones, heavy post-processing, very small clones (<~40 px), and clones of perfectly periodic texture are missed. Repetitive man-made structure can still produce matches.

**References.** J. Fridrich, D. Soukal, J. Lukáš, "Detection of copy-move forgery in digital images", DFRWS 2003; V. Christlein et al., "An evaluation of popular copy-move forgery detection approaches", IEEE TIFS 2012.
