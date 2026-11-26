# Noise residual (`noise`)

**Shows:** the high-frequency residual (luma minus 3×3 median) and a 32×32-block map of the noise level (MAD-based σ).

## Finding `noise.inconsistent_region` (low)
Blocks with mid-range brightness and below-median gradient energy (to limit texture leakage) are compared on log σ using robust z-scores (median/MAD). A connected region of ≥4 blocks with |z| ≥ 3.5 (grown at |z| ≥ 2.5) covering 1–50 % of comparable blocks yields a *low* finding with confidence 0.3–0.7.

## Failure modes
Noise depends on brightness, ISO, denoising, HDR merging, focus and sharpening. Skies after denoising, bokeh, and phone "beauty" filters can all trigger it. Sophisticated splicers add matching noise.

## References
B. Mahdian, S. Saic, "Using noise inconsistencies for blind image forensics", IVC 2009; J. Lukáš et al. (noise residuals).
