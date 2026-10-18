# xkcd identity, external backing cleanup

The original96px extension PNG remains identity.original.png, SHA256
af54452ef43503c5d08363ae37bf6b2602e6aeb2960a525fb4470ddbfcb82478.
Original fixed Keiyoushi provenance is recorded in NOTICE.

Official https://xkcd.com/ declares /s/919f27.ico. The Chinese provider
https://xkcd.tw/ declares /img/favicon.ico. Both retrieved originals have
the same2166B SHA256 c28f469c450e9ab2b86ea47038d2b324c6ad3b1e9a4bd8960da13214afd0ca38,
retained in identity.favicon.original.ico. They corroborate the two-person
identity but their16/32px frame/backing is not used as the displayed icon.
The existing direct-ICO candidate remains rejected, not silently reopened.

identity.derived.svg is a derived2D curve model, NOT an original official
vector. It preserves the curved trapezoidal left head, rounded rectangular
right head, hand/limb relationships and source-canvas positions. Curves and
stroke centerlines describe intended geometry rather than blurred pixel
steps. icon.png is its512x512 transparent librsvg render; no external white
plate, shadow, mask, new text or App slot/clip change.

An imagegen alternative changed head shape and stroke weight and was rejected;
it is not in this package. Current original-first evidence, history, candidate
review, standard comparison and finite Native receipt remain at
/private/tmp/koma-xkcd-transparency-20261019/.
Brand/icon rights remain with their holders; no affiliation asserted.
