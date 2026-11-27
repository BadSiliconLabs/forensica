# Error Level Analysis (`ela`)

**Shows:** how much each pixel changes when the image is re-saved as JPEG at quality *q* (default 90, adjustable 50–99 in Tools).

## How
Re-encode the decoded image with the `image` crate's JPEG encoder, decode it, take the maximum absolute per-channel difference, render with an inferno colormap scaled to the 99.5th percentile.

## Reading it
Regions edited *after* the last compression (pasted, cloned, painted) have not converged to the same quantisation grid and often look brighter (or darker for heavy smoothing) than comparable surfaces. Compare like with like: edges, text and fine textures are always bright; flat skies are always dark.

## Failure modes
- Produces no automatic finding by design: interpretation needs a human.
- Meaningless for PNG screenshots of JPEGs, heavily recompressed images, or images resaved at the same quality many times.
- Different encoders than the original produce global differences.

## References
N. Krawetz, "A Picture's Worth…", Black Hat 2007; Wang et al., "Error level analysis…", 2010 (critique of ELA reliability is widespread; treat as visual aid).
