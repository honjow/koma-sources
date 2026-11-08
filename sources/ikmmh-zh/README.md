# Ikmmh / 爱看漫 0.1.3

Independent source `com.ikmmh.zh.koma` provides Chinese category browsing, search, manga details, complete chapter directories and original-image descriptors. It does not provide latest, home, login, settings or filters. The source is marked NSFW; individual work ratings and availability follow the provider.

The chapter directory uses the provider's `orderby=desc` request and preserves its complete newest-first sequence. The package declares `chapterOrder: newest_first` for compatible hosts; chapter IDs and displayed ordering remain unchanged. Chapter numbers remain unknown rather than being inferred from array positions.

Page requests validate chapter ownership and require all original-image metadata batches to be complete before returning a page list. Images use the existing HTTPS CDN allowlist and source image-request validation. Category pagination uses the existing 48-item heuristic, which does not establish an exact total.

The package uses the approved temporary `IKM` wordmark, not a claimed official icon. The fixed Keiyoushi reference requests the same category and detail endpoints for the affected routes. The provider returned HTTP 403 with the explicit current-region denial on both routes. Koma now preserves that outcome as `region_restricted`, using `permission_denied` when the body gives no such explanation. Other statuses and successful parsing keep their previous behavior. This change does not establish website availability or native reading.

Source behavior is independently adapted from the fixed Keiyoushi reference. Licensing and attribution are in `LICENSE` and `NOTICE.md`.
