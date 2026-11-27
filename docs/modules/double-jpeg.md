# Double compression & JPEG grid (`double_jpeg`)

## Aligned double compression (DCT domain)
**Detects:** a JPEG that was decoded and saved again on the same 8×8 grid, the quality of the *earlier* save, and regions that lack the earlier compression (pasted in before the last save).

**How.** The baseline coefficient decoder (`dct.rs`) recovers quantised luma coefficients. For each of the first 15 zig-zag AC frequencies with ≥400 non-zero samples, we histogram |coefficient| (1…40) and search the primary step `q1` whose *reachable set* `round(j·q1/q2)` (both neighbours on exact .5 ties) captures the most histogram mass relative to a fitted Laplacian envelope. A frequency counts as evidence when mass > 0.5 and mass − expected > 0.35 (calibrated: single-compressed images ≤ 0.27, double-compressed 0.4–0.7 in tests). If at least half of the usable frequencies agree, the image is double-compressed; the estimated `q1` values are matched to IJG tables to estimate the primary quality (accepted when the median error ≤ 1 step).

**Localisation.** For each block, coefficients on unreachable bins add log-likelihood for "not double-compressed" (Lin et al. 2009 style, with P(reachable | DQ) = 0.97 and P(reachable | single) = the envelope-expected mass), aggregated over 3×3 blocks and mapped through a logistic. A connected region ≥ 24 blocks (0.4–60 % of the image) above 0.85 yields a **high** finding.

## Block-grid analysis (pixel domain)
Boundary-ness of luma steps is accumulated per phase (x mod 8, y mod 8). The peak phase gives the grid offset; peak-to-median/spread is its strength. Uses: an 8×8 grid in a PNG/TIFF ⇒ earlier JPEG (low); non-zero offset ⇒ cropped after compression; 64×64 windows with a strong (≥ 3.5) grid at a different offset clustered ≥ 4 ⇒ possible paste from another JPEG (medium).

## Failure modes
- q1 = q2 or q1 dividing q2 leaves no trace; q2 much coarser than q1 hides it.
- Non-aligned recompression (after cropping) is not handled in the DCT domain (only via the grid).
- Flat/saturated areas carry little evidence; heavy texture with 8-px periodicity mimics grids.
- Progressive or arithmetic JPEGs are not decoded.

## References
J. Lukáš, J. Fridrich, "Estimation of primary quantization matrix in double compressed JPEG images", DFRWS 2003; A. Popescu, H. Farid, "Statistical tools for digital forensics", IH 2004; Z. Lin, J. He, X. Tang, C.-K. Tang, "Fast, automatic and fine-grained tampered JPEG image detection via DCT coefficient analysis", Pattern Recognition 2009; T. Bianchi, A. Piva, "Image forgery localization via block-grained analysis of JPEG artifacts", IEEE TIFS 2012; W. Li, Y. Yuan, N. Yu, "Passive detection of doctored JPEG image via block artifact grid extraction", Signal Processing 2009.
