# Numera

A clean, minimal scientific and graphing calculator for macOS, Windows and Linux, written in Rust (egui).

## Build

```
bash build.sh
```

This compiles a release build and assembles `Numera.app` in this folder. The script offers to
install Rust if it is missing. For development, `cargo run` starts the app directly and
`cargo test` runs the math engine tests.

## Modes (⌘1 – ⌘8)

| Mode | What it does |
| --- | --- |
| Calculate | Scientific calculator with history, live preview, complex numbers, variables, fractions / π / surd forms |
| Graph | y = f(x), polar (use θ) and parametric `(x(t), y(t))` curves; roots, extrema, intersections between every pair of curves, integrals, trace, PNG export |
| Table | Value tables for the graphed functions |
| Equation | Polynomial roots (degree 2–6), simultaneous linear equations (2–6 unknowns), numeric solver |
| Matrix | Six matrices up to 6×6; determinant, inverse, transpose, row-reduce, rank, trace, free expressions |
| Statistics | One-variable statistics, six regression models with plot, normal / binomial / Poisson distributions |
| Convert | Units in 13 categories, number bases, physical constants |
| Programmer | One integer in hex, decimal, octal and binary; 8–64-bit words, signed or unsigned; clickable bit grid; AND, OR, XOR, NOT, shifts, rotates and wrapping arithmetic |

## Simple and full view

The app opens as a simple calculator. The `›` arrow at the top left expands it to the full
calculator with scientific keys and all modes; `‹` goes back. The `?` button opens the input guide. The version is shown at the bottom of the guide.

## Settings and conveniences

- **Menu bar (macOS)**: Numera, Edit, View, Window and Help menus. View switches modes (⌘1 – ⌘8), the full calculator (⌘0), appearance, angle unit and number format; Window has Keep on Top; Help opens the input guide (⌘?).

- **Settings** (sliders icon, top right): appearance (System, Light, Dark), digit grouping (1,234,567 · 12,34,567 · 1 234 567), decimal point or comma, keep on top, clear history. Settings are remembered.
- **Keep on top**: the pin icon keeps the window above other apps.
- **Copy**: click a result to copy it; right-click for *Copy as shown*, *Copy calculation* or *Insert into the input*. ⌘⇧C (Ctrl+Shift+C) copies the latest result.
- **History**: kept between launches, together with `Ans` and stored variables.

## Typing expressions

- Operators: `+ - * / ^ !  %`, implicit multiplication (`2π`, `3(4+1)`, `2sin(30)`)
- Return or `=` evaluates, ↑ / ↓ recall history, Esc clears
- Store a value: `5 → A` (type `->`), then use `A` anywhere. `Ans` is the last result
- Powers: `x^12` is shown as x¹² once you type on
- Greek letters: type `alpha`, `theta`, `Delta` … or use the `αβγ` key; they work as variables
- Constants: `π` or `pi`, `e`, `i`
- Functions: `sin cos tan asin acos atan sinh cosh tanh asinh acosh atanh ln log log(b,x) log2 exp sqrt cbrt root(n,x)
  abs arg conj re im int frac floor ceil round(x,n) sign gamma nCr nPr gcd lcm mod min max rand randint`
- Calculus: `diff(x^3, 2)`, `integ(x^2, 0, 3)`, `sum(k^2, k, 1, 10)`, `prod(k, k, 1, 5)`, `solve(x^2-2, 1)`, `seq(n^2, n, 1, 5)`
- Lists: `{1,2,3}`, `mean median stdev stdevp variance sum prod sort len dot cross`
- Matrices: `[[1,2],[3,4]]` or `[1,2;3,4]`, `det inv trn rref rank trace identity norm`, `MatA … MatF`
- Number bases: `0xFF`, `0b1010`, `0o17`, `and or xor not shl shr`
- Distributions: `normpdf normcdf invnorm binompdf binomcdf poissonpdf poissoncdf`

## Layout

```
src/engine.rs     lexer, parser, evaluator
src/numeric.rs    calculus, root finding, polynomial roots, linear systems
src/matrix.rs     matrix type
src/stats.rs      statistics, regression, distributions
src/fmt.rs        number formatting and exact forms
src/units.rs      unit tables and constants
src/theme.rs      colours and fonts          src/widgets.rs   shared controls
src/plot.rs       plot surface               src/ui_*.rs      one file per mode
```

The interface font is Inter (SIL Open Font License, see `assets/Inter-LICENSE.txt`).

## Download

Pick the file for your system from the [`downloads`](downloads) folder (click the file, then the download button):

- **macOS (Apple silicon):** [`Numera-1.1.0-macOS-AppleSilicon.zip`](downloads/Numera-1.1.0-macOS-AppleSilicon.zip). This is version 1.1.0 until the next Mac build. Unzip it, drag Numera to Applications, then right-click it and choose Open the first time (the app is not notarised). Intel Macs: build it yourself with `bash build.sh`.
- **Linux:** [`numera_1.5.1_amd64.deb`](downloads/numera_1.5.1_amd64.deb) for Debian, Ubuntu and Mint (`sudo apt install ./numera_1.5.1_amd64.deb`), or [`Numera-1.5.1-linux-x86_64.tar.gz`](downloads/Numera-1.5.1-linux-x86_64.tar.gz) for any other distribution (extract it and run `./install.sh`).
- **Windows:** the installer is not published yet. Build it on a Windows PC with `build-windows.bat` (see below).

## Signed and notarised Mac builds

The release workflow signs and notarises the Mac app when these repository secrets are set
(Settings → Secrets and variables → Actions); without them it makes an unsigned build.

| Secret | Value |
| --- | --- |
| `MACOS_CERT_P12` | Your *Developer ID Application* certificate exported as .p12, base64-encoded (`base64 -i cert.p12 \| pbcopy`) |
| `MACOS_CERT_PASSWORD` | The password chosen when exporting the .p12 |
| `APPLE_ID` | The Apple ID email of the developer account |
| `APPLE_TEAM_ID` | The 10-character team ID |
| `APPLE_APP_PASSWORD` | An app-specific password from appleid.apple.com |

## Windows

On a Windows 10/11 PC, copy this folder over and double-click `build-windows.bat` (or run `.\build-windows.ps1` in PowerShell).
It compiles the app and produces `dist\Numera-<version>-Setup.exe`, a normal installer with a Start-menu entry, an optional desktop shortcut and an uninstaller.
Needs Rust (MSVC toolchain), Visual Studio Build Tools ("Desktop development with C++") and Inno Setup; the script offers to install Rust and Inno Setup through winget.
