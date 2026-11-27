# Module documentation

Each page explains what the module detects, how, its failure modes and references. Severity is never a verdict: *info* (context), *low* (weak/common), *medium* (worth a look), *high* (strong indicator of processing or inconsistency).

| Module id | Page | Typical findings |
|---|---|---|
| `metadata` | [metadata.md](metadata.md) | editing software, date inconsistencies, thumbnail mismatch, missing/impossible metadata, XMP history |
| `provenance` | [c2pa.md](c2pa.md) | Content Credentials intact / failing, declared actions |
| `generator` | [generator-hints.md](generator-hints.md) | Stable Diffusion / ComfyUI / Midjourney parameters |
| `jpeg` | [jpeg-structure.md](jpeg-structure.md) | quality, encoder family, hidden appended data |
| `double_jpeg` | [double-jpeg.md](double-jpeg.md) | double compression, primary quality, spliced-region map, grid misalignment |
| `ela` | [ela.md](ela.md) | (visual) error level analysis |
| `noise` | [noise.md](noise.md) | noise residual, inconsistent noise region |
| `spectrum` | [spectrum.md](spectrum.md) | (visual) FFT; also channel and bit-plane viewers |
| `resampling` | [resampling.md](resampling.md) | global / local resampling |
| `copymove` | [copy-move.md](copy-move.md) | duplicated regions with match lines |
| `cfa` | [cfa.md](cfa.md) | demosaicing traces present / missing region |
| `illuminant` | [illuminant.md](illuminant.md) | (visual, experimental) illuminant colour |
| `hashes` | [phash.md](phash.md) | pHash/dHash; near-duplicate search (Tools) |
| `synthetic` | [synthetic.md](synthetic.md) | weak AI-generation hints |
| PRNU (Tools) | [prnu.md](prnu.md) | camera fingerprint match (PCE) |
