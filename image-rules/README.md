# Image rules

The three JPEG templates are extracted byte-for-byte from Koma's `SourceBaoziBannerTemplates.ets`, which cites Keiyoushi commit `949dea252eb8e867b7b40834a26e13088764ff55` and stevenyomi/baozibanner. Their SHA-256 values are:

- current: `876cb19330763cb8aaeb3b09a5d563b5ddd537715e4a2a7b273814024b2b00fe`
- old: `8d4706b611ab8c3799782698be3fe4e759d66418522504620d041e890cfc60d7`
- narrow: `d900935cae37df0fed07d74b5f51d36ac1e491c67a56f9c24ac2d46f0395cb79`

Set `PACK_VERSION` in `generate.py` when changing the published pack. Run `python3 image-rules/generate.py` with Pillow to produce the content-addressed pack and index in `image-rules/dist`. The `image-rules` branch keeps previous resources; the fixed, non-latest `image-rules-v1` release publishes the same pack before replacing its index. Each row averages 16 samples in the 20%–80% horizontal interval using integer floor. The narrow bottom match scales all 186 source rows before taking the scaled 160-row height. The generator uses Pillow's JPEG decoder; exact pixel equivalence to Harmony ImageKit has not been established.
