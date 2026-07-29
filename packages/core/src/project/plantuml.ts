// ADR-0002 + ADR-0005 — IR → PlantUML projection (sequence diagram).
//
// Pure function. Symmetric to the Structurizr projection. We use sequence-
// diagram semantics (participants + edge labels) because the bundled
// PlantUML in the Kroki image most reliably classifies them — the C4 macros
// from C4-PlantUML require a remote !include which the local renderer
// cannot fetch. The C4 flavour is available via the Structurizr DSL
// projection; this projection is the lightweight UML fallback.
//
// Identifiers mirror the IR (kebab kept intact; ":" → "_" so the PUML parser
// sees valid identifiers).
import type { ArchitectureIR, IRElement, IRRelationship } from "../ir/ir.ts";

const KIND_TO_PU: Record<IRElement["kind"], string> = {
  person: "Person",
  softwareSystem: "System",
  container: "Container",
  component: "Component",
  codeElement: "Code",
};

function elementSlug(id: string): string {
  return id.replace(/[:]/g, "_");
}

function quote(s: string): string {
  return s.replace(/"/g, '\\"');
}

export function projectIRToPlantUML(ir: ArchitectureIR): string {
  const out: string[] = [];
  out.push("@startuml");
  out.push(`title archctl — Architecture IR v1 (${ir.sourceIdentitySummary})`);
  out.push("");

  // Sequence-diagram semantics — participants + edge labels. The bundled
  // PlantUML in Kroki reliably parses this form; component/class diagrams
  // require colon-bearing labels that the parser rejects inside rectangles.
  // The participant's display label must be QUOTED; the alias follows `as`
  // and is unquoted.
  for (const e of ir.elements) {
    const techSuffix = e.technology && e.technology.length > 0 ? ` [${quote(e.technology.join(", "))}]` : "";
    out.push(`participant "${quote(e.name)}${techSuffix}" as ${elementSlug(e.id)}`);
  }
  out.push("");
  for (const r of ir.relationships) {
    const label = r.description ?? r.via ?? "";
    out.push(`${elementSlug(r.source)} -> ${elementSlug(r.target)} : ${quote(label)}`);
  }
  out.push("");
  out.push("@enduml");
  return out.join("\n");
}

export function projectPUmlRelationship(r: IRRelationship): string {
  return `${elementSlug(r.source)} -> ${elementSlug(r.target)} : ${quote(r.description ?? "")}`;
}

export function projectPUmlElement(e: IRElement): string {
  const kind = KIND_TO_PU[e.kind] ?? "Container";
  return `participant ${elementSlug(e.id)} as ${quote(e.name)} (${kind})`;
}
