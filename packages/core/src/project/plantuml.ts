// ADR-0002 + ADR-0005 — IR → PlantUML C4 projection.
//
// Pure function. Symmetric to the Structurizr projection but emits PlantUML
// using the C4-PlantUML macro library. Identifiers mirror the IR (kebab →
// snake) so the two projections are diffable line-by-line.
//
// PlantUML is **optional** at Gate Zero; it is required for the M1 spike
// report's UML arm but the C4 arm can ship without it.
import type { ArchitectureIR, IRElement, IRRelationship } from "../ir/ir.ts";

const KIND_TO_PU: Record<IRElement["kind"], string> = {
  person: "Person",
  softwareSystem: "System",
  container: "Container",
  component: "Component",
  codeElement: "Component",
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
  out.push("!include https://raw.githubusercontent.com/plantuml-stdlib/C4-PlantUML/master/C4_Container.puml");
  out.push(`title archctl — Architecture IR v1 (${ir.sourceIdentitySummary})`);
  out.push("");
  out.push("LAYOUT_WITH_LEGEND()");
  out.push("");

  for (const e of ir.elements) {
    const kind = KIND_TO_PU[e.kind] ?? "Container";
    const tech = e.technology && e.technology.length > 0 ? `, "${quote(e.technology.join(", "))}"` : "";
    const desc = e.description ? `\\n\\n<size:10>${quote(e.description)}</size>` : "";
    out.push(`${kind}(${elementSlug(e.id)}, "${quote(e.name)}"${tech}${desc})`);
  }
  out.push("");
  for (const r of ir.relationships) {
    const via = r.via ? ` : ${quote(r.via)}` : "";
    out.push(`Rel(${elementSlug(r.source)}, ${elementSlug(r.target)}, "${quote(r.description ?? "")}"${via})`);
  }
  out.push("");
  out.push("@enduml");
  return out.join("\n");
}

export function projectPUmlRelationship(r: IRRelationship): string {
  return `Rel(${elementSlug(r.source)}, ${elementSlug(r.target)}, "${quote(r.description ?? "")}")`;
}

export function projectPUmlElement(e: IRElement): string {
  const kind = KIND_TO_PU[e.kind] ?? "Container";
  return `${kind}(${elementSlug(e.id)}, "${quote(e.name)}")`;
}
