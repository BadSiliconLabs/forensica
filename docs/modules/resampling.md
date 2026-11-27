# Resampling (`resampling`)

**Detects:** periodic correlations introduced by interpolation when an image or a region is resized or rotated.

**How (Kirchner 2008).** A fixed 3×3 linear predictor (cross +0.5, diagonals −0.25) gives residual e; the p-map exp(−e²/2) is periodic after interpolation. Its Hann-windowed spectrum is searched for the strongest peak outside the low-frequency disc (r < N/10) and away from 8×8-JPEG frequencies; score = peak / median. Global: central 512² crop at full resolution, threshold 18. Local: 128² windows (stride 64, or 128 above 8 MP), threshold 14; ≥2 hot windows covering <25 % ⇒ local finding.

**Failure modes.** Phones resample routinely (digital zoom, lens correction, stabilisation). Regular textures produce peaks. JPEG quality < ~85 masks traces. Downscaling by large factors leaves weak traces. Thresholds were calibrated on synthetic images only.

**References.** M. Kirchner, "Fast and reliable resampling detection by spectral analysis of fixed linear predictor residue", ACM MM&Sec 2008; A. Popescu, H. Farid, "Exposing digital forgeries by detecting traces of resampling", IEEE TSP 2005.
