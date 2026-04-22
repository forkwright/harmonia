# Foliate-JS Reader Bundle

This directory contains the vendored foliate-js library for in-browser ebook reading.

## Vendored Version

- **Upstream:** https://github.com/johnfactotum/foliate-js
- **Pinned SHA:** `76dcd8f0f7ccabd59199fc5eddbe012d8d463b18` (2026-04-10)
- **License:** MIT (see `LICENSE.foliate-js`)

## Supported Formats

- EPUB (reflowable and fixed-layout)
- MOBI / KF8 (AZW3)
- FB2 / FB2.zip
- CBZ (comic book archives)
- PDF (experimental; requires PDF.js)

## Entry Points

- **`view.js`** — Main library component; the `<foliate-view>` web component
- **`reader.html`** — Reference reader shell (incomplete; paroche provides its own)

## Updating

To update foliate-js to a newer version:

1. Update `PINNED_SHA` in `xtask/vendor-reader.sh`
2. Run `./xtask/vendor-reader.sh` to download and verify
3. Update this file's "Pinned SHA" section
4. Commit the vendored bundle with a message like:
   ```
   chore(paroche): vendor foliate-js <new-sha>
   ```

The vendor script enforces SHA256 verification to guard against MITM attacks and ensure reproducible builds.

## Architecture Notes

foliate-js uses native ES modules with no build step. Modules can be imported directly:

```javascript
import { View } from '/static/reader/foliate-js-76dcd8f0f7ccabd59199fc5eddbe012d8d463b18/view.js';
import EPUBBook from '/static/reader/foliate-js-76dcd8f0f7ccabd59199fc5eddbe012d8d463b18/epub.js';
```

Harmonia's reader SPA wraps the `<foliate-view>` web component to add:
- Toolbar and navigation UI
- Settings / theming controls
- Reading-state persistence (future work)
- Accessibility enhancements (future work)
