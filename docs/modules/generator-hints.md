# Generator hints (`generator`)

**Detects:** explicit declarations by AI image generators and editors in text chunks: Stable Diffusion WebUI `parameters`, ComfyUI `prompt`/`workflow`, InvokeAI, NovelAI, Midjourney `Job ID`, AI tool names in Software/Author/Description.

## How
PNG `tEXt`/`zTXt`/`iTXt` (zlib-inflated with a 4 MB cap) and JPEG `COM` segments are extracted; keyword + value patterns map to a generator. AI product names are matched with word boundaries to avoid false hits.

## Failure modes
Metadata is easily stripped (most social networks do) or forged. Absence is meaningless; presence is a strong but forgeable hint.
