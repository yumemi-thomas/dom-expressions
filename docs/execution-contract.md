# Semantic execution contract

`CompileOptions::semantic_trace` adds a DOM-only sidecar to `compile`. The
sidecar is produced by the same lowering pass as `CompileOutput::code`; it
does not alter generated code. The producer is intentionally conservative:
facts are source spans and lowering observations, not claims about runtime
ownership, ancestry, timing, or whether a render eventually occurs.

## Totality and reconciliation

Before lowering, `ExecutionCensus` enumerates every supported JSX execution
site. During lowering, `TraceRecorder` records the decision at the emission
site. `finish()` rejects an unresolved censused site, a conflicting decision,
or a decision for a site absent from the census. The corpus test runs the DOM
fixture corpus and more than 400 parity probes through this reconciliation.

The harness also compiles every probe with tracing enabled and disabled and
asserts byte-identical `CompileOutput::code`. This is the invariant for all
future semantic-trace changes: enrichment may add facts, but `transform()`
output must not move.

## Execution sites

`ExecutionSite` contains a byte span, a closed `ExecutionSiteKind`, and one
terminal decision. The kind and decision describe the lowering branch:

- value sites use `eager-once`, `reactive-rerun`, `caller-context`, or
  `elided`;
- callback sites use `later-event`, `later-render`, or `ref-apply`;
- `control-flow-render` is emitted authoritatively for a function child of a
  configured, unshadowed built-in. Consumers must consume that fact rather
  than re-deriving built-in identity from their own JSX walk.

Unconfigured or shadowed built-ins remain ordinary `component-child` sites.
Literal-only expressions and values discarded by lowering are not sites.

## Additive wrapper facts

`owner_establishments` contains `{ span, wrapper, group_id? }`. It records
where a wrapper was emitted and preserves the wrapper identity as a string.
The consumer maps audited identities through its dialect and maps an unknown
or unaudited identity to `Unknown`; it must not infer runtime meaning from the
string. Current producer identities include:

- `effect` (or the configured effect wrapper), with one shared `group_id` for
  the entries emitted by one multi-dynamic effect;
- `memo` (or the configured memo wrapper);
- `createComponent`, `insert`, `addEventListener`, `delegated-event`, and
  `ref-apply`;
- `scope`, the 2.0-only hydration-scope emission site.

`group_id` links spans sharing one wrapper invocation. It is not a runtime
owner id and says nothing about flush order or post-flush timing.

## Component and deferred callback facts

`component_render_sites` contains `{ span }` at each `createComponent`
lowering site. It records where the render operation is emitted and does not
claim that the component will render at runtime.

`deferred_callback_sites` contains `{ span, receiver_span }`. It links a
deferred component prop, spread, or ref value to the enclosing JSX component
span. It is a source relationship only; the consumer may attach the callback
to the receiver span but must not infer callback timing or receiver behavior.

This vocabulary replaces the retired ownership-decision fields. Consumers
must migrate to the three additive fact collections before consuming this
producer pin. Unknown wrapper identities remain representable so the consumer
can fail closed to `Unknown` without losing the source location.

## Scope

Spans are byte offsets into the exact source and options used for compilation.
Only DOM generation currently produces a trace; unsupported output modes and
files skipped by `requireImportSource` return a configuration error instead of
an incomplete trace.
