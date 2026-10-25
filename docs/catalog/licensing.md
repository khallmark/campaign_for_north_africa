# Source-class distribution record — CNA-136

This is a distribution decision record, **not a copyright license or legal determination**. Kevin authorized retaining supplied artwork in his private repository and using it in authenticated private review. That authorization does not establish unrestricted public redistribution rights. Source locks record integrity only and remain unchanged.

| Class | Origin / locations | Authorized current use | Public-release status |
|---|---|---|---|
| Original game counters in VASSAL | Supplied `docs/CNAv2.1.0/images/*.{png,svg}`, referenced by the locked module XML/census | Private repository and opt-in authenticated review | Permission/rightsholder evidence pending; no default public copy |
| VASSAL OC boards, characteristics and other charts | Supplied module chart/board images under the same directory | Private source review, transcription/OCR and verification | Underlying charts and module contributions require separate review; public distribution pending |
| Original manual scans and OCR copies | Supplied `docs/The Campaign for North Africa*.pdf`, `docs/rules.txt` | Private reference and source verification | Public republication permission not documented |
| VASSAL PDFs, text and incorporated errata | Supplied module root `.pdf`/`.txt` files | Private reference and extraction | Attribution and redistribution terms require review; not presumed cleared by module availability |
| Derived text/transcriptions | `data/sources/vassal-2.1-document-text.json`, evidence JSON, supplemental reference data | Private development/review, with source hashes and confidence labels | Extraction is not rights clearance. Review quotations/descriptions and extracted text before a public release |
| Committed reference database | `data/catalog.sqlite`, importing VASSAL metadata, manual observations, private extracted text and SQL corrections | Private repository and offline reference queries | Not a public/runtime bundle; exclude from `public/`, `dist/` and Wasm. Any future consumer projection needs explicit field selection and distribution review |
| Generated artwork | Existing `public/sprites/` illustrations; provenance in sprite manifests and `docs/unit-identities.md` | Existing application/private review, clearly labeled generated | No third-party scanned-token permission inferred; historical accuracy and generated-output rights require their own review |

## Serving boundary

The data importer never copies source image bytes into `public/`. It records original repository paths and hashes. Every original token has `public_path:null`; present images additionally have a `private_review_path` under `/__source-tokens/`. This is metadata for the frontend owner's **dev-only opt-in** route, not a new production route.

The UI owner reports that the route follows the existing `CNA_PRIVATE_REVIEW_TILES` opt-in, restricts PNG/SVG basename containment and GET/HEAD, uses private/no-store headers and an SVG sandbox CSP, and does not copy assets into production output. Authentication must be supplied by the private review environment; the environment flag alone is not authentication. Do not expose an opted-in dev server publicly. This data branch does not implement or certify the UI route.

Nonlocal/default builds receive token metadata only; source image URLs may be unavailable. Consumers must tolerate that without replacing the source token with generated art while still calling it an original token. Existing generated art remains separately addressable.

`public/data/unit-reference.json` contains metadata, stats and a quoted formation description but no image bytes. The repository/build is currently private; before any public release, review that description, source-derived texts/data and all source classes above, and add any necessary exclusion/redaction to the release build. No unrestricted redistribution decision has been recorded by this batch. Rights clearance must be documented per class rather than inferred from a SHA-256 match or Kevin's private push authorization.
