# Official MPC identity

Official https://mangaplus-creators.jp/ declares /img/mpc_logo.svg in multiple Logo images.
identity.original.svg preserves the retrieved bytes (23473B; SHA256
43a66b442aba9e98eab28189fc6900bd30d7278d3803520ad9376c136959a828).
icon.png is only a1024x354 render of that vector, with original aspect and alpha.
No redraw, crop, backing, shadow or mask. App slot dimensions and clipping remain.
Prior192x66 render is superseded by the original-vector render, already observed
in the actual installed and browse consumers on237 at0.1.1. It remains a wide
wordmark with small text at30vp, so this does not claim all legibility solved.
Original binding and limited native receipt:
/private/tmp/koma-icon-geometry-correction-20261018/mpc-official-svg/.
Canonical0.1.2 update acceptance remains pending until observed.

## 2026-10-22 standalone identity investigation

The complete retained `identity.original.svg` contains24 paths/nine gradients in a110×38 viewBox, with no image, symbol or external image reference; it supplies only the full official wordmark. The retained successful official about HTML (`Koma/.hvigor/outputs/user-flow-audit/mpc-about-official-20261021.html`) repeats `/img/mpc_logo.svg` and the already reviewed wide OGP. It declares no apple-touch icon/manifest or MPC-specific social-profile link. Its app-store links explicitly target MANGA Plus (`id1442476536` / `jp.co.shueisha.mangaplus`) and Jump Paint (`id1230936997` / `com.medibang.android.jumppaint`), separate products rather than an independent MPC app identity; their icons are not substitutes. Login X/Apple images are authentication-provider icons, not MPC portraits.

That exact successful parent also declares `https://mangaplus-creators.jp/build/assets/app-DOozS4q_.js`; one ordinary anonymous bounded urllib GET on this previously unread URL returned HTTP503. It was stopped without retry, alternate client/endpoint or challenge handling. No script body/new identity URI was obtained. No new source image, version, package or device action followed. The remaining gap is an explicitly bound independent MPC identity resource; the wordmark is not cropped/reassembled to invent one.

2026-10-22 new official parent: Shueisha's launch release https://prtimes.jp/main/html/rd/p/000000362.000011454.html explicitly names MPC and medibang.com/mpc. Only its first declaredImage17 was fetched once (HTTP200, JPEG237094B,1920×1080); Root viewed `Koma/.hvigor/outputs/user-flow-audit/mpc-shueisha-pr-image17-original.jpg`: full platform wordmark plus phone/service promotional layout, without an independent square identity. Not adopted or cropped; existingSVG/version remain unchanged. No other press images or the prior503 script were fetched. A captcha word in a separate successful Link-u products HTML probe did not prove an actual challenge; that branch remains unknown, not classified403.
