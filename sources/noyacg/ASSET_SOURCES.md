# NoyAcg package assets

- `icon.png` is the public NoyManga PWA icon published by the site currently
  reached from this source's configured `beta.noyteam.online` endpoint at
  `https://noymanga.com/icons/icon-512.png`.
- Retrieved on 2026-08-14; SHA-256:
  `a533f90586975c47dd1a8bbea4d3b173e86b00b2f1bcb2e1ca10a3e319654c44`.
- It is packaged only as the identity icon for this NoyAcg source. Do not use
  it as a manga cover or as a substitute for another source's brand asset.

## Official release identity, 0.1.1 (2026-10-20)

The former 512px PWA flower image remains historical provenance above. The package now uses the full, unmodified 908x908 transparent WebP published at https://release.noymanga.com/assets/favicon-BPmZMpu7.webp . The public main site https://noymanga.com/ links its declared app script, whose release link is https://release.noymanga.com/?source=webapp ; the release page titled 即刻下載 NoyAcg！ declares this exact image in rel=icon, type=image/webp. Root read that declaration with HTTP 200 and reviewed the original image. No cropping, repainting, resizing, added background, shadow or mask. This changes only package identity artwork; it does not establish source content availability. App slot and rounded rendering stay unchanged.


## Official mascot identity, 0.1.2 (2026-10-21)

The current 0.1.1 full-body transparent character remained narrow in the native source-icon slot. The existing full-line resource task and Root review reopen that specific suitability issue; no App geometry or rounded rendering changes are authorized here. The normal public release page https://release.noymanga.com/?source=webapp returned HTTP 200 and declares /assets/index-B4itRXxT.js. That exact public script returned HTTP 200 and explicitly renders `img src="/nry.webp" alt="Mascot"`, followed by the heading `遇見諾莉亞`. The accurate image https://release.noymanga.com/nry.webp returned HTTP 200 image/webp, 335456 bytes, 4096x4096 RGB. The complete original is retained at Koma/.hvigor/outputs/user-flow-audit/noy-release-mascot-official-original.webp.

Root and the source owner reviewed the full original: the same white-haired, green-eyed, flower-adorned mascot occupies more of the square canvas than the prior full-body transparent icon. The original scene background is part of the official image, not a new backing plate. Only the complete square image is proportionally downsampled to 512x512 with Lanczos and lossless WebP output to satisfy the existing icon pixel limit. No crop, cutout, repainting, generated detail, new background, external mask or shadow. Only this identity asset, provenance and package version change to 0.1.2; source parsing, capabilities and App presentation are preserved. On 2026-10-21, the ordinary system file picker imported 0.1.2 on device237 (VDE-AL00, unfolded portrait 1320x2120); management and Browse both visibly rendered the full official mascot scene in the existing rounded icon slot, and Root reviewed both complete original screenshots. NoyAcg was a new installation at position 49: the original 48 sourceId/enabled pairs remained an identical prefix before and after import. Only the package sent by this run was removed from Download; the original OverrideTimeout=10000ms was read back with AWAKE, and the session lease was released. Evidence remains under Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/noy012-{manager237-001,browse237-001,session-restored237-001}/. This accepts only these two icon consumers, not source content, downloads or cold-start behavior.
