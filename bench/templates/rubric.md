# FP/FN Rubric — {{DATASET}}

> Fill in below for each dataset. Compare the bundle's nodes[] against the
> repository's actual structure (README, module layout, etc.).

## True Positives (TP)

Real containers/components in the repo that archctl correctly detected.

- [ ] <name> @ <path>
- [ ] ...

## False Positives (FP)

Containers/components archctl reported that don't exist in the repo.

- [ ] <name> @ <path>: <why is this FP?>

## False Negatives (FN)

Real containers/components in the repo that archctl missed.

- [ ] <name> @ <path>: <why was it missed?>

## Metrics

- TP: {{TP}}
- FP: {{FP}}
- FN: {{FN}}
- Precision = TP / (TP + FP) = {{PRECISION}}
- Recall = TP / (TP + FN) = {{RECALL}}
- FP ratio = FP / (TP + FP) = {{FP_RATIO}}
- FN ratio = FN / (TP + FN) = {{FN_RATIO}}

## Verdict

- [ ] FP ratio < 20% threshold
- [ ] FN ratio < 30% threshold
