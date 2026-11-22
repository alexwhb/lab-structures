# Data sources and provenance

SPEC **PROV-2**: every constant the app uses, its provenance class and, for representative
values, a public reference. Structures Playground is **educational, not for real structural
design**: nothing here is certified data, and no value may be used to size a real structure.

All values were typed in by hand from the design mockups (`docs/design/mockup/*.dc.html`),
which in turn use standard textbook and design-code figures. **No database is bundled or
extracted.** Nothing comes from MatWeb (its terms forbid redistribution). The Materials Project
(CC BY 4.0) is not used today; if a future feature uses it, it must be credited here and in
the About box ("Data: Materials Project, materialsproject.org, CC BY 4.0").

Policy: constants are documented, never "corrected" silently. A value that looks physically
off goes to `PROGRESS.md` › Requests for the lead to decide.

## Provenance classes

| Class | Badge dot | Meaning |
|---|---|---|
| **Exact** (for the model) | green `#5FBF8A` | Follows exactly from the stated model: linear pin-jointed truss statics, Euler buckling, the uniform-cantilever mode shape, unit conversions, geometry. The *model* is an idealisation; the arithmetic is not. |
| **Representative** | sand `#D6C7A1` | A typical published value for a named material grade or class, inside its standard or textbook range. Real products vary, and design values differ from typical ones. |
| **Simplified** | sand `#D6C7A1` | A real physical law used in a reduced form (one mode, one soil layer, a power-law wind profile, linear interpolation), with representative coefficients. |
| **Illustrative** | grey `#A3A3A3` | Chosen to show the right trend and order of magnitude. Not taken from a single source; don't quote it. |
| **Game** | grey `#A3A3A3` | Made up for play: costs, budgets, stars, timings, challenge thresholds. |
| **Drawing** | none | Pixel geometry, exaggeration factors, animation pacing. They change how things look, never a number shown. |

Badges on screen (PROV-1): Bridge "Solver: linear statics, exact" · "Materials: representative" ·
"Game costs"; Advanced the same plus the netlist's `; representative` comment; Tower "Wind +
sway: simplified cantilever" · "Comfort bands: illustrative" · "Game costs"; Ground "Bearing +
settlement: simplified" · "Soil presets: illustrative" · "Game costs"; Materials "Curves:
representative" · "Heat effects: illustrative" · "Not certified data".

## References

Public standards and textbooks; the figures quoted are the commonly published ones.

- **[AISC]** ANSI/AISC 360, *Specification for Structural Steel Buildings*: E = 29 000 ksi
  (200 GPa) for structural steel.
- **[A36]** ASTM A36/A36M, *Carbon Structural Steel*: minimum yield 36 ksi (250 MPa), tensile
  58–80 ksi (400–550 MPa).
- **[EN10025]** EN 10025-2, *Hot rolled products of structural steels*: S275 — ReH ≥ 275 MPa
  (t ≤ 16 mm), Rm 410–560 MPa, elongation A ≈ 22–23 %.
- **[EC0-1]** EN 1991-1-1, *Densities, self-weight*: steel 77–78.5 kN/m³ (7850 kg/m³),
  normal-weight concrete 24 kN/m³ (25 reinforced).
- **[EC3-1-2]** EN 1993-1-2, *Steel — structural fire design*, Table 3.1: effective yield
  strength factor k_y,θ = 1.00 up to 400 °C, 0.78 at 500 °C, **0.47 at 600 °C**.
- **[AA]** Aluminum Association, *Aluminum Standards and Data* / ASM Handbook Vol. 2: 6061-T6
  typical yield 276 MPa (40 ksi), tensile 310 MPa (45 ksi), elongation ≈ 12 %, E 68.9 GPa,
  ρ 2700 kg/m³.
- **[EC9]** EN 1999-1-1, *Aluminium structures*: 6061-T6 extrusions f0 = 240 MPa (0.2 % proof,
  characteristic), E = 70 GPa. EN 1999-1-2 (fire): 6xxx-T6 strength falls to roughly a third by
  300 °C and to around a tenth by 350–400 °C.
- **[EC2]** EN 1992-1-1, Table 3.1: C30/37 — fck 30 MPa, fcm 38 MPa, Ecm 33 GPa, strain at peak
  εc1 ≈ 0.22 %, ultimate εcu1 = 0.35 %, mean tensile strength fctm 2.9 MPa (about a tenth of
  the compressive strength).
- **[EC2-1-2]** EN 1992-1-2, Table 3.1 (siliceous aggregate): fc,θ/fck = 1.00 to 100 °C, 0.95 at
  200 °C, 0.85 at 300 °C, 0.75 at 400 °C, 0.60 at 500 °C, **0.45 at 600 °C**.
- **[FPL]** USDA Forest Products Laboratory, *Wood Handbook* (FPL-GTR-190, 2010), ch. 5:
  clear softwoods along the grain — E ≈ 8–14 GPa, density ≈ 350–550 kg/m³, tensile strength
  parallel to the grain ≈ 60–140 MPa, brittle in tension.
- **[EN338]** EN 338, *Structural timber — strength classes*: C24 — f_m,k 24 MPa, f_t,0,k 14.5,
  f_c,0,k 21, E0,mean 11 GPa, ρ_mean 420 kg/m³.
- **[EN14080]** EN 14080, *Glued laminated timber*: GL24h — f_m,k 24 MPa, E0,mean 11.5 GPa,
  ρ_mean 420 kg/m³.
- **[EC5-1-2]** EN 1995-1-2, *Timber — structural fire design*: the char line is the 300 °C
  isotherm; strength and stiffness fall in the heated zone below it.
- **[CMH17]** *Composite Materials Handbook* (CMH-17, formerly MIL-HDBK-17) and fibre makers'
  data: quasi-isotropic carbon/epoxy laminates — E ≈ 45–70 GPa (standard to intermediate
  modulus fibre), tensile strength ≈ 500–800 MPa, linear to failure, ρ ≈ 1550–1600 kg/m³;
  epoxy glass transition typically 120–200 °C.
- **[ASCE7]** ASCE/SEI 7, *Minimum Design Loads*: velocity pressure q = 0.613·V² (SI, Pa, with
  V in m/s), power-law profiles with exponents ≈ 0.1–0.33 by terrain.
- **[EC1-4]** EN 1991-1-4, *Wind actions*: Annex E — Strouhal number St = 0.12 for square
  sections; force coefficients for rectangular prisms.
- **[ISO10137]** ISO 10137, *Serviceability of buildings against vibration*, Annex D, and
  common tall-building practice (≈ 5–7 milli-g residential, 10–15 milli-g office, 1- to 10-year
  winds).
- **[Blevins]** R. D. Blevins, *Formulas for Natural Frequency and Mode Shape*: uniform
  cantilever f1 = (1.8751²/2π)·√(EI/(μL⁴)) = 0.5596·√(EI/(μL⁴)).
- **[DenHartog]** J. P. Den Hartog, *Mechanical Vibrations*: optimum tuned-mass-damper tuning
  and the equivalent damping of a small mass ratio.
- **[BS8004]** BS 8004 / IBC Table 1806.2 presumptive bearing values: dense sand and gravel
  ≈ 200–600 kPa, stiff clay ≈ 150–300 kPa, soft clay < 75 kPa, loose sand < 100 kPa.
- **[Tomlinson]** M. J. Tomlinson, *Pile Design and Construction Practice*: shaft friction ≈
  5–25 kPa in soft clay, tens of kPa in stiff clay and dense sand.
- **[Terzaghi]** K. Terzaghi, R. B. Peck, G. Mesri, *Soil Mechanics in Engineering Practice*:
  one-dimensional consolidation, settlement approaching its final value as 1 − e^(−t/τ)
  (first-term approximation).
- **[Burland]** J. B. Burland, M. Jamiolkowski, C. Viggiani, "Leaning Tower of Pisa: behaviour
  after stabilization operations" (2009): tilt ≈ 5.5° before 1990, ≈ 4° (3.97°) after 2001.

---

## Shared (all worlds)

| Constant | Value | Class | Source / note | Where |
|---|---|---|---|---|
| Gravity g | 9.81 m/s² | Exact (rounded standard gravity 9.80665) | SI | `truss::G`, `tower::G` |
| Tonne-force | 9 810 N per t | Exact (= 1000 kg × g) | — | `bridge::data::N_PER_TONNE` |
| Truss model | linear, small-displacement, pin-jointed; axial springs k = EA/L | Exact for the model | direct stiffness method | `truss` |
| Euler buckling | P_cr = π²EI/(KL)², K = 1.0 (pinned–pinned) | Exact for the model | Euler | `truss::check` |
| Box section | I = A² (thin square box, b/t = 24) | Exact for the model (the geometry) | A ≈ 4bt = b²/6, I ≈ (2/3)b³t = A² | `truss::Member::box_section` |
| Self-weight | A·L·ρ·g lumped half at each end | Simplified | standard lumping | `truss::Truss::self_weight` |
| "Carrying almost nothing" | \|N\| < 500 N | Illustrative | display threshold | `truss::ZERO_FORCE_N` |
| Pivot tolerance | 1e-9 × max \|K_ii\| | Exact (numerical) | the mockups' singular test | `linalg::PIVOT_TOL` |

## Bridge — Playground (`Playground.dc.html` `mats()`, `sizes()`, `levels()`)

Game materials (one material per structure, "Built from"):

| Material | E | fy (tension / crushing limit) | ρ | $/kg | Class | Source / note |
|---|---|---|---|---|---|---|
| Steel | 200 GPa | 250 MPa | 7850 kg/m³ | 3.5 | Representative (E, fy, ρ); Game ($) | [AISC] E; [A36] fy; [EC0-1] ρ |
| Timber | 12 GPa | 30 MPa | 500 kg/m³ | 2.2 | Representative; Game ($) | [FPL] E, ρ; fy between [EN338] C24 characteristic (14.5 tension, 21 compression) and clear-wood values |
| Aluminum | 69 GPa | 240 MPa | 2700 kg/m³ | 8 | Representative; Game ($) | [AA] E, ρ; fy = [EC9] 6061-T6 f0 |
| Carbon fiber | 70 GPa | 600 MPa | 1600 kg/m³ | 40 | Representative; Game ($) | [CMH17] quasi-isotropic laminate (upper end of E) |

`sp_engine::bridge::data` (the `MaterialKey` table).

| Constant | Value | Class | Note |
|---|---|---|---|
| Member sizes | Thin 0.004 m², Medium 0.008 m², Thick 0.016 m² | Game | 40 / 80 / 160 cm² box sections |
| Joint cost | $0.25k per joint | Game | `JOINT_COST_K` |
| Budgets | River crossing $30k, Overhang $14k, Heavy haul $60k, Sandbox none | Game | |
| Stars | 1 / 2 / 3 at cost ≤ 100 % / 75 % / 55 % of budget | Game | |
| Level loads | 20 t at mid-span; 5 t 12 m out; two 25 t | Game | |
| Weight steps | 1, 5, 10, 25 t; 0–200 t per joint | Game | `WEIGHT_STEPS`, `MAX_LOAD_T` |
| Site | 2 m grid, x 0–58 m, y −12…16 m, river 40 m (x 8–48 m), riverbed −12 m | Game | `GRID_STEP`, `LEFT_BANK`, … |
| Test ramp | load factor +1/55 per tick (~30 ticks/s), ≤ 30 breaks per tick | Game | `LAMBDA_STEP`, `MAX_BREAKS_PER_TICK` |
| Collapse sim | Verlet damping 0.996, 14 constraint passes, members tear at 5 % strain, water drag ×0.86 below −8 m, joint mass 60 kg, ground friction 0.3, 3.2 s (slow motion 0.3× for 1.6 s) | Simplified (position-based dynamics) / Game (timings) | `bridge::sim` |
| Sag drawing | displacements × 40 | Drawing | `SAG_FACTOR` |
| Undo depth, toast | 40 steps; 2.6 s | Game | |

## Advanced — the truss netlist (`Advanced.dc.html` `mats()`, `build()`, `solve()`)

| Material | E | fy | ρ | $/kg | Class | Source / note |
|---|---|---|---|---|---|---|
| Steel | 200 GPa | 250 MPa | 7850 kg/m³ | 3.5 | Representative; Game ($) | [AISC], [A36], [EC0-1] |
| Aluminum | 69 GPa | 240 MPa | 2700 kg/m³ | 8 | Representative; Game ($) | [AA], [EC9] |
| Glulam | 12 GPa | 24 MPa | 500 kg/m³ | 2.2 | Representative; Game ($) | fy = [EN14080] GL24h bending strength; E ≈ GL24h 11.5 GPa; ρ above GL24h mean (420) |
| Carbon fiber | 70 GPa | 600 MPa | 1600 kg/m³ | 40 | Representative; Game ($) | [CMH17] |

| Constant | Value | Class | Note |
|---|---|---|---|
| Truss geometry | 48 m span, 8 m panels, 6 m deep; Pratt / Howe / Warren, optional cross-bracing | Game (the example) | `generate` |
| Sections | chord A = 25 cm² × size, web A = 15 cm² × size (size 1–10, default 4); I = A² | Game; Exact (I = A²) | netlist `.section` |
| Deck dead load | 80 kN at each bottom panel point (B1–B5), per truss, plus self-weight | Illustrative | |
| Truck | 0–60 t (default 40 t), half to each truss (`share=0.5`), split linearly between the two nearest panel points | Simplified (lever rule) / Game (weight) | `.load truck` |
| Deflection target | L/800 | Illustrative | road-bridge serviceability is typically L/500–L/1000 |
| Drive across | x 0 → 48 m, 0.5 m per 90 ms | Drawing | |

## Tower (`Tower.dc.html` `defs()`, `calc()`) — `sp_engine::tower`, `sp_engine::dynamics`

| Constant | Value | Class | Source / note |
|---|---|---|---|
| Floor height | 4 m | Representative | typical office storey 3.6–4.2 m (`FLOOR_H`) |
| Plan width B | 40 m square | Game | `PLAN_B` |
| Wind profile | V(z) = V10·(max(z,10)/10)^0.2 | Simplified | [ASCE7] power law, α = 0.2 (suburban) (`ALPHA`) |
| Wind pressure | 0.613·V² Pa (½ρ with ρ = 1.226 kg/m³) | Exact (unit constant) | [ASCE7] (`HALF_RHO`) |
| Drag coefficient Cd | Straight 1.4, Tapered 1.25, Twisted 1.15, Hollow core 1.0 | Illustrative (trend: shaping lowers drag) | [EC1-4] gives ≈ 2.1 for a sharp-edged square, less with rounding and aspect |
| Cross-wind factor | 1.0 / 0.6 / 0.45 / 0.55 | Illustrative | tapering and twisting disrupt vortex shedding |
| Mass factor | 1.0 / 0.75 / 1.0 / 0.92 | Illustrative | |
| Taper | width × (1 − 0.55·z/H) | Game | |
| Stiffness EI | 4.3e13 N·m² × system k (Core 1, Tube 2.2, Outriggers 3, Braced tube 4.2) | Illustrative (calibrated to give realistic periods and drifts) | `EI_CORE` |
| Mass per metre μ | 4e5 kg/m × mass factor | Representative | 40 × 40 m plan at ≈ 250 kg/m³ building density (`MU`) |
| First frequency | 0.5596·√(EI/(μH⁴)) | Exact for the model | [Blevins] uniform cantilever |
| Deflection | Σ F·z²(3H − z)/(6EI) | Exact for the model | cantilever point-load formula, floor by floor |
| Damping ζ | 1.5 %; 5 % with the tuned mass damper | Representative | tall buildings 1–2 %; TMDs add a few % [DenHartog] (`ZETA`, `ZETA_DAMPER`) |
| Damper mass ratio | 2 % of the modal mass | Representative | [DenHartog] (`DAMPER_MASS_RATIO`) |
| Peak acceleration | 0.25·(2πf1)²·δ·√(0.015/ζ), × (1 + 3·near·cross·√(0.015/ζ)) | Illustrative | gust factor and lock-in amplification chosen for trend |
| Vortex shedding | fs = 0.12·V_H/B; lock-in when fs/f1 within ±25 % | Simplified | [EC1-4] St = 0.12 for squares (`STROUHAL`, `LOCK_BAND`) |
| Comfort bands | 5 / 10 / 20 milli-g (bar to 30) | Illustrative | [ISO10137] (`COMFORT_BANDS`, `COMFORT_MAX`) |
| Drift aim / danger | H/500 / H/200 | Representative / Illustrative | serviceability limits are typically H/400–H/600 (`DRIFT_AIM`, `DRIFT_DANGER`) |
| Cost | $1M per floor × system × shape factor, + $6M damper; challenge budget $160M | Game | system ×1, 1.2, 1.3, 1.45; shape ×1, 0.95, 1.18, 1.1 (`DAMPER_COST`, `BUDGET`) |
| Floors, wind, exaggeration | 5–120 floors; V10 5–50 m/s (default 30); sway drawn × 1–150 (default 40) | Game / Drawing | |
| m/s → mph | × 2.237 | Exact (2.23694) | `MPH` |
| Gust playback | modal mass factor 33/140, turbulence 8 %, vortex force 6 %, mean lean 0.85, 48 steps per period, playback 0.55 Hz | Illustrative / Drawing | `dynamics` (owner: `worlds`) |
| Challenge | 100 floors, at V10 = 30 m/s; rung 7 / 12 | Game | |

## Ground (`Ground.dc.html` `defs()`, `calc()`) — `sp_engine::ground`

Soil presets (one uniform layer to bedrock):

| Soil | Allowable bearing qa | Stiffness ks | Variability vr | Shaft friction | Liquefiable | Consolidation τ | Class | Source / note |
|---|---|---|---|---|---|---|---|---|
| Dense sand | 400 kPa | 8 kPa/mm | 0.10 | 25 kPa | no | 0.3 yr | Illustrative | qa inside [BS8004] range; friction [Tomlinson] |
| Stiff clay | 250 kPa | 4 kPa/mm | 0.15 | 18 kPa | no | 3 yr | Illustrative | [BS8004]; [Tomlinson] |
| Soft clay | 60 kPa | 0.6 kPa/mm | 0.50 | 5 kPa | no | 9 yr | Illustrative | [BS8004] (< 75 kPa); [Tomlinson] low end |
| Loose wet sand | 100 kPa | 1.5 kPa/mm | 0.30 | 8 kPa | yes | 0.5 yr | Illustrative | [BS8004] |

| Constant | Value | Class | Note |
|---|---|---|---|
| Building load | 12 kPa per floor over a 40 × 40 m (1600 m²) footprint | Representative | 10–15 kPa per storey dead + live is typical (`FLOOR_LOAD`, `FOOTPRINT_AREA`) |
| Footings | cover 35 % of the plan (pressure ÷ 0.35) | Illustrative | `FOOTING_COVER` |
| Bedrock | at 30 m; piles ≥ 30 m add 900 kPa end bearing; caissons 1800 kPa | Illustrative | `BEDROCK_DEPTH`, `ROCK_END_BEARING`, `CAISSON_CAPACITY` |
| Pile capacity | shaft friction × length (+ rock) | Simplified | [Tomlinson] |
| Liquefaction | capacity × 0.12, settlement ≥ 900 mm, tilt ≥ 9°, τ 0.08 yr | Illustrative | `LIQUEFIED_*` |
| Settlement | q/ks × (1 shallow, 0.5 deep, 0.12 piles to rock, 0.08 caissons) | Simplified | |
| Differential settlement, tilt | settlement × vr (× 0.3 deep); tilt = atan(diff/40 m); + (1.6 − FS)·4.5° (≤ 7°) when FS < 1.6 | Illustrative | |
| Bearing failure | settlement ≥ 400 mm when FS < 1 | Illustrative | `FAILURE_SETTLEMENT` |
| Consolidation in time | 1 − e^(−t/τ) | Simplified | [Terzaghi] |
| Safety factor target | 3 (shallow), 2 (deep); bar to 5 | Representative | common practice for allowable-stress foundation design |
| Pisa reference | "Pisa today ≈ 4°" | Representative | [Burland] |
| Years, floors, piles | 0–30 yr (0.25 yr per 60 ms); 2–80 floors; piles 8–40 m | Game / Drawing | |
| Challenge | tilt under 0.5° at 30 years from the leaning-tower scenario; rung 10 / 12 | Game | `LEAN_TILT`, `CHALLENGE_RUNG` |

## Materials — tensile test (`Materials.dc.html` `defs()`, `tempFactor()`) — `sp_engine::materials::curves`

The five curves are piecewise linear, strain in % and stress in MPa, fracturing past the last
corner. **Class: representative** (badge "Curves: representative"; "Not certified data").

| Material | Curve corners (strain %, MPa) | E | Yield | Peak | Break | ρ | Source / note |
|---|---|---|---|---|---|---|---|
| Steel (mild, S275-like) | (0,0) (0.1375,275) (1.5,275) (5,360) (10,415) (15,430) (19,405) (22,360) | 200 GPa | 275 MPa | 430 MPa at 15 % | 22 % | 7850 | [AISC] E; [EN10025] S275 yield, Rm 410–560, A ≈ 22 %; yield plateau (Lüders) to 1.5 %, uniform elongation ≈ 15 %, then necking |
| Aluminum (6061-T6) | (0,0) (0.4,276) (1,290) (4,305) (8,310) (11,300) (12,290) | 69 GPa | 276 MPa (the 0.4 % corner) | 310 MPa at 8 % | 12 % | 2700 | [AA] typical 276 / 310 MPa, 12 %; no sharp yield point |
| Concrete (≈ C30, in compression) | (0,0) (0.1,30) (0.2,40) (0.3,34) (0.35,28) | 30 GPa | — | 40 MPa at 0.2 % | 0.35 % | 2400 | [EC2] C30/37 fcm 38 MPa, εc1 ≈ 0.22 %, εcu1 0.35 %, Ecm 33 GPa; tension ≈ a tenth (fctm 2.9 MPa) |
| Timber (clear softwood, along the grain) | (0,0) (0.67,80) | 12 GPa | — | 80 MPa | 0.67 % | 500 | [FPL] clear-wood tension parallel to the grain, linear and brittle |
| Carbon fiber (quasi-isotropic laminate) | (0,0) (0.86,600) | 70 GPa | — | 600 MPa | 0.86 % | 1600 | [CMH17] QI carbon/epoxy, linear to failure (E at the upper end; standard-modulus QI is nearer 50 GPa) |

Derived values (all **exact for the model**): toughness = area under the curve (steel 83.72,
aluminium 35.58, concrete 0.10, timber 0.27, carbon fibre 2.58 MJ/m³); specific strength = peak
÷ density (55, 115, 17, 160, 375 kN·m/kg at 20 °C); load cell force = stress × 50 mm².

Temperature strength factors (× every stress; linear between the points, flat outside).
**Class: illustrative** (badge "Heat effects: illustrative").

| Material | Points (°C → factor) | Source / note |
|---|---|---|
| Steel | ≤ 300 → 1; 600 → 0.47 | end point = [EC3-1-2] k_y at 600 °C; the code holds full yield to 400 °C |
| Aluminum | ≤ 100 → 1; 300 → 0.30; 400 → 0.08 | [EC9] (EN 1999-1-2) trend for 6xxx-T6 |
| Concrete | ≤ 200 → 1; 600 → 0.45 | end point = [EC2-1-2] siliceous concrete at 600 °C |
| Timber | ≤ 100 → 1; 200 → 0.5; 300 → 0 | [EC5-1-2]: charred at the 300 °C isotherm |
| Carbon fiber | ≤ 100 → 1; 200 → 0.25 | [CMH17]: the epoxy softens above its glass transition |

Bench constants:

| Constant | Value | Class | Where |
|---|---|---|---|
| Gauge section | 50 mm² | Game (a typical flat tensile coupon is 20–80 mm²) | `curves::SECTION_MM2` |
| Temperature slider | −40…600 °C, step 10; room temperature 20 °C; "hot" from 200 °C | Game | `curves::TEMP_*`, `HOT_C` |
| Necking | gauge half-width × (1 − n), n rising linearly to 0.55 from the ultimate strain to fracture (metals only) | Drawing | `curves::NECK_MAX` |
| Strain axes | 0–25 % and 0–2 %; stress axis 0–700 MPa | Drawing | `bench::Zoom`, `curves::STRESS_AXIS_MAX_MPA` |
| Pull | 0.08 % per tick (25 % axis) / 0.006 % (2 % axis), 20 ticks/s | Game | `bench::Zoom::pull_step`, `bench::TICK_SECONDS` |
| Drawn stretch | 4.5 px per % strain ("stretch exaggerated") | Drawing | `view::RIG_PX_PER_PERCENT` |
| Challenge | Break all five; rung 5 / 12 | Game | `bench::CHALLENGE_RUNG` |

## Known inconsistencies (documented, not changed)

- **Steel and aluminium strength differ between worlds.** The Bridge and Advanced use
  A36-like steel (fy 250 MPa) and the 6061-T6 *characteristic* proof strength (240 MPa); the
  Materials bench uses S275-like steel (yield 275 MPa) and the 6061-T6 *typical* yield (276
  MPa). Both are representative for their grade; they are different grades.
- **Timber**: the Bridge's 30 MPa and Advanced's 24 MPa (GL24h) are structural design-class
  strengths; the Materials bench's 80 MPa is clear, defect-free wood along the grain.
- **Toughness at temperature**: the Materials toughness tile shows the room-temperature area
  (as the mockup does) while the plotted curve is scaled by the temperature factor (raised in
  `PROGRESS.md` › Requests).
