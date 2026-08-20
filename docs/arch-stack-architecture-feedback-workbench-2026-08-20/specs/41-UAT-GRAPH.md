# Spec — UAT Graph

## Entities
Persona, Capability, AcceptanceJourney, UATScenario, UATStep, AcceptanceCriterion, UATRun, UATObservation, UATArtifact, UATVerdict.

## Relations
`Capability VALIDATED_BY UATScenario`, `UATScenario HAS_STEP UATStep`, `UATRun EXECUTES UATScenario`, `UATRun PRODUCED UATArtifact`, `Change IMPACTS Capability`, `Change REQUIRES UATScenario`.

## Value
Changed files → symbols → architecture → capability → required UAT, con razonamiento explicable.
