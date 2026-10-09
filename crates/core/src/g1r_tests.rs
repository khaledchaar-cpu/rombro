use super::*;

/// Renders picks as text: `key: + picked | - rejected (reason)` for snapshot-style asserts.
fn render(names: &[&str], rules: &Rules) -> String {
    let mut out = String::new();
    for g in select(names, |n| n, rules) {
        out.push_str(&format!("{}\n", g.key));
        for p in &g.picked {
            out.push_str(&format!("  + {p}\n"));
        }
        for (r, why) in &g.rejected {
            out.push_str(&format!("  - {r} ({why:?})\n"));
        }
    }
    out
}

#[test]
fn snapshot_zelda_and_mario() {
    let names = [
        "Legend of Zelda, The - A Link to the Past (USA)",
        "Legend of Zelda, The - A Link to the Past (Germany)",
        "Legend of Zelda, The - A Link to the Past (Europe) (Beta)",
        "Legend of Zelda, The - A Link to the Past (Europe)",
        "Legend of Zelda, The - A Link to the Past (USA)[h2]",
        "Legend of Zelda, The - A Link to the Past (USA) (Alt 1)",
        "Super Mario World (USA)",
        "Super Mario World (Europe) (Rev 1)",
        "Super Mario World (Europe)",
        "Super Mario World (Japan)",
    ];
    let expected = "\
legend of zelda link to the past
  + Legend of Zelda, The - A Link to the Past (Europe)
  - Legend of Zelda, The - A Link to the Past (Europe) (Beta) (Excluded(\"beta\"))
  - Legend of Zelda, The - A Link to the Past (USA)[h2] (Excluded(\"hack\"))
  - Legend of Zelda, The - A Link to the Past (USA) (Region)
  - Legend of Zelda, The - A Link to the Past (USA) (Alt 1) (Region)
  - Legend of Zelda, The - A Link to the Past (Germany) (Region)
super mario world
  + Super Mario World (Europe) (Rev 1)
  - Super Mario World (Europe) (Revision)
  - Super Mario World (USA) (Region)
  - Super Mario World (Japan) (Region)
";
    assert_eq!(render(&names, &Rules::default()), expected);
}

#[test]
fn snapshot_multi_disc_and_languages() {
    let names = [
        "Final Fantasy VII (USA) (Disc 1)",
        "Final Fantasy VII (USA) (Disc 2)",
        "Final Fantasy VII (USA) (Disc 3)",
        "Final Fantasy VII (Europe) (Fr) (Disc 1)",
        "Final Fantasy VII (Europe) (Fr) (Disc 2)",
        "Final Fantasy VII (Europe) (En) (Disc 1)",
        "Final Fantasy VII (Europe) (En) (Disc 2)",
        "Tetris (World) (Virtual Console)",
        "Tetris (World)",
        "Proto Only (USA) (Proto)",
    ];
    let expected = "\
final fantasy vii
  + Final Fantasy VII (Europe) (En) (Disc 1)
  + Final Fantasy VII (Europe) (En) (Disc 2)
  - Final Fantasy VII (Europe) (Fr) (Disc 1) (Language)
  - Final Fantasy VII (Europe) (Fr) (Disc 2) (Language)
  - Final Fantasy VII (USA) (Disc 1) (Region)
  - Final Fantasy VII (USA) (Disc 2) (Region)
  - Final Fantasy VII (USA) (Disc 3) (Region)
proto only
  - Proto Only (USA) (Proto) (Excluded(\"proto\"))
tetris
  + Tetris (World)
  - Tetris (World) (Virtual Console) (Variant)
";
    assert_eq!(render(&names, &Rules::default()), expected);
}

#[test]
fn custom_rules() {
    let rules = Rules {
        regions: vec!["Japan".into()],
        ..Rules::default()
    };
    let names = ["Mario (USA)", "Mario (Japan)", "Mario (Europe)"];
    let picks = select(&names, |n| n, &rules);
    assert_eq!(picks[0].picked, [&"Mario (Japan)"]);
}

#[test]
fn ties_need_a_decision() {
    let names = [
        "Game (USA) (Capcom Town)",
        "Game (USA)",
        "Other (Europe)",
        "Other (Europe)",
    ];
    let picks = select(&names, |n| n, &Rules::default());
    assert!(picks[0].needs_decision);
    assert!(!picks[1].needs_decision);
    assert_eq!(picks[1].picked.len(), 1);
    assert_eq!(picks[1].rejected[0].1, Reason::Duplicate);
}

#[test]
fn rules_json_fills_missing_fields_with_defaults() {
    let r: Rules = serde_json::from_str(r#"{"regions":["Japan"]}"#).unwrap();
    assert_eq!(r.regions, vec!["Japan".to_owned()]);
    assert_eq!(r.languages, Rules::default().languages);
    let back: Rules = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(back, r);
}

#[test]
fn atari_st_disk_parts_are_one_release() {
    let names = [
        "Disciples of Steel [cr Elite]",
        "Disciples of Steel (Intro)[cr Elite]",
        "Disciples of Steel (Boot)[cr Elite]",
        "Explora - Time run (France) [m Tom Pouce][Disk 3 and 4]",
        "Explora - Time run (France) [m Tom Pouce][Disk 1 and 2]",
    ];
    let picks = select(&names, |n| n, &Rules::default());
    assert!(picks.iter().all(|g| !g.needs_decision && g.rejected.is_empty()));
    let disciples: Vec<_> = picks[0].picked.iter().map(|n| **n).collect();
    assert_eq!(
        disciples,
        [
            "Disciples of Steel (Boot)[cr Elite]",
            "Disciples of Steel (Intro)[cr Elite]",
            "Disciples of Steel [cr Elite]"
        ]
    );
}
