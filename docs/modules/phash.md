# Perceptual hashes and near-duplicate search (`hashes`, Tools → Near-duplicates)

**How.** pHash: 32×32 grey, 2-D DCT, the 8×8 lowest frequencies (DC excluded) thresholded at their median → 64 bits. dHash: 9×8 grey, sign of horizontal differences → 64 bits. Both are computed on ≤4 MP and written to the report. The search hashes every chosen file (or every image in a folder, up to 3 levels / 5000 files, Linux) in parallel and lists those within Hamming distance 12 of pHash, sorted by distance (0 = same picture; ≤ 6 = resized/recompressed copy; 7–12 = similar, e.g. light edits or crops).

**Failure modes.** Large crops, rotations, mirroring and heavy edits change the hash; unrelated images with similar layout can collide at distance ~10–12. Everything stays on the device; no reverse image search is performed.

**References.** C. Zauner, "Implementation and benchmarking of perceptual image hash functions", 2010; N. Krawetz, "Looks Like It" (dHash), 2013.
