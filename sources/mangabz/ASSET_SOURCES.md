# MangaBZ source identity asset

`icon.png` is the 76 x 76 standalone BZ symbol cropped from the retained official header image obtained from `https://css.mangabz.com/v202609180933/mangabz/images/logo_mangabz.png`. The original request receipt is `/private/tmp/koma-source-icon-integrity-20261017/mangabz/request.json`; its actual URL is authoritative if the documented endpoint changes.

Original header SHA-256: `d327c5b05b554dba07dcd77de80af755f6b4fd175d6c3afaff68307076606805`. Crop uses source rectangle [1,2,77,78), excluding only the neighboring wordmark and transparent margin. No rescaling, redrawing, added plate, shadow or masking. The output PNG SHA-256 is `94c05673d268d6f5460c5994cd2f06a73bed1c48e9ed8af71581862898168b26`.

Replaces the former 16 x 16 same-origin favicon (`2190dfc3d139824b09afea0aa962b2502027d2341b77d9b7316cc48972ccad98`) following the user's current low-resolution feedback. The 76px source is a resolution improvement, not an official vector or recovered high-resolution master. Automatic vector and generated redraw candidates changed the original geometry and were not adopted.

Device 237: ordinary local import of the icon-only 0.1.5 package, installed icon/manifest/WASM/archive hashes verified; source-management list appearance reviewed. Manifest and WASM remain byte-identical to the previously installed package. App icon size, fit and clipping unchanged. Scope and data evidence: `/private/tmp/koma-icon-current-audit-20261018/hd-update/native-receipt.json`. No source-content, all-view, download or cold-start reacceptance is claimed.

## Versioned delivery

Version 0.1.6 now carries this same icon through the existing ordinary index
update route. Device 237 loaded an owned three-source index, showed the three
new versions as updatable, then completed its existing batch update. All 12
installed package/manifest/WASM/icon hashes matched the standard packages;
Library, Session, Queue and source-index preference hashes were preserved.
The owned index was removed through UI and the server stopped. This is
finite local index-update acceptance, not remote publication or renewed
source-content/cold-start acceptance. Receipt: `/private/tmp/koma-icon-current-audit-20261018/version-delivery/native-receipt.json`.
