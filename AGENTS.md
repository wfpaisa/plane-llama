# AGENTS.md

Proyecto **plane-llama**: port de llama-bench (`/home/projects/plane-llama-bench`,
solo lectura) a Rust. Leer `PLAN.md` de este repo antes de empezar y seguir solo la
fase que se pida.

- El comportamiento lo define el código TS del referente; el plan es el mapa.
- UI y comentarios en español; identificadores en inglés. Comentarios cortos, al
  estilo del referente (cada módulo con un encabezado que diga su responsabilidad).
- JSON idéntico al backend TS (serde camelCase, mismos nombres y nulls).
- Nada de `unwrap()`/`expect()` fuera de tests y `main`.
- Antes de commitear: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`. Todo en verde.
- No tocar nada fuera de este repo (excepto leer el referente y, en el repo Tauri,
  leer `/home/projects/plane-llama-rust`).
- Al terminar la fase: commit `feat(fase-N): ...` y un resumen en
  `docs/progress.md` (qué quedó hecho, qué falta, decisiones tomadas).
