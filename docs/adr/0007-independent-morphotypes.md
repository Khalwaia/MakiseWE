---
status: accepted
date: 2026-08-19
---

# Independent data-driven morphotypes

Human и Neko являются независимыми root `MorphotypeDefinition`, которые могут ссылаться на общие mammalian mechanisms, но не наследуются друг от друга. Runtime разрешает anatomy, development, bindings и параметры через package data без `is_neko`, закрытого enum известных morphotypes или ветвления по ID, чтобы добавление нового morphotype не требовало изменения `WorldEngine`.
