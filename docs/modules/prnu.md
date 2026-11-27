# PRNU camera fingerprint (Tools → Camera fingerprint)

**Answers:** "was this image taken by the same camera as these reference photos?"

**How.** Luma of the central power-of-two crop (≤1024²). Noise residual W = I − Wiener₅ₓ₅(I) (adaptive local Wiener, assumed noise variance 9), with row/column means removed. Fingerprint K = Σ Wᵢ·Iᵢ / Σ Iᵢ² over references with the *same resolution* as the query (others are skipped). Score: circular cross-correlation of the query residual with I·K via FFT; PCE = peak² / mean energy outside an 11×11 neighbourhood. PCE ≥ 60 → consistent with the same sensor; the peak offset should be (0,0).

**Use:** at least 5 (better 20+) photos of flat, bright, unsaturated content (sky, walls), full resolution, same orientation.

**Failure modes.** Resizing, cropping (other than centred), digital zoom, stabilisation, heavy JPEG compression, HDR/multi-frame phone pipelines and strong denoising all destroy or misalign the pattern → *no match is inconclusive*. Same-model cameras with shared readout artefacts can give weak false matches (mitigated by zero-meaning). The Wiener denoiser is simpler than the wavelet denoiser of the literature, so PCE values are lower.

**References.** J. Lukáš, J. Fridrich, M. Goljan, "Digital camera identification from sensor pattern noise", IEEE TIFS 2006; M. Goljan, J. Fridrich, T. Filler, "Large scale test of sensor fingerprint camera identification", SPIE 2009.
