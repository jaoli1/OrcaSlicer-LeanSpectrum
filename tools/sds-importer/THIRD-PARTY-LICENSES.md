# Third-party licenses — Optimisateur de filament et de profils d'impression by Maison Drabiec

The Optimiser is **proprietary** (see LICENSE.md) but bundles open-source
components under their own licenses. None is GPL/AGPL (no viral copyleft);
MPL-2.0 components are used unmodified. Grouped license inventory below,
generated with `cargo license` on 2026-05-29.

## Inventory (SPDX license: crates)

```
(Apache-2.0 OR MIT) AND BSD-3-Clause (1): encoding_rs
(Apache-2.0 OR MIT) AND NCSA (1): libfuzzer-sys
(Apache-2.0 OR MIT) AND Unicode-3.0 (1): unicode-ident
0BSD OR Apache-2.0 OR MIT (1): adler2
Apache-2.0 (2): sync_wrapper, tao
Apache-2.0 AND ISC (1): ring
Apache-2.0 AND MIT (1): dpi
Apache-2.0 OR Apache-2.0 WITH LLVM-exception OR MIT (13): wasi, wasip2, wasip3, wasm-encoder, wasm-metadata, wasmparser, wit-bindgen, wit-bindgen, wit-bindgen-core, wit-bindgen-rust, wit-bindgen-rust-macro, wit-component, wit-parser
Apache-2.0 OR BSD-2-Clause OR MIT (2): zerocopy, zerocopy-derive
Apache-2.0 OR BSD-3-Clause (2): moxcms, pxfm
Apache-2.0 OR BSD-3-Clause OR MIT (2): num_enum, num_enum_derive
Apache-2.0 OR BSL-1.0 (1): ryu
Apache-2.0 OR CC0-1.0 (1): imgref
Apache-2.0 OR CC0-1.0 OR MIT-0 (1): dunce
Apache-2.0 OR ISC OR MIT (2): hyper-rustls, rustls
Apache-2.0 OR LGPL-2.1-or-later OR MIT (2): r-efi, r-efi
Apache-2.0 OR MIT (368): ahash, aligned, android_system_properties, anstream, anstyle, anstyle-parse, anstyle-query, anstyle-wincon, anyhow, arbitrary, arrayvec, as-slice, atomic-waker, autocfg, base64, base64, bit-set, bit-vec, bit_field, bitflags, bitflags, bitstream-io, block-buffer, bs58, bumpalo, camino, cargo-platform, cargo_toml, cc, cesu8, cfg-expr, cfg-if, chrono, colorchoice, console_error_panic_hook, console_log, cookie, core-foundation, core-foundation-sys, core-graphics, core-graphics-types, cpufeatures, crc32fast, crossbeam-channel, crossbeam-deque, crossbeam-epoch, crossbeam-utils, crypto-common, ctor, ctor-proc-macro, dbus, deranged, digest, dirs, dirs, dirs-sys, dirs-sys, displaydoc, dtoa, dtor, dtor-proc-macro, dyn-clone, either, embed_plist, env_filter, env_logger, equivalent, erased-serde, euclid, fallible-iterator, fallible-streaming-iterator, fastrand, fdeflate, field-offset, find-msvc-tools, flate2, fnv, foreign-types, foreign-types-macros, foreign-types-shared, form_urlencoded, futf, futures-channel, futures-core, futures-executor, futures-io, futures-macro, futures-sink, futures-task, futures-util, fxhash, getopts, getrandom, getrandom, getrandom, gif, glob, half, hashbrown, hashbrown, hashbrown, hashbrown, hashlink, heck, heck, hex, html5ever, html5ever, http, httparse, iana-time-zone, iana-time-zone-haiku, id-arena, ident_case, idna, idna_adapter, image, image-webp, indexmap, indexmap, ipnet, is_terminal_polyfill, itertools, itoa, jni, jni-sys, jni-sys, jni-sys-macros, jobserver, js-sys, json-patch, jsonptr, keyboard-types, leb128fmt, libappindicator, libappindicator-sys, libc, libdbus-sys, lock_api, log, mac, markup5ever, markup5ever, maybe-owned, md-5, mime, minimal-lexical, muda, ndk, ndk-sys, no_std_io2, num-bigint, num-conv, num-derive, num-integer, num-rational, num-traits, once_cell, once_cell_polyfill, parking_lot, parking_lot_core, paste, pastey, pathdiff, pdfium-render, percent-encoding, pin-project-lite, pkg-config, png, png, portable-atomic, portable-atomic-util, postscript, powerfmt, ppv-lite86, prettyplease, proc-macro-crate, proc-macro-crate, proc-macro-crate, proc-macro-error, proc-macro-error-attr, proc-macro2, profiling, profiling-procmacros, qoi, quick-error, quinn, quinn-proto, quinn-udp, quote, rand, rand, rand_chacha, rand_chacha, rand_core, rand_core, rangemap, rayon, rayon-core, ref-cast, ref-cast-impl, regex, regex-automata, regex-syntax, reqwest, reqwest, rustc-hash, rustc_version, rustls-pki-types, rustversion, scopeguard, semver, serde, serde-untagged, serde_core, serde_derive, serde_derive_internals, serde_json, serde_repr, serde_spanned, serde_spanned, serde_urlencoded, serde_with, serde_with_macros, serialize-to-javascript, serialize-to-javascript-impl, servo_arc, servo_arc, sha2, shlex, siphasher, siphasher, smallvec, socket2, softbuffer, stable_deref_trait, string_cache, string_cache, string_cache_codegen, string_cache_codegen, swift-rs, syn, syn, system-deps, tao-macros, tauri, tauri-build, tauri-codegen, tauri-macros, tauri-plugin, tauri-plugin-dialog, tauri-plugin-fs, tauri-runtime, tauri-runtime-wry, tauri-utils, tendril, tendril, thiserror, thiserror, thiserror-impl, thiserror-impl, time, time-core, time-macros, tokio-rustls, toml, toml, toml, toml_datetime, toml_datetime, toml_datetime, toml_edit, toml_edit, toml_edit, toml_parser, toml_writer, tray-icon, typeid, typenum, unic-char-property, unic-char-range, unic-common, unic-ucd-ident, unic-ucd-version, unicode-normalization, unicode-segmentation, unicode-width, unicode-xid, url, utf-8, utf16string, utf8_iter, utf8parse, uuid, vcpkg, version_check, wasm-bindgen, wasm-bindgen-futures, wasm-bindgen-macro, wasm-bindgen-macro-support, wasm-bindgen-shared, wasm-streams, web-sys, web-time, web_atoms, weezl, winapi, winapi-i686-pc-windows-gnu, winapi-x86_64-pc-windows-gnu, window-vibrancy, windows, windows-collections, windows-core, windows-core, windows-future, windows-implement, windows-interface, windows-link, windows-link, windows-numerics, windows-result, windows-result, windows-strings, windows-strings, windows-sys, windows-sys, windows-sys, windows-sys, windows-sys, windows-sys, windows-targets, windows-targets, windows-targets, windows-targets, windows-threading, windows-version, windows_aarch64_gnullvm, windows_aarch64_gnullvm, windows_aarch64_gnullvm, windows_aarch64_gnullvm, windows_aarch64_msvc, windows_aarch64_msvc, windows_aarch64_msvc, windows_aarch64_msvc, windows_i686_gnu, windows_i686_gnu, windows_i686_gnu, windows_i686_gnu, windows_i686_gnullvm, windows_i686_gnullvm, windows_i686_msvc, windows_i686_msvc, windows_i686_msvc, windows_i686_msvc, windows_x86_64_gnu, windows_x86_64_gnu, windows_x86_64_gnu, windows_x86_64_gnu, windows_x86_64_gnullvm, windows_x86_64_gnullvm, windows_x86_64_gnullvm, windows_x86_64_gnullvm, windows_x86_64_msvc, windows_x86_64_msvc, windows_x86_64_msvc, windows_x86_64_msvc, wry, zeroize
Apache-2.0 OR MIT OR Zlib (24): bytemuck, dispatch2, lru-slab, miniz_oxide, objc2-app-kit, objc2-cloud-kit, objc2-core-data, objc2-core-foundation, objc2-core-graphics, objc2-core-image, objc2-core-location, objc2-core-text, objc2-exception-helper, objc2-io-surface, objc2-quartz-core, objc2-ui-kit, objc2-user-notifications, objc2-web-kit, raw-window-handle, tinyvec, tinyvec_macros, zune-core, zune-inflate, zune-jpeg
Apache-2.0 WITH LLVM-exception (1): target-lexicon
BSD-2-Clause (3): av1-grain, rav1e, v_frame
BSD-3-Clause (7): alloc-no-stdlib, alloc-stdlib, avif-serialize, exr, lebe, ravif, subtle
BSD-3-Clause AND MIT (1): brotli
BSD-3-Clause OR MIT (1): brotli-decompressor
CDLA-Permissive-2.0 (1): webpki-roots
ISC (6): ego-tree, libloading, libloading, rustls-webpki, scraper, untrusted
LicenseRef-Proprietary (1): custom-filament-profile-creator
MIT (143): adobe-cmap-parser, aligned-vec, arg_enum_proc_macro, atk, atk-sys, av-scenechange, block2, built, bytes, cairo-rs, cairo-sys-rs, cargo_metadata, cfb, cfg_aliases, color_quant, combine, crunchy, darling, darling_core, darling_macro, derive_more, derive_more, derive_more-impl, dlopen2, dlopen2_derive, dom_query, embed-resource, equator, equator-macro, fax, gdk, gdk-pixbuf, gdk-pixbuf-sys, gdk-sys, gdkwayland-sys, gdkx11, gdkx11-sys, generic-array, gio, gio-sys, glib, glib-macros, glib-sys, gobject-sys, gtk, gtk-sys, gtk3-macros, http-body, http-body-util, hyper, hyper-util, ico, infer, interpolate_name, is-docker, is-wsl, javascriptcore-rs, javascriptcore-rs-sys, libredox, libsqlite3-sys, loop9, lopdf, maybe-rayon, memoffset, mio, new_debug_unreachable, nom, nom, noop_proc_macro, objc2, objc2-encode, objc2-foundation, open, pango, pango-sys, pdf-extract, phf, phf, phf, phf_codegen, phf_codegen, phf_codegen, phf_generator, phf_generator, phf_generator, phf_macros, phf_macros, phf_shared, phf_shared, phf_shared, piston-float, plist, pom, precomputed-hash, quick-xml, redox_syscall, redox_users, redox_users, rfd, rgb, rusqlite, schemars, schemars, schemars, schemars_derive, simd-adler32, simd_helpers, slab, soup3, soup3-sys, strsim, synstructure, tauri-winres, tiff, tokio, tokio-util, tower, tower-http, tower-layer, tower-service, tracing, tracing-core, try-lock, type1-encoding-parser, urlpattern, vecmath, version-compare, vswhom, vswhom-sys, want, webkit2gtk, webkit2gtk-sys, webview2-com, webview2-com-macros, webview2-com-sys, winnow, winnow, winnow, winreg, x11, x11-dl, y4m, zmij
MIT OR Unlicense (9): aho-corasick, byteorder, byteorder-lite, jiff, jiff-static, memchr, same-file, walkdir, winapi-util
MPL-2.0 (7): cssparser, cssparser, cssparser-macros, dtoa-short, option-ext, selectors, selectors
Unicode-3.0 (18): icu_collections, icu_locale_core, icu_normalizer, icu_normalizer_data, icu_properties, icu_properties_data, icu_provider, litemap, potential_utf, tinystr, writeable, yoke, yoke-derive, zerofrom, zerofrom-derive, zerotrie, zerovec, zerovec-derive
Zlib (2): foldhash, foldhash
```

## Compliance notes
- **No GPL/AGPL** dependency — proprietary distribution is permitted.
- **MPL-2.0** (cssparser, selectors, dtoa-short, option-ext): weak/file-level
  copyleft. Used UNMODIFIED; their source stays public on crates.io. No
  obligation to open the Optimiser's own source.
- **r-efi** is tri-licensed (Apache-2.0 OR LGPL-2.1+ OR MIT) — the permissive
  option is taken.
- **Permissive** components (MIT / Apache-2.0 / BSD / ISC / Zlib / Unicode)
  require their copyright + permission notices in distributions. Before
  commercial release, bundle the FULL license texts in the installer
  (e.g. via `cargo about generate`); this inventory is the interim record.
