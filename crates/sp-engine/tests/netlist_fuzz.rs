//! No panics on any netlist (SPEC ENG-3): seeded random mutations of real netlists, garbage,
//! NaN / huge numbers, huge counts, zero-length members, empty structures. Whatever parses is
//! also analysed and swept (report + influence line) without panicking.

use sp_engine::generate::{self, MaterialKind, Params, Style};
use sp_engine::influence;
use sp_engine::netlist::{self, MAX_NODES};

/// xorshift64*: deterministic, no dependencies.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const VOCAB: &[&str] = &[
    ".node",
    ".member",
    ".load",
    ".material",
    ".section",
    ".units",
    ".static",
    ".buckle",
    ".plot",
    ".story",
    ".wind",
    "fix=xy",
    "fix=y",
    "fix=",
    "B0",
    "B1",
    "B6",
    "T3",
    "M8",
    "M*",
    "chord",
    "web",
    "steel",
    "0",
    "-0",
    "8",
    "1e308",
    "1e-320",
    "NaN",
    "inf",
    "-1",
    "0.0",
    "40t",
    "x=24",
    "x=",
    "lane=B0..B6",
    "lane=B6..B0",
    "lane=B0..B999999999",
    "lane=",
    "share=0",
    "share=0.5",
    "Fy=-80",
    "Fy=",
    "+self",
    "E=200GPa",
    "E=-1GPa",
    "fy=0MPa",
    "rho=-5",
    "A=0cm2",
    "A=1e300m2",
    "I=A^2",
    "I=-1cm4",
    "K=0",
    "K=1e9",
    "N(M8)",
    "N(",
    "vs",
    "truck.x",
    "0",
    "48",
    "0.000001",
    ";",
    "*",
    "..",
    "B1..",
    "..B5",
    ",",
    ",,",
    "\u{2212}80",
    "é",
    "\u{0}",
    "\t",
    "kN",
    "mm",
];

fn exercise(text: &str) {
    let Ok(net) = netlist::parse(text) else {
        return;
    };
    // writing and re-parsing never panics either
    let _ = netlist::parse(&netlist::write(&net));
    let Ok(report) = influence::analyse(&net) else {
        return;
    };
    let picks = influence::picks(&net, &report);
    if let Some(p) = picks.first() {
        let _ = influence::influence_line(&net, p.member);
    }
    for row in &report.rows {
        assert!(!row.id.is_empty());
    }
    let _ = report.status.label();
}

#[test]
fn mutated_netlists_never_panic() {
    let mut rng = Rng(0x5EED_CAFE_F00D_0001);
    let bases: Vec<String> = Style::ALL
        .iter()
        .flat_map(|&style| {
            [false, true].map(|braced| {
                generate::generate(&Params {
                    style,
                    braced,
                    material: MaterialKind::ALL[style as usize % 4],
                    ..Params::default()
                })
                .text
            })
        })
        .collect();
    for _ in 0..1500 {
        let base = &bases[rng.below(bases.len())];
        let mut lines: Vec<String> = base.lines().map(String::from).collect();
        for _ in 0..1 + rng.below(4) {
            if lines.is_empty() {
                lines.push(String::new());
            }
            let i = rng.below(lines.len());
            match rng.below(8) {
                0 => {
                    lines.remove(i);
                }
                1 => {
                    let j = rng.below(lines.len());
                    lines.swap(i, j);
                }
                2 => {
                    // replace one token
                    let mut toks: Vec<String> =
                        lines[i].split_whitespace().map(String::from).collect();
                    if !toks.is_empty() {
                        let k = rng.below(toks.len());
                        toks[k] = VOCAB[rng.below(VOCAB.len())].to_string();
                        lines[i] = toks.join("  ");
                    }
                }
                3 => {
                    // insert a random token
                    let t = VOCAB[rng.below(VOCAB.len())];
                    let line = &mut lines[i];
                    let at = line
                        .char_indices()
                        .map(|(b, _)| b)
                        .nth(rng.below(line.chars().count() + 1))
                        .unwrap_or(line.len());
                    line.insert_str(at, &format!(" {t} "));
                }
                4 => {
                    // truncate the line at a char boundary
                    let n = lines[i].chars().count();
                    let keep = rng.below(n + 1);
                    lines[i] = lines[i].chars().take(keep).collect();
                }
                5 => lines.insert(i, lines[i].clone()), // duplicate ids
                6 => {
                    // a random line from the vocabulary
                    let n = 1 + rng.below(7);
                    let l: Vec<&str> = (0..n).map(|_| VOCAB[rng.below(VOCAB.len())]).collect();
                    lines.insert(i, l.join(" "));
                }
                _ => {
                    // flip a character to random unicode / control
                    let chars: Vec<char> = lines[i].chars().collect();
                    if !chars.is_empty() {
                        let k = rng.below(chars.len());
                        let c = char::from_u32((rng.next() % 0x2FFF) as u32).unwrap_or('?');
                        lines[i] = chars[..k]
                            .iter()
                            .chain(std::iter::once(&c))
                            .chain(&chars[k + 1..])
                            .collect();
                    }
                }
            }
        }
        exercise(&lines.join("\n"));
    }
}

#[test]
fn garbage_and_extremes_never_panic() {
    let mut rng = Rng(42);
    for _ in 0..300 {
        let n = rng.below(400);
        let s: String = (0..n)
            .map(|_| char::from_u32((rng.next() % 0x7F) as u32).unwrap_or(' '))
            .collect();
        exercise(&s);
    }
    for s in [
        "",
        "\n\n\n",
        "*",
        ";",
        ".",
        "..",
        ".node",
        ".node A",
        ".node A 1e400 0",
        ".node A NaN 0",
        ".load",
        ".load truck",
        ".load truck lane=",
        ".plot N(M*) vs truck.x 0 1e9 1e-9",
        ".plot N(M*) vs truck.x 5 5 1",
        ".buckle euler K=",
        ".section s A=1e-320",
        ".material m E=1e-320Pa fy=1e-320Pa rho=0\n.section s A=1e-300\n.node A 0 0 fix=xy\n.node B 1e-300 0 fix=y\n.member M A B s\n",
        ".material m\n.section s A=1cm2\n.node A 0 0\n.node B 1 0\n.member M A B s\n.load t 1t x=0 lane=A,B\n",
        ".material steel\n.section s A=1cm2\n.node A 0 0 fix=xy\n.node B 1 0 fix=y\n.node C 0.5 0\n.member M1 A C s\n.member M2 C B s\n.load d C Fy=-1000\n",
    ] {
        exercise(s);
    }
}

#[test]
fn huge_counts_are_refused_quickly() {
    let mut s = String::from(".material steel\n.section s A=1cm2\n");
    for i in 0..5000 {
        s.push_str(&format!(".node N{i} {i} 0\n"));
    }
    s.push_str(".load d N0..N4999 Fy=-1\n.load t 1t lane=N0..N999999 x=1\n");
    let t = std::time::Instant::now();
    let e = netlist::parse(&s).unwrap_err();
    assert!(t.elapsed().as_millis() < 500);
    assert!(
        e.iter()
            .any(|d| d.message.contains(&format!("at most {MAX_NODES}")))
    );
    // a long line of nonsense is fine too
    let long = ".node ".to_string() + &"x".repeat(100_000);
    assert!(netlist::parse(&long).is_err());
}

#[test]
fn a_mechanism_reports_without_numbers() {
    let text = ".material steel\n.section s A=10cm2\n.node A 0 0 fix=xy\n.node B 4 0 fix=y\n.node C 4 4\n.node D 0 4\n.member M1 A B s\n.member M2 B C s\n.member M3 C D s\n.member M4 D A s\n.load d C 0 -10\n";
    let net = netlist::parse(text).unwrap();
    let r = influence::analyse(&net).unwrap();
    assert_eq!(r.status.label(), "Mechanism");
    assert!(
        r.rows
            .iter()
            .all(|row| row.force == "\u{2014}" && row.limit == "unstable")
    );
    assert_eq!(
        r.checks[0].value,
        "m + r \u{2212} 2j = 4 + 3 \u{2212} 8 = -1"
    );
    assert_eq!(r.checks[3].value, "no solution");
    assert!(r.failed());
    // no truck: no influence line
    assert!(influence::influence_line(&net, 0).is_none());
}
