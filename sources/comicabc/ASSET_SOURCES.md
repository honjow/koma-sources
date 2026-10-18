# ComicABC source identity asset

`icon.png` is the unmodified 180 x 180 PNG obtained from the actual source origin `https://www.8comic.com/apple-touch-icon.png`, replacing its former 32 x 32 favicon after the user's current low-resolution feedback. The original request, HTTP200 response and pixels are retained in `/private/tmp/koma-source-icon-integrity-20261017/comicabc/`. This provenance records the same-origin Apple touch URL and does not claim an unverified HTML declaration.

PNG SHA-256: `e69e6a8ccb99032ef6debe2f170af1ef5e3a89b70576be199e646fd550955688` (27758 bytes). The original blue ray pattern and white 8 remain; no drawing, scaling, external backing, shadow or mask was added. The prior 32px favicon hash was `68c8132cd9587a25732dcdfc6acd1863f463be980055c7feb0945ae2a812a221`. The unrelated 440 x 100 header wordmark was not substituted.

Device 237: ordinary local import of the icon-only 0.1.9 package, installed icon/manifest/WASM/archive hashes verified; source-management list appearance reviewed. Manifest and WASM remain byte-identical to the previously installed package. App icon size, fit and clipping unchanged. Scope and data evidence: `/private/tmp/koma-icon-current-audit-20261018/hd-update/native-receipt.json`. No source-content, all-view, download or cold-start reacceptance is claimed.

## Versioned delivery

Version 0.1.10 now carries this same icon through the existing ordinary index
update route. Device 237 loaded an owned three-source index, showed the three
new versions as updatable, then completed its existing batch update. All 12
installed package/manifest/WASM/icon hashes matched the standard packages;
Library, Session, Queue and source-index preference hashes were preserved.
The owned index was removed through UI and the server stopped. This is
finite local index-update acceptance, not remote publication or renewed
source-content/cold-start acceptance. Receipt: `/private/tmp/koma-icon-current-audit-20261018/version-delivery/native-receipt.json`.
