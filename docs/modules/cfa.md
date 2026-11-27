# CFA consistency (`cfa`)

**Detects:** presence of Bayer demosaicing traces, and regions where they are missing.

**How.** Green channel, full resolution. Prediction error of each pixel from its 4-neighbour mean; per 32×32 block F = ln(var(e) on even lattice / var(e) on odd lattice). Camera images show |F| ≫ 0 with a consistent sign. If the median |F| over textured blocks > 0.35 we report traces present; blocks where F collapses towards 0 or flips (≥ 0.9 "missing") forming a connected region ≥ 4 blocks (< 30 % of the image) give a **medium** finding. Skipped for JPEGs whose luma table mean step > 4 (≈ q < 90).

**Failure modes.** Modern phone pipelines (multi-frame merge, denoise, sharpening, AI demosaicing) often erase the pattern globally ⇒ "no traces" is informational only. Dark, saturated or blurred areas weaken it.

**References.** A. Popescu, H. Farid, "Exposing digital forgeries in color filter array interpolated images", IEEE TSP 2005; P. Ferrara, T. Bianchi, A. De Rosa, A. Piva, "Image forgery localization via fine-grained analysis of CFA artifacts", IEEE TIFS 2012.
