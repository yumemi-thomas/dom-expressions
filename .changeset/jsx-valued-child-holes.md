---
"@dom-expressions/compiler": patch
---

Match Babel when a native child hole contains a JSX element. DOM and universal output now preserve the element setup IIFE inside the child getter, while SSR preserves Babel's expression-scope variable hoisting.
