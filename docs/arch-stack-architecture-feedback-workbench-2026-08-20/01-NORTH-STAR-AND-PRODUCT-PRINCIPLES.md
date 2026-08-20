# North Star & Product Principles

## North Star

Reducir el coste cognitivo de comprender y mantener arquitectura software.

## Jobs humanos

1. Orientarme — ¿qué hay aquí y qué es importante?
2. Entender — ¿cómo se relacionan estas piezas?
3. Explicar — ¿por qué existe esta dependencia?
4. Verificar — ¿puedo confiar en esta conclusión?
5. Comparar — ¿qué cambió realmente?
6. Predecir — ¿qué romperá este cambio?
7. Decidir — ¿qué alternativa es mejor?
8. Corregir — esto que ha inferido el sistema es incorrecto.
9. Recordar — ¿por qué tomamos esta decisión?
10. Comunicar — explícaselo visualmente a otra persona.

## Principios

- **Question before notation.** El usuario formula la pregunta; el sistema selecciona o recomienda una lente.
- **Model != View.** El grafo canónico y sus vistas nunca son la misma entidad.
- **Evidence before confidence.** Todo resultado importante explica por qué se cree.
- **Preserve contradictions.** Intent, código y runtime pueden discrepar.
- **Progressive disclosure.** Overview → focus/filter → details/evidence.
- **Stable identity across views.** Una entidad conserva ID, selección y contexto.
- **Agent output is structured.** Markdown es explicación secundaria.
- **Human feedback is data.** Aceptar/rechazar/corregir persiste y afecta futuro contexto.
- **Deterministic treatment of probabilistic output.** El LLM puede variar; validación/promoción no.
- **No big-bang rewrites.** Reutilizar LadybugDB, Rust, SolidJS, G6 y ELK.
- **Visual features must prove value.** UAT por tarea, no por estética.
- **Local-first/source-read-only by default.**
