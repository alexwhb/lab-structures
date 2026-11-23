# Data sources and provenance

SPEC **PROV-2**: every constant the app uses, its provenance class and, for representative
values, a public reference. Structures Playground is **educational, not for real structural
design**: nothing here is certified data, and no value may be used to size a real structure.

All values were typed in by hand from the design mockups (`docs/design/v2/mockup/Builder.dc.html`
for the v2 builder, `docs/design/mockup/*.dc.html` for Advanced and the retired v1 screens),
which in turn use standard textbook and design-code figures. The v2 fold-ins (fire, the sample
drawer) add published reduction factors (Eurocodes) and a few documented assumptions. **No database is bundled or
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

Badges on screen (PROV-1, v2): the builder's one provenance line — "Physics: simplified dynamic
settling" (sand) · "Materials: representative" (sand) · "Soil, weather, fire: illustrative"
(grey) · "Game costs" (grey); Advanced "Solver: linear statics, exact" · "Materials:
representative" · "Game costs" and the netlist's `; representative` comment. (v1, retired: Bridge,
Tower, Ground and Materials had their own badges.)

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
- **[EC3-1-2]** EN 1993-1-2, *Steel — structural fire design*, Table 3.1 (carbon steel),
  reduction factors at θ = 20, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1100, 1200 °C:
  effective yield strength k_y,θ = 1.00, 1.00, 1.00, 1.00, 1.00, 0.78, **0.47**, 0.23, 0.11,
  0.06, 0.04, 0.02, 0; slope of the linear elastic range k_E,θ = 1.00, 1.00, 0.90, 0.80, 0.70,
  0.60, 0.31, 0.13, 0.09, 0.0675, 0.045, 0.0225, 0. "For intermediate values of the steel
  temperature, linear interpolation may be used." Unprotected steel members reach 500–700 °C
  in roughly 10–20 minutes of a standard fire.
- **[AA]** Aluminum Association, *Aluminum Standards and Data* / ASM Handbook Vol. 2: 6061-T6
  typical yield 276 MPa (40 ksi), tensile 310 MPa (45 ksi), elongation ≈ 12 %, E 68.9 GPa,
  ρ 2700 kg/m³.
- **[EC9]** EN 1999-1-1, *Aluminium structures*: 6061-T6 extrusions f0 = 240 MPa (0.2 % proof,
  characteristic), E = 70 GPa. EN 1999-1-2 (fire): 6xxx-T6 strength falls to roughly a third by
  300 °C and to around a tenth by 350–400 °C.
- **[EC2]** EN 1992-1-1, Table 3.1: C30/37 — fck 30 MPa, fcm 38 MPa, Ecm 33 GPa, strain at peak
  εc1 ≈ 0.22 %, ultimate εcu1 = 0.35 %, mean tensile strength fctm 2.9 MPa (about a tenth of
  the compressive strength).
- **[EC2-1-2]** EN 1992-1-2, *Concrete — structural fire design*, Table 3.1, normal-weight
  concrete with siliceous aggregate, at θ = 20, 100, 200, …, 1200 °C: f_c,θ / f_ck = 1.00, 1.00,
  0.95, 0.85, 0.75, 0.60, **0.45**, 0.30, 0.15, 0.08, 0.04, 0.01, 0; strain at peak stress
  ε_c1,θ = 0.0025, 0.0040, 0.0055, 0.0070, 0.0100, 0.0150, 0.0250 (and 0.0250 to 1100 °C).
  §3.2.2.2: tensile strength k_c,t(θ) = 1.0 up to 100 °C, falling linearly to 0 at 600 °C.
- **[FPL]** USDA Forest Products Laboratory, *Wood Handbook* (FPL-GTR-190, 2010), ch. 5:
  clear softwoods along the grain — E ≈ 8–14 GPa, density ≈ 350–550 kg/m³, tensile strength
  parallel to the grain ≈ 60–140 MPa, brittle in tension.
- **[EN338]** EN 338, *Structural timber — strength classes*: C24 — f_m,k 24 MPa, f_t,0,k 14.5,
  f_c,0,k 21, E0,mean 11 GPa, ρ_mean 420 kg/m³.
- **[EN14080]** EN 14080, *Glued laminated timber*: GL24h — f_m,k 24 MPa, E0,mean 11.5 GPa,
  ρ_mean 420 kg/m³.
- **[EC5-1-2]** EN 1995-1-2, *Timber — structural fire design*: the char line is the 300 °C
  isotherm (§3.4.2); strength and stiffness fall in the heated zone below it (Annex B:
  pyrolysis starts around 100–200 °C).
- **[EN10138]** EN 10138 / prEN 10138, *Prestressing steels* (cold-drawn wire and strand): 0.1 %
  proof stress about 0.88 of the tensile strength (grades 1570–1860 MPa), elongation at maximum
  force ≥ 3.5 %. **[EC3-1-11]** EN 1993-1-11, *Tension components*: f_uk 1570 / 1770 MPa wire,
  E ≈ 150–165 GPa for spiral strand.
- **[Fibre]** Polymer handbooks / rope makers' data: nylon 6,6 melts at about 255–265 °C, nylon 6
  at about 220 °C, polyester (PET) at about 250–260 °C, polypropylene at about 165 °C; fibre
  ropes lose strength well before they melt.
- **[ISO834]** ISO 834-1 standard fire curve: gas temperature 20 + 345·log10(8t + 1) °C (t in
  minutes) — about 840 °C at 30 min, 945 °C at 60 min; the hydrocarbon curve of EN 1991-1-2
  reaches about 1100 °C.
- **[EC1-3]** EN 1991-1-3, *Snow loads*: characteristic ground snow loads of roughly 0.5–2 kN/m²
  in most of Europe (≈ 50–200 kg/m²), much more in the mountains.
- **[Vehicles]** Typical masses: cars 1.2–2 t, city buses 11–18 t laden, three-axle trucks
  26–32 t (EU gross weight limit 26 t for a rigid 3-axle truck, 40–44 t articulated).
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

## Shared

| Constant | Value | Class | Source / note | Where |
|---|---|---|---|---|
| Gravity g | 9.81 m/s² | Exact (rounded standard gravity 9.80665) | SI | `truss::G`, `tower::G` |
| Tonne-force | 9 810 N per t | Exact (= 1000 kg × g) | — | the builder's `N / 9810`, `builder::sample::curve::N_PER_TONNE`; v1 `bridge::data::N_PER_TONNE` |
| Truss model | linear, small-displacement, pin-jointed; axial springs k = EA/L | Exact for the model | direct stiffness method | `truss` |
| Euler buckling | P_cr = π²EI/(KL)², K = 1.0 (pinned–pinned) | Exact for the model | Euler | `truss::check` |
| Box section | I = A² (thin square box, b/t = 24) | Exact for the model (the geometry) | A ≈ 4bt = b²/6, I ≈ (2/3)b³t = A² | `truss::Member::box_section` |
| Self-weight | A·L·ρ·g lumped half at each end | Simplified | standard lumping | `truss::Truss::self_weight` |
| "Carrying almost nothing" | \|N\| < 500 N | Illustrative | display threshold | `truss::ZERO_FORCE_N` |
| Pivot tolerance | 1e-9 × max \|K_ii\| | Exact (numerical) | the mockups' singular test | `linalg::PIVOT_TOL` |

## Builder — Playground v2 (`Builder.dc.html` `types()`, `soils()`, `vehicles()`, conditions, `levels()`) — `sp_engine::builder`

Owner of the tables: `engine` (`builder::data`); documented here by `lab`.

### Piece types (`types()`, `builder::data::PieceType`)

Every piece is one pin-jointed member: area A, Young's modulus E, buckling factor k (I = k·A²),
tension and compression strength. Capacities: tension A·fT; compression min(A·fC, π²·E·k·A²/L²)
(Euler, pinned–pinned) — **exact for the model**.

| Type | A | E | k | fT | fC | Mass | $/m | Max | Flags | Class | Source / note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Road (river only) | 0.02 m² | 30 GPa | 2 | 25 MPa | 40 MPa | 600 kg/m | 300 | 10 m | deck | Illustrative (fT, mass); Representative (E, fC); Game ($) | a reinforced-concrete deck strip as one member: E [EC2] 30–33 GPa, fC ≈ C40 f_ck / C30 f_cm; fT is an effective reinforced value (plain concrete ≈ 3 MPa); mass is the slab's, not A·ρ |
| Floor (open ground only) | 0.02 m² | 30 GPa | 2 | 25 MPa | 40 MPa | 350 kg/m | 100 | 10 m | deck | as Road | a lighter floor slab |
| Wood | 0.02 m² | 12 GPa | 1 | 30 MPa | 30 MPa | ρ 550 kg/m³ | 90 | 10 m | — | Representative (E, ρ); Illustrative (f, k); Game ($) | [FPL] E 8–14 GPa, ρ 350–550; f between [EN338] C24 characteristic and clear wood (as v1's Bridge timber); k = 1 is a thin box — solid timber would be k = 1/12, so builder wood buckles less than a solid post |
| Steel | 0.006 m² | 200 GPa | 1.5 | 250 MPa | 250 MPa | ρ 7850 kg/m³ | 260 | 14 m | ductile | Representative (E, f, ρ); Illustrative (k); Game ($) | [AISC] E; [A36] fy; [EC0-1] ρ; k = 1.5 is a thin box with b/t ≈ 36 |
| Concrete | 0.06 m² | 30 GPa | 1 | 2.5 MPa | 40 MPa | ρ 2400 kg/m³ | 110 | 8 m | — | Representative; Game ($) | [EC2] C30/37 fctm 2.9 MPa, Ecm 33 GPa; [EC0-1] ρ |
| Rope | 0.0008 m² | 2 GPa | 1 | 100 MPa | 0 | ρ 1100 kg/m³ | 25 | 40 m | tension only | Illustrative; Game ($) | a ≈ 32 mm synthetic fibre rope, deliberately weak (8 t; a polyester rope that size breaks at ≈ 15–20 t); ρ ≈ nylon 1140 |
| Cable | 0.003 m² | 160 GPa | 1 | 1500 MPa | 0 | ρ 7850 kg/m³ | 180 | 40 m | tension only, ductile | Representative; Game ($) | [EC3-1-11] spiral strand E ≈ 150–165 GPa, wire f_uk 1570–1770 MPa |

Display: forces in tonnes are N / 9810 (exact, 1000 kg × g); "Doing almost nothing" below 500 N
(illustrative); utilisation bands < 35 % / < 70 % / ≥ 70 % (illustrative).

### Soils and foundations (`soils()`, `builder::data::Soil`)

`[push capacity, pull capacity, stiffness]` per foundation; a footing's push capacity is about
the soil's presumptive bearing pressure on a 2 × 2 m pad. **Class: illustrative** (badge "Soil,
weather, fire: illustrative").

| Soil | Footing (push / pull / k) | Pile (push / pull / k) | Liquefies | Source / note |
|---|---|---|---|---|
| Rock | 30 MN / 3 MN / 5 GN/m | 40 MN / 20 MN / 8 GN/m | no | 7.5 MPa on 4 m² |
| Dense sand | 1.6 MN / 0.1 MN / 20 MN/m | 6 MN / 3 MN / 100 MN/m | no | [BS8004] 400 kPa × 4 m² |
| Stiff clay | 1 MN / 0.15 MN / 10 MN/m | 4 MN / 2.5 MN / 60 MN/m | no | [BS8004] 250 kPa × 4 m² |
| Soft clay | 0.25 MN / 0.05 MN / 1.5 MN/m | 1.6 MN / 1 MN / 20 MN/m | no | [BS8004] < 75 kPa (60 × 4 m²); [Tomlinson] low shaft friction |
| Loose wet sand | 0.5 MN / 0.05 MN / 4 MN/m | 2.5 MN / 1.5 MN / 30 MN/m | yes | [BS8004] < 100 kPa |

| Constant | Value | Class | Note |
|---|---|---|---|
| Foundation costs | Footing $0.3k, Pile $1.5k (level anchors free) | Game | |
| Pile drawing depth | 14 m | Drawing | |
| Foundation mass | +800 kg at the joint | Illustrative | |
| Liquefaction (shaking > 20 %) | footing capacity × 0.15, pile × 0.6, stiffness × 0.3 | Illustrative | piles reach firmer layers |
| Sinking | past capacity the spring's rest level creeps down by (R − Q)/k × 0.02 per substep (after 0.5 s); "sank" past 0.3 m | Illustrative | |
| Ripping out | pulled past the pull capacity (after 0.5 s) | Illustrative | |
| Reaction smoothing | R ← 0.97·R + 0.03·R_step | Simplified | |

### Vehicles and traffic (`vehicles()`, `trafficSets()`)

| Vehicle | Mass | Length | Height | Axle half-spacing | Speed | Class | Source |
|---|---|---|---|---|---|---|---|
| Car | 1.5 t | 4.4 m | 1.4 m | 1.4 m | 10 m/s | Representative (mass, size); Game (speed) | [Vehicles] |
| Bus | 12 t | 11 m | 3.0 m | 3.8 m | 8 m/s | as Car | [Vehicles] |
| Truck | 30 t | 12 m | 3.4 m | 4.6 m | 7 m/s | as Car | [Vehicles] |

Traffic sets: Level traffic (the level's), Rush hour (8 cars), Buses (3), Trucks (2), Convoy
(car, bus, truck, car, truck) — Game. Vehicles start 12 m apart, from x = 2 m going left; each
axle carries half the weight, split between the two joints of the Road piece under it
(Simplified, lever rule). A river run ends 1.2 s after the last vehicle is done, or at 45 s;
Sandbox vehicles loop from x = 70 m back to −20 m (Game).

### Conditions (`winds()`, `snows()`, `quakes()`)

| Constant | Value | Class | Source / note |
|---|---|---|---|
| Wind levels | Calm 0, Breezy 15, Strong 25, Storm 35, Hurricane 50 m/s (at 10 m) | Illustrative | Beaufort 7 / 10 / 12+ territory |
| Wind profile | v(y) = V·(max(y, 4)/10)^0.2 | Simplified | [ASCE7] power law α = 0.2 |
| Floor wind force | 0.613·v²·1.3·4·4 × gust on each windward floor-edge joint (Cd 1.3 on a 4 × 4 m tributary area) | Simplified (q = 0.613 V² exact; Cd illustrative) | [ASCE7], [EC1-4] |
| Gust | 1 + 0.3·sin(2π·f_s·t) + 0.15·sin(2π·0.37·t) + 0.1·sin(2π·0.13·t + 1), f_s = 0.12·V_top/width | Illustrative (St 0.12 from [EC1-4]) | `Sim::shedding` scales the 0.3 term (CMF-2) |
| Wind easing | calm for 1 s, then eases toward the setting at 0.5 per second | Game | |
| Deck flutter | lift q10·0.25·10·L·sin(2π·0.45·t + 0.35·x)/2 on Road pieces | Illustrative | |
| Snow levels | None 0, Light 150, Heavy 400, Blizzard 800 kg per metre of exposed deck | Illustrative | [EC1-3] 0.5–2 kN/m² on a ≈ 3–4 m wide deck |
| Snow build-up | at most 90 kg/m per second toward the setting | Game | |
| Earthquake levels | Off 0, Moderate 0.15, Strong 0.3, Huge 0.5 "g" | Illustrative | see "Known inconsistencies": the setting is the ground displacement amplitude in metres |
| Quake motion | supports moved by setting × envelope × sin(2π·1.1·t); envelope 2 s up, 5 s, 2 s down; starts 3 s in, 0.5 s after a change, 0.2 s after "Shake now" | Illustrative / Game | |
| Fire levels | Off, Small 400 °C, Big 700 °C, Inferno 900 °C | Illustrative | see § Fire |

### The settling engine (`makeSim`, `step`, `preview`) — badge "Physics: simplified dynamic settling"

| Constant | Value | Class | Note |
|---|---|---|---|
| Model | XPBD distance constraints, explicit integration, gravity on joint and member masses | Simplified | position-based dynamics |
| Piece stiffness | E/15 · A / L | Simplified | a fifteenth of the real axial stiffness, so settling is visible and the solve converges |
| Joint mass | 40 kg + crates (t × 1000 kg) + half of each member's mass | Illustrative | |
| Running | 20 substeps per 1/30 s frame (× 0.3 in slow motion), damping 1 − 0.4·h | Game | |
| Gravity ramp | over 0.8 s (running), 0.3 s (preview) | Game | |
| Preview | 520 steps at h = 1/400 s, damping 0.985 | Game | |
| Breaking | smoothed force N̄ ← 0.96·N̄ + 0.04·N past capacity, after 0.5 s | Simplified | |
| "Sags and folds" | a free joint drops more than 1.5 m in the preview | Illustrative | |
| Water drag | velocities × 0.97 below −8 m | Illustrative | |
| Open-ground test | 18 s; passes when the top floor stays above 80 % of its height, reached the target and stayed under the comfort limit | Game | |
| Comfort | peak smoothed top-floor acceleration (0.6/0.4 smoothing per frame, after 2 s), milli-g | Simplified | |

### Levels, costs and stars (`levels()`, `cost()`)

| Level | Scene / soil | Conditions | Budget | Goal | Class |
|---|---|---|---|---|---|
| 1 · First crossing | river, dense sand (bed soft clay) | — | $22k | three cars | Game |
| 2 · Bus route | river, dense sand | — | $30k | car, bus, car; piles set back | Game |
| 3 · Heavy haul | river, dense sand (bed soft clay, piers allowed) | wind 15, snow 400 | $45k | car, 30 t truck | Game |
| 4 · First tower | open ground, dense sand | wind 15 | $70k | 40 m, comfort 100 milli-g | Game |
| 5 · Storm tower | open ground, stiff clay | wind 35 | $260k | 64 m, comfort 300 milli-g | Game |
| 6 · Shaky ground | open ground, loose wet sand | quake 0.3 | $65k | 32 m | Game |
| Sandbox | river or open ground | any | none | — | Game |

Cost = Σ length × $/m + foundations (Game). Stars 1 / 2 / 3 at cost ≤ 100 % / 80 % / 60 % of
the budget; none for a practice run (any condition easier than the level's) — Game. Grid 2 m
(river) / 4 m (open ground), halved at ≥ 250 % zoom, doubled below 60 %; 18 / 5 px per metre at
100 %; zoom 35–500 %; undo 40 steps; toasts 2.6 s — Drawing / Game. The comfort limits are game
numbers on the mockup's scale, far above real comfort criteria ([ISO10137] 5–15 milli-g; see the
comfort rows below).

### Comfort and settlement fold-ins (CMF, TIME; agent `time`)

From SPEC CMF-1 / CMF-2 / TIME-1 and `time`'s notes in PROGRESS; `time` owns these constants
(`builder::comfort`, `builder::longterm`) and may refine the rows.

| Constant | Value | Class | Note |
|---|---|---|---|
| Comfort bands | v1's 5 / 10 / 20 milli-g × 15 for the builder's game scale (`comfort::GAME_SCALE`): 75 / 150 / 300 mg, meter 0–450+ | Illustrative | [ISO10137] for the real-world bands; the ×15 maps the mockup's comfort number onto them |
| Same storm, four shapes | Tapered: each joint's x drawn toward the centre line in proportion to height, top 60 % as wide; Twisted: drag × 1.15/1.3, shedding × 0.45; Hollow core: drag × 1.0/1.3, shedding × 0.55 | Illustrative | factors from v1's tower model (`sp_engine::tower` Cd and cross-wind factors) |
| Consolidation time τ | dense sand 0.3 yr, stiff clay 3 yr, soft clay 9 yr, loose wet sand 0.5 yr, rock none | Illustrative | as v1's Ground; [Terzaghi] first-term 1 − e^(−t/τ) |
| Toppling | a tower leaning past 5° | Game | |

## Fire (FIRE-1) — `sp_engine::builder::fire` (owner `lab`)

Published reduction factors applied to illustrative heating. Factors are interpolated linearly
between the table points (as both Eurocodes allow) and held flat below 20 °C and past the last
point.

| Material | Strength | Stiffness | Class | Source |
|---|---|---|---|---|
| Steel, Cable | k_y,θ: 1.0 to 400 °C, 0.78 at 500, 0.47 at 600, 0.23 at 700, 0.11 at 800, 0.06 at 900, 0.04 at 1000, 0.02 at 1100, 0 at 1200 | k_E,θ: 1.0 to 100 °C, 0.9 at 200, 0.8 at 300, 0.7 at 400, 0.6 at 500, 0.31 at 600, 0.13 at 700, 0.09 at 800, 0.0675 at 900, 0.045 at 1000, 0.0225 at 1100, 0 at 1200 | Representative (published) | [EC3-1-2] Table 3.1, carbon steel. Cable follows the same table (SPEC FIRE-1); real cold-drawn wire loses strength sooner (EN 1992-1-2 Table 3.3) — simplified |
| Concrete, Road, Floor | compression f_c,θ/f_ck: 1.0 to 100 °C, 0.95 at 200, 0.85 at 300, 0.75 at 400, 0.60 at 500, 0.45 at 600, 0.30 at 700, 0.15 at 800, 0.08 at 900, 0.04 at 1000, 0.01 at 1100, 0 at 1200 (tension k_c,t: 1.0 to 100 °C, 0 at 600 — tabulated, not applied: see below) | secant modulus to the peak, (f_c,θ/ε_c1,θ)/(f_ck/ε_c1,20): 1.0, 0.625 at 100, 0.43 at 200, 0.30 at 300, 0.19 at 400, 0.10 at 500, 0.045 at 600, 0.03 at 700, … | Representative (strength, published); Simplified (stiffness, derived) | [EC2-1-2] Table 3.1, siliceous aggregate; the stiffness includes the code's transient creep |
| Wood | 1.0 to 100 °C, falling linearly to 0 at 300 °C | the same | Illustrative | [EC5-1-2]: the char line is the 300 °C isotherm; as v1's Materials timber |
| Rope | 1.0 to 100 °C, falling linearly to 0 at 250 °C | the same | Illustrative | [Fibre]: nylon / polyester melt at ≈ 220–265 °C |

| Constant | Value | Class | Note |
|---|---|---|---|
| One strength factor per piece | the builder scales A·fT and A·fC by one factor (`Constraint::strength`): the steel / wood / rope factor, and for concrete, Road and Floor the compressive one | Simplified | concrete pieces work in compression; the sample drawer uses the same factor so its "breaks at" equals the builder's |
| Fire temperatures | Small 400 °C, Big 700 °C, Inferno 900 °C | Illustrative | [ISO834]: a standard fire passes 700 °C after ≈ 12 min and 900 °C after ≈ 45 min |
| Reach (fully grown) | 8 / 12 / 16 m of plume distance (interpolated for other temperatures) | Illustrative | |
| Where | river: on the water under mid-span (28, −8 m); open ground: the mean x of the founded joints at y = 0 (else the joints at ground level, else x = 100 m) | Game | |
| Timing | lights 3 s into a run (as the quake), 0.5 s after Fire is switched on mid-run, 0.2 s after "Light it now"; grows over 5 s (temperature and reach); Off puts it out | Game | |
| Plume distance | only what is at or above the fire's height heats; height above it counts × 0.5 (flames and hot gas rise); full fire temperature within half the reach, falling linearly to ambient at the reach | Illustrative | |
| Heating lag | first-order, time constant steel / cable 3 s, wood 4 s, rope 2 s, concrete 8 s | Illustrative (time compressed: real members take minutes; [EC3-1-2]) | thin steel heats fastest, thick concrete slowest |
| Recovery | steel and cable follow their temperature now (they regain strength as they cool); concrete, wood and rope keep the worst they reached | Simplified | steel regains most of its strength after cooling; charring, melting and cracking don't heal |
| "Softened" / "burned through" | a piece that breaks with less than 90 % of its strength left (steel, cable / wood, rope) | Game | |
| HUD "Fire 650 °C" | the fire's temperature now | Exact for the model | |

## Sample curves (LAB-1) — `sp_engine::builder::sample` (owner `lab`)

The sample drawer turns each piece type's builder properties into a stress–strain curve. The
straight line (slope E × the fire's k_E) ends exactly at the builder's capacity ÷ area, so "a
piece this size breaks at N t" is the engine's number (**exact for the model**); the shapes
after it are assumptions:

| Type | Pull after the straight line | Push (1 m sample unless chosen) | Class | Source / note |
|---|---|---|---|---|
| Steel | plateau to 1.5 %, hardening to 1.31 / 1.51 / 1.56 × yield at 5 / 10 / 15 %, necking down to 1.47 / 1.31 × yield at 19 / 22 %, then it tears | gives way at min(A·fC, Euler); squashes on a plateau for 1.875 % more strain, or bows if it buckles | Representative (shape) | v1's S275 curve scaled to the builder's 250 MPa ([EN10025]); in the builder a steel piece counts as broken at yield |
| Cable | plateau to 1.2 %, 1.10 × at 2.5 %, 1.136 × (1/0.88) at 3.5 %, 1.08 × at 4.5 %, then it tears | — (only pulls) | Representative (shape) | [EN10138] proof ≈ 0.88 × tensile, elongation at max force ≥ 3.5 % |
| Concrete, Road, Floor | cracks at fT | crushes: 0.85 × peak 0.117 % past the peak, 0.70 × at 0.217 % past (0.25 / 0.35 % at 20 °C), then crushed | Representative | [EC2] ε_cu1 = 0.35 % |
| Wood | snaps at fT | 0.9 × peak 0.55 % past the peak, 0.75 × at 0.95 % past, then crushed | Illustrative | fibres kink and fold in compression |
| Rope | snaps at fT (5 % at 20 °C) | — (only pulls) | Illustrative | real fibre rope stretches 15–25 % on a curved line |
| Buckling (any type that pushes) | — | at σ_cr = π²·E·k·A/L² (× k_E), then 0.85 × σ_cr at twice the strain | Exact (σ_cr) / Illustrative (tail) | Euler, pinned–pinned |

| Constant | Value | Class | Note |
|---|---|---|---|
| At temperature | E × k_E,θ; every stress × the builder's strength factor; plastic strains keep their offsets past the (shifted) yield strain | Simplified | [EC3-1-2] Fig. 3.1 has its own elevated-temperature curve shape |
| Sample length (Push) | 1 m default, whole metres up to the type's longest piece | Game | |
| Temperature slider | −40…900 °C, step 10; factors 1.0 below 20 °C | Game | SPEC LAB-1 |
| Stepping | 20 steps/s; the straight line in 12 steps, the rest of the axis in 160 | Game | |
| Strain axis | the 1 / 2 / 2.5 / 5 × 10ⁿ at or above 1.1 × the failure strain; stress axis the same above 1.15 × the peak (room-temperature curve included) | Drawing | |
| Drawn travel | 110 px at the axis end ("stretch exaggerated") | Drawing | |
| Necking | gauge half-width × (1 − n), n rising to 0.55 from the peak strain to fracture (ductile pull) | Drawing | as v1 |

## Advanced — the truss netlist (`Advanced.dc.html` `mats()`, `build()`, `solve()`; unchanged in v2)

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

## Retired v1 screens

The v1 Bridge, Tower, Ground and Materials screens are retired in v2 (git history before
`2234b1f`). Their rows stay for the engines MCP still runs: `tower_wind` (`sp_engine::tower`,
`dynamics`), `ground_check` (`sp_engine::ground`; the soils' τ also feed TIME-1) and
`tensile_test` (`sp_engine::materials`). The v1 Bridge engine (`sp_engine::bridge`) is removed
in v2 wave 2; its rows are kept for reference only.

### Bridge — Playground v1 (`Playground.dc.html` `mats()`, `sizes()`, `levels()`) — **retired**, engine removed in v2 wave 2

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

### Tower v1 (`Tower.dc.html` `defs()`, `calc()`) — `sp_engine::tower`, `sp_engine::dynamics` — **screen retired**; engine kept for MCP `tower_wind` and CMF-2's factors

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
| Gust playback | modal mass factor 33/140 (exact for the tip-load mode shape), turbulence 2 %, vortex force 1.5 % of the mean force at full lock-in, gust correlation time 1 period, mean lean 0.85, 48 steps per period, playback 0.55 Hz | Illustrative (calibrated so 3σ of the swing ≈ the mockup's dynamic amplitude) / Drawing | `tower::{TURBULENCE, VORTEX_FORCE, MEAN_LEAN, MODAL_MASS_FACTOR}`, `dynamics` (owner: `worlds`) |
| Challenge | 100 floors at V10 = 30 m/s, drift better than H/350 (SPEC MB-6; the tile keeps "aim H/500"), under 10 milli-g, under $160M; rung 7 / 12 | Game | `CHALLENGE_FLOORS`, `CHALLENGE_V10`, `DRIFT_GOAL`, `CHALLENGE_RUNG` |

### Ground v1 (`Ground.dc.html` `defs()`, `calc()`) — `sp_engine::ground` — **screen retired**; engine kept for MCP `ground_check` and TIME-1's τ

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
| Tower → Ground note | the tower's overturning moment adds ± M / (B³/6) at the foundation's edges (40 m square footprint: section modulus 10 667 m³; 248 kPa for the default 60-floor tower) | Simplified (rigid base, linear pressure) | `FromTower::edge_pressure` |
| Years, floors, piles | 0–30 yr (0.25 yr per 60 ms); 2–80 floors; piles 8–40 m | Game / Drawing | |
| Challenge | tilt under 0.5° at 30 years from the leaning-tower scenario; rung 10 / 12 | Game | `LEAN_TILT`, `CHALLENGE_RUNG` |

### Materials v1 — tensile test (`Materials.dc.html` `defs()`, `tempFactor()`) — `sp_engine::materials::curves` — **screen retired**; engine kept for MCP `tensile_test` (the v2 sample drawer has its own curves, § Sample curves)

The five curves are piecewise linear, strain in % and stress in MPa, fracturing past the last
corner. **Class: representative** (badge "Curves: representative"; "Not certified data").

| Material | Curve corners (strain %, MPa) | E | Yield | Peak | Break | ρ | Source / note |
|---|---|---|---|---|---|---|---|
| Steel (mild, S275-like) | (0,0) (0.1375,275) (1.5,275) (5,360) (10,415) (15,430) (19,405) (22,360) | 200 GPa | 275 MPa | 430 MPa at 15 % | 22 % | 7850 | [AISC] E; [EN10025] S275 yield, Rm 410–560, A ≈ 22 %; yield plateau (Lüders) to 1.5 %, uniform elongation ≈ 15 %, then necking |
| Aluminum (6061-T6) | (0,0) (0.4,276) (1,290) (4,305) (8,310) (11,300) (12,290) | 69 GPa | 276 MPa (the 0.4 % corner) | 310 MPa at 8 % | 12 % | 2700 | [AA] typical 276 / 310 MPa, 12 %; no sharp yield point |
| Concrete (≈ C30, in compression) | (0,0) (0.1,30) (0.2,40) (0.3,34) (0.35,28) | 30 GPa | — | 40 MPa at 0.2 % | 0.35 % | 2400 | [EC2] C30/37 fcm 38 MPa, εc1 ≈ 0.22 %, εcu1 0.35 %, Ecm 33 GPa; tension ≈ a tenth (fctm 2.9 MPa) |
| Timber (clear softwood, along the grain) | (0,0) (0.67,80) | 12 GPa | — | 80 MPa | 0.67 % | 500 | [FPL] clear-wood tension parallel to the grain, linear and brittle |
| Carbon fiber (quasi-isotropic laminate) | (0,0) (0.86,600) | 70 GPa | — | 600 MPa | 0.86 % | 1600 | [CMH17] QI carbon/epoxy, linear to failure (E at the upper end; standard-modulus QI is nearer 50 GPa) |

Derived values (all **exact for the model**): toughness = area under the curve as drawn, i.e.
heat-scaled (at 20 °C: steel 83.72, aluminium 35.58, concrete 0.10, timber 0.27, carbon fibre
2.58 MJ/m³; SPEC MB-8); specific strength = peak ÷ density (55, 115, 17, 160, 375 kN·m/kg at
20 °C); load cell force = stress × 50 mm².

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
- **Toughness at temperature** (resolved, SPEC MB-8): the mockup's toughness tile showed the
  room-temperature area while its curve was heat-scaled; the app shows the area under the drawn
  (heat-scaled) curve.
- **The Tower's 100-floor challenge** (SPEC MB-6, resolved by `worlds` in wave 2): with the
  mockup's model no 100-floor tower reaches H/500 at 30 m/s (best: a braced tapered tube ≈
  H/367), so the mockup's goal could never complete; the goal reads "Drift better than H/350 at
  30 m/s" while the drift tile keeps "aim H/500" (serviceability practice, H/400–H/600).
- **The builder's earthquake "g"** (v2, from the mockup): the Conditions chips say Moderate 0.15
  / Strong 0.3 / Huge 0.5 g, but the engine moves the supports by setting × envelope ×
  sin(2π·1.1·t) **metres**, so the peak ground acceleration is setting × (2π·1.1)² ≈ 4.9 ×
  setting in g (Strong ≈ 1.5 g). Illustrative; kept as the mockup (`engine` owns it).
- **Builder steel vs the v1 bench**: the builder's Steel is A36-like (250 MPa, like v1's Bridge and
  Advanced); the sample drawer draws v1's S275 curve shape scaled to the builder's 250 MPa, so
  the drawer's "breaks at" is the builder's yield, and its hardening above it is the shape's.
- **Concrete in tension in a fire**: EN 1992-1-2 §3.2.2.2 takes tensile strength to 0 at 600 °C,
  faster than the compressive table; the builder applies one strength factor per piece (the
  compressive one) to both directions (simplified; § Fire).
- **Cable in a fire** follows the steel table (SPEC FIRE-1); cold-drawn wire really loses its
  strength sooner (EN 1992-1-2 Table 3.3: about 0.5 at 400 °C).
- **Comfort scale**: the builder's comfort number is the mockup's game scale (level 5 passes at
  272 milli-g, limit 300); real comfort criteria are 5–15 milli-g ([ISO10137]). The CMF-1 meter
  maps v1's bands × 15 (`time`'s decision, pending the lead).
