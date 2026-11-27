# Frequency spectrum (`spectrum`) and channel/bit-plane viewers

## Spectrum
Log-magnitude 2-D FFT (rustfft) of the central power-of-two crop (≤1024²) of full-resolution luma, Hann-windowed, centred. Periodic patterns appear as off-axis peaks: resampling, JPEG 8×8 grids (peaks at multiples of N/8), halftones, and upsampling grids of some generative models. No automatic finding; used as a visual aid and by later modules.

## Channel viewer
Luma, R, G, B, Cb, Cr, hue and saturation with a 0.5–99.5 % contrast stretch. Chroma channels often reveal compositing invisible in RGB.

## Bit planes
Bit *k* of a channel. Camera LSB planes look like noise; flat or structured LSB planes suggest editing, synthetic content or steganography.

## Failure modes
Spectral peaks also come from textures (fabrics, fences, screens). Bit planes of heavily compressed images show block structure.
