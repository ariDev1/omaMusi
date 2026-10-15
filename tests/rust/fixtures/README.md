# Migration oracle

`visual-reference.json` and the fixed star assets were exported by
the migration exporter (retained in Git at `d7a4bf5:scripts/export-visual-reference.py`) from the Python implementation at
development `d000103`. Inputs use fixed f32 sine buffers, 48 deterministic
16.666667-ms steps, quiet/bass/mid/high/maximum input, 48/96-kHz rates and
paused frames. A separate forced beat-phase sequence exercises director scene
changes. Waveform probes include transparent pixels and density/palette output.

Comparisons permit relative/absolute 2e-5 for analysis state, 2e-4 for FFT bands,
and relative 2e-5 for aggregated f32 waveform density. Pixel channel tolerances
are one byte for floating-point interpolation. These permit numeric-library
rounding; changed algorithms or parameters must not be hidden by wider tolerances.
