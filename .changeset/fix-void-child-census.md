---
"@dom-expressions/compiler": patch
---

Reconcile the semantic-trace census with DOM lowering over discarded child
lists, so tracing no longer fails a file it can compile. A void element in
nested native-child position keeps its children through lowering and now
censuses them; a void element that is its own template root still discards
them and claims nothing, a `children` attribute on a void element stays an
attribute site, and a nested element whose dynamic `textContent` replaces its
content retracts the children it drops. `transform()` output is unchanged.
