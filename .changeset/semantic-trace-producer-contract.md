---
"@dom-expressions/compiler": patch
---

Add the DOM compiler's semantic-trace producer contract, including additive
wrapper, component-render, and deferred-callback facts, while preserving
transform output byte-for-byte. The trace remains source-span based so the
consumer can map unaudited wrapper identities to `Unknown`.
