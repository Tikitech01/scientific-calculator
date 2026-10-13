# Changelog

All notable changes to Scientific Calculator. Downloads for the latest version are in the
[README](README.md#download).

## 2.5.0 — 14 October 2026

**Engineering**
- Physical constants by name: c, g, G, h, hbar, qe, eV, me, mp, mn, amu, NA, kB, Rgas, eps0 (ε0),
  mu0 (μ0), Z0, kSB, atm. 2nd + αβγ shows them on the keys (CONST).
- SI prefixes glued to numbers: 4.7k, 220µF, 10nF, 5mA, 3.3kV, 1MHz
- Eng display format: 3.197k, 2.2m, 4.7µ
- Parallel operator: 10k ∥ 4.7k (2nd + %, or type ||); works for complex impedances
- dB(x), dBv(x), invdB(x), invdBv(x)

**Maths**
- limit(sin(x)/x, x, 0), limit((1+1/x)^x, x, inf)
- taylor(e^x, x, 0, 4): the coefficients of the Taylor series
- Matrices: eig(MatA) eigenvalues, eigvec(MatA) eigenvectors, with buttons in Matrix mode
- Complex arguments to sin, cos and tan now follow the angle unit like real ones

## 2.4.0 — 13 October 2026

**Feel**
- Smooth animation throughout: keys press in, tabs and switches glide, panels and new results
  fade in. Settings ▸ Animation turns it off or sets Slow / Normal / Fast. Graphs never animate.
- Redesigned Settings and Input guide: grouped cards, toggle switches, clearer spacing. In the
  simple calculator, Settings opens as its own page.
- Faster: graph analysis, statistics columns and Tukey's test are only recomputed when their
  inputs change; smaller, faster release build.

**Fixes**
- Highlight behind the selected option in switches always sits under the selected label.
- Help menu: Check for Updates and Send Feedback.

## 2.3.0 — 12 October 2026

**Statistics**
- Tukey HSD post-hoc comparisons after one-way ANOVA
- Two-way ANOVA with interaction and cell means
- Non-parametric tests: Mann–Whitney U, Wilcoxon signed-rank, Kruskal–Wallis, Jarque–Bera

**Graph**
- Sliders for any letter in a function (`a·x² + b`), with animation
- Inequalities with shaded regions (`y < x^2`)
- Derivative curve and a tangent line with its slope while tracing
- Plot Statistics data as points

**Calculate**
- Percentages like a desk calculator: `200 + 10%` = 220
- Polar complex numbers: type `5∠30`; optionally show results as r∠θ
- Unit conversion inline: `5 km to m`, `98.6 °F in °C`

**Everyday**
- Text size, open as simple or full calculator, last mode remembered
- Paste data into Statistics from a spreadsheet; copy tables back out

## 2.2.0 — 12 October 2026

- Calculator types: Scientific, Financial, Statistics, Programmer, All in one
- Finance mode: time value of money (N, I/Y, PV, PMT, FV), loan / EMI with schedule, compound
  interest and CAGR, NPV / IRR / MIRR / payback, depreciation, break-even, margin, GST

## 2.1.0 — 12 October 2026

- Hypothesis tests: t, z, F, χ², one-way ANOVA, correlation, with p-values, critical values and
  confidence intervals
- Student t, χ² and F distributions

## 2.0.0 — 12 October 2026

- Renamed to Scientific Calculator; Linux packages (.deb and .tar.gz)
- About window; the simple calculator expands towards the middle of the screen

## 1.4.0 – 1.5.0 — 11 October 2026

- Native macOS menu bar; window never runs off screen when expanding

## 1.3.0 — 11 October 2026

- Copy results, saved history, digit grouping and decimal comma, keep on top
- Programmer mode (HEX/DEC/OCT/BIN, bit grid, bitwise operations)
- Graph intersections and PNG export

## 1.2.0 — 11 October 2026

- System, Light and Dark appearance

## 1.1.0 — 11 October 2026

- First public version: calculate, graph, table, equation, matrix, statistics and convert modes
