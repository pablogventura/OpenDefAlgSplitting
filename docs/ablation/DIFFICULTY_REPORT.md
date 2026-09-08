# Difficulty predictors (exploratory)

Join of `difficulty_metrics.csv` and `results_safe.csv` (baseline variant).
Sample size: n=17 models with valid steps.

## Spearman rho (steps and ms vs static metrics)

| metric | rho(steps) | rho(ms) |
|--------|------------|---------|
| $H_{\mathrm{pat}}$ | n/a | n/a |
| $H_{\mathrm{fp}}$ | 0.772 | 0.771 |
| $H(R)$ | 0.271 | 0.322 |
| $|A|$ | 0.093 | 0.541 |
| $|A^{(k)}|$ | 0.237 | 0.648 |
| pat classes | n/a | n/a |

**Disclaimer:** n is small; correlations are exploratory, not universal predictors.
Information-gain split ordering (experimental) is not pattern entropy.

## Paper table (7 representative models)

```latex
\begin{tabular}{lrrrrr}
\hline
Model & $|A|$ & $k$ & steps & $H_{\mathrm{pat}}$ & $H_{\mathrm{fp}}$ \\
\hline
modeloqueanda & 3 & 3 & 12 & 0.000000 & 2.584963 \\
suma4 & 4 & 2 & 12 & 0.000000 & 2.751629 \\
retrombo\_nodef & 4 & 2 & 21 & 0.000000 & 1.483356 \\
msimple & 8 & 2 & 2 & 0.000000 & 1.584500 \\
cadena5 & 5 & 4 & 287 & 0.000000 & - \\
gigante & 18 & 2 & 476 & 0.000000 & 8.250852 \\
16\_T3\_4\_0 & 16 & 3 & 11 & 0.000000 & - \\
\hline
\end{tabular}
```

## Full joined table

| model | k | steps | ms | H_pat | H_fp | H_R |
|-------|---|-------|----|-------|------|-----|
| `model_examples/16_T3_4_0.model` | 3 | 11 | 1.715 | 0.000000 |  | 0.000000 |
| `model_examples/algebra.model` | 2 | 6 | 0.347 | 0.000000 | 1.584963 | 0.000000 |
| `model_examples/cadena4.model` | 3 | 42 | 1.302 | 0.000000 | 2.584963 | 0.000000 |
| `model_examples/cadena5.model` | 4 | 287 | 12.493 | 0.000000 |  | 0.000000 |
| `model_examples/gigante.model` | 2 | 476 | 690.121 | 0.000000 | 8.250852 | 0.720619 |
| `model_examples/malvada.model` | 2 | 6 | 0.580 | 0.000000 | 1.483356 | 0.000000 |
| `model_examples/miprueba.model` | 2 | 1 | 0.271 | 0.000000 | 0.000000 | 0.286397 |
| `model_examples/modeloqueanda.model` | 3 | 12 | 0.733 | 0.000000 | 2.584963 | 0.650022 |
| `model_examples/modelosexperimento.model` | 3 | 21 | 5.558 | 0.000000 | 8.392317 | 0.073603 |
| `model_examples/modelosimplequefalla.model` | 3 | 1169 | 36.870 | 0.000000 | 8.392317 | 0.482066 |
| `model_examples/msimple.model` | 2 | 2 | 0.580 | 0.000000 | 1.584500 | 0.924134 |
| `model_examples/p0_d0.0625_a2_u30_q5.model` | 2 | 1 | 0.699 | 0.000000 |  | 0.000000 |
| `model_examples/retrombo.model` | 2 | 6 | 0.462 | 0.000000 | 1.483356 | 0.000000 |
| `model_examples/retrombo_nodef.model` | 2 | 21 | 0.516 | 0.000000 | 1.483356 | 0.413817 |
| `model_examples/retrombo_nodef_sinpura.model` | 2 | 3 | 0.276 | 0.000000 | 1.483356 | 0.000000 |
| `model_examples/retromboconstantes.model` | 2 | 1 | 0.327 | 0.000000 | 1.483356 | 0.000000 |
| `model_examples/suma4.model` | 2 | 12 | 0.415 | 0.000000 | 2.751629 | 0.000000 |
