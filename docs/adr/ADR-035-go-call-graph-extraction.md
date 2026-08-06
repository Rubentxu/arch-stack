# ADR-035 — Go Call-Graph Extraction

> **Ciclo:** `m30-call-graph-go-support`
> **Estado:** Aceptado
> **Fecha:** 2026-08-06
> **Decisor:** architecture decision
> **Alternativas consideradas:** NA (Go real support, not graceful degradation)

## Contexto

El smoke test `smoke_echo` sobre `labstack/echo` (proyecto Go real) devolvía
`elements_written: 0` con exit 0. El extractor call-graph no soportaba Go:
los `.go` se ignoraban silenciosamente. La decisión era entre:
1. Soporte Go real (tree-sitter-go)
2. Graceful degradation con mensaje claro (0 archivos escaneados)

Se eligió **soporte Go real** para cerrar la cobertura falsa del smoke y
demostrar que el MVP set puede crecer.

## Decisiones

### D1 — Go tree-sitter source

**Elección:** `ast-grep-language` 0.45.0 con feature `builtin-parser`.

**Rationale:**
- `ast-grep-language` bundla `tree-sitter-go` v0.25.0 internamente
- `SupportLang::Go.get_ts_language()` retorna un `TSLanguage` compatible con `Parser::set_language()` de tree-sitter 0.26
- **Cero crate nuevo** — ya estaba en `Cargo.toml`
- Verificado por `astgrep.rs:210`: `Lang::from_path(Path::new("main.go")) == Some(Lang::Go)`

**Trade-offs:**
- Versión pinned a 0.45.0 — si tree-sitter-go cambiaBreaking, hay que actualizar
- No hay acceso directo a `tree-sitter-go` como crate separado (es un internal detail)

### D2 — Method identity (canonical_key)

**Elección:** Simple name solo — `go:file:Handler:42`, sin receiver qualifier.

**Rationale:**
- Matching Rust/Python convention (simple name en canonical_key)
- `func (r *T)` y `func (r T)` son ambos `method_declaration` en tree-sitter-go
- Identity = simple name (canonical_key `go:file:Name:line`)
- No se diferencia receiver kinds en MVP

**Trade-offs:**
- Dos methods con mismo nombre en diferentes receivers (ej: `T1.Method()` y `T2.Method()`) colisionan en canonical_key
- Si hay demanda real, se puede añadir receiver qualifier en Phase 2

### D3 — Anonymous funcs (func_literal)

**Elección:** `func_literal` NO produce FunctionNode. Calls inside are attributed to the nearest enclosing named function.

**Rationale:**
- `func_literal` es anonymous — no tiene nombre estable para identificarlo
- Calls dentro de func_literal deben ser atribuidos al enclosing named function (thread `parent_key` through nesting)
- El `parent_key` de la FunctionNode no se usa actualmente para nada en Go (se ignora en `extract_go_function`)
- Equivale al comportamiento de closures en Rust/TypeScript

**Implementation:**
```rust
} else if kind == "func_literal" {
    // func_literal is NOT a FunctionNode — anonymous, calls attributed to enclosing named function
    // Do NOT recurse into func_literal body (same guard as Rust closure_expression)
    return;
}
```

### D4 — func main / func init

**Elección:** Regular FunctionNode (kind: Function), igual que cualquier otra función.

**Rationale:**
- tree-sitter-go parsea `func main()` y `func init()` como `function_declaration`
- No hay distinción en el AST entre `main`/`init` y otras funciones
- Si el usuario quiere filtrar por `main`/`init`, puede hacerlo post-extracción

### D5 — Package-qualified calls (pkg.Func)

**Elección:** Extraer `field_identifier` de `selector_expression` como callee. El call_kind es `MethodCall`.

**Rationale:**
- `fmt.Println()` → call_expression → selector_expression → field_identifier ("Println")
- `s.Save()` → call_expression → selector_expression → field_identifier ("Save")
- `helper()` → call_expression → identifier ("helper")

**AST shape (tree-sitter-go 0.25.0):**
```
call_expression
├── identifier "fmt" (para pkg.Func) o implicit receiver
├── selector_expression
│   └── field_identifier "Println"
└── ... otros children
```

**Implementation:**
```rust
fn extract_go_callee(node: tree_sitter::Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            let child_kind = child.kind();
            if child_kind == "identifier" {
                return Some(source.get(child.start_byte()..child.end_byte())?.to_string());
            } else if child_kind == "selector_expression" {
                for j in 0..child.child_count() {
                    if let Some(field_child) = child.child(j as u32)
                        && field_child.kind() == "field_identifier"
                    {
                        return Some(source.get(field_child.start_byte()..field_child.end_byte())?.to_string());
                    }
                }
            }
        }
    }
    None
}
```

### D6 — Confidence

**Elección:** 0.85 (mismo que TypeScript).

**Rationale:**
- Extracción sintáctica pura, sin resolución de tipos
- No se resuelve el receiver de un method call
- No se verifica que el callee realmente exista
- TypeScript también es 0.85 — mismo nivel de certeza

### D7 — ADR

**Elección:** Este ADR documenta las decisiones D1–D6.

## Node types extracted

| Go AST node | Resulting FunctionNode |
|---|---|
| `function_declaration` | FunctionNode (kind: Function) |
| `method_declaration` | MethodNode (kind: Method) |
| `func_literal` | **NO node** (calls attributed to enclosing named function) |

## Edge types extracted

| Go AST pattern | Resulting CallEdge |
|---|---|
| `call_expression` → `identifier` | DirectCall |
| `call_expression` → `selector_expression` → `field_identifier` | MethodCall |

## MVP set

Después de M30, el MVP set para call-graph es: `{rust, typescript, python, go}`.

## Out of scope (Phase 2)

- Resolución de tipos (interfaces, generics, embedded structs)
- `func_literal` como FunctionNode
- Go en `code sequence` (funciona automáticamente una vez los edges están en el grafo)
- `vendor/` filtering (ya respetado por .gitignore del walker)

## Referencias

- Spec: `sddk/m30-call-graph-go-support/spec.md`
- Proposal: `sddk/m30-call-graph-go-support/proposal.md`
- Implementation: `archctl/src/code/call_graph.rs`
- Smoke test: `archctl/tests/smoke_real_projects.rs` (`smoke_echo`)
- Human loop: `e2e/HUMAN_LOOP_TEST.md` (Fase 6, 9.2)
