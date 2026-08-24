---
"@dom-expressions/compiler": patch
---

Promote a `children` attribute on a nested native element to a child insert, as
the Babel plugin does and as this compiler already did for a template root.
`<div><span children={content()} /></div>` emitted nothing at all; it now emits
`insert(_el$2, content)`. Only nested native-child position changes — every
shape that already matched the Babel plugin still does: source children shadow
the attribute, a void element's `children` attribute is never promoted, a spread
keeps `children` in the merged props, and a value the constant fold resolves
stays a `children` property write. Babel's single `children` slot is honored, so
a dynamic `textContent` after the attribute still takes the element's content
(and one before it now loses to the attribute, both matching Babel), the
textarea `value` fold still wins outright, and a `<noscript>`'s children are
still dropped. With `semanticTrace`, the shape reconciles instead of failing:
the value is reported as `jsx-child`/`reactive-rerun`, and a capture the slot's
winner discards as `jsx-child`/`elided`.
