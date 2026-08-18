---
status: accepted
date: 2026-08-19
---

# Unified causal graph, cross-cutting timeline and external model improvement

MakiseWE использует единый causal graph с feedback и mixed resolution. L0–L7 являются causal domains, не последовательным pipeline или отдельными engines; durable causal timeline поперечно записывает их committed transitions, а не образует `WORLD EVENTS` layer. Такой выбор сохраняет причинные связи между physics, biology, neural state, cognition и action без требования одинаковой детализации повсюду.

Resolution меняется только explicit causally triggered `ResolutionChanged`: trigger объявлен contract-ом, split/merge сохраняет quantities, lineage и observable continuity, а capacity failure не разрешает hidden downgrade. Model improvement вынесен во внешний control plane; validated candidate artifact активируется только авторизованным commit с old/new digests и rollback, поэтому прежняя история replay-ится точными архивными artifacts.
