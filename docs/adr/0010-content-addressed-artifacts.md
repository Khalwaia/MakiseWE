---
status: accepted
---

# Content-addressed model and solver artifacts

Каждая committed transition ссылается на digests mechanism, model, resolution и solver artifacts. Fast replay применяет сохранённые deltas, а audit replay загружает точные архивные bytes по digest и пересчитывает переход; отсутствие совместимого artifact вызывает `SafeStop`. Имена версий остаются человекочитаемыми метаданными, но не заменяют идентичность содержимого.
