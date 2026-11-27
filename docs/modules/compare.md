# Compare two versions (Tools → Compare with another version; `forensica-cli compare a b`)

**Answers:** given two files of the same scene, *what* changed and *which one is derived from the other* — works without any metadata.

**How.**
1. Both images are resampled to the smaller one's size (≤ 1 MP), luma-normalised (mean/contrast), and aligned by exhaustive translation search (±12 px).
2. Per 16×16 block, mean absolute difference; blocks above max(3× median, median + 12) form the change map.
3. For each file at its own native resolution: noise level (mean |3×3-median residual|) inside vs outside the changed blocks; and, on the common grid, fine detail (mean |Laplacian|) in the *unchanged* blocks.
4. Indicators: share of the picture that differs; **direction** — the version with ≥ 1.4× more detail in identical areas is the closer-to-original one, because resizing, smoothing or AI re-rendering cannot create genuine detail; and noise inconsistency of the changed region within each file.

**Worked example.** A phone photo vs. the same photo after an AI editor added a glass of juice (metadata stripped from both): the map isolates the glass; the original has ≈2× more detail and normal sensor noise in unchanged areas → the edited copy is identified as derived, so the glass was added in it.

**Failure modes.** Different framing/rotation/perspective (only translation is searched); both versions independently processed from a third original; aggressive sharpening; heavy in-camera denoising. Removal vs addition is decided from the direction indicator, not from the map itself.
