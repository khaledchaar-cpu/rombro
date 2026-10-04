use super::*;

#[test]
fn regions_languages_revision() {
    let i = parse("Super Mario World (USA, Europe) (En,Fr,De) (Rev 1)");
    assert_eq!(i.title, "Super Mario World");
    assert_eq!(i.regions, ["USA", "Europe"]);
    assert_eq!(i.languages, ["En", "Fr", "De"]);
    assert_eq!(i.revision, 1);
    assert_eq!(i.flags, Flags::default());
}

#[test]
fn flags_and_brackets() {
    let i = parse("Legend of Zelda, The - A Link to the Past (France) (Beta 2)");
    assert_eq!(i.title, "Legend of Zelda, The - A Link to the Past");
    assert!(i.flags.beta && i.flags.is_unofficial());
    assert!(parse("Zelda (USA)[h2]").flags.hack);
    assert!(parse("Zelda (USA)[tr es](Alt 1)").flags.translation);
    assert!(parse("Zelda (USA)[tr es](Alt 1)").flags.alt);
    assert!(parse("Zelda (USA) [T-En by X v1.0]").flags.translation);
    assert!(parse("Zelda (USA) [b]").flags.bad_dump);
    assert!(!parse("Zelda (USA) [!]").flags.is_unofficial());
    assert!(parse("[BIOS] PlayStation (Europe) (v3.0)").flags.bios);
    assert_eq!(
        parse("[BIOS] PlayStation (Europe) (v3.0)").title,
        "PlayStation"
    );
    assert!(
        parse("Mario (Japan) (Virtual Console)")
            .flags
            .virtual_console
    );
    assert!(parse("Zelda (World) (REV-F) (Digital)").flags.rerelease);
    assert!(parse("Game (USA) (Unl)").flags.unlicensed);
    assert!(parse("Afraid Gear [Reprint] (Japan)").flags.rerelease);
    assert!(parse("Acid (Japan) (Major Wave)").flags.rerelease);
    assert!(parse("Addie (Japan) (Otameshi-ban)").flags.demo);
}

#[test]
fn revisions_and_discs() {
    assert_eq!(parse("Zelda (World) (REV-F)").revision, 6);
    assert_eq!(parse("Zelda (USA) (Rev A)").revision, 1);
    assert_eq!(parse("Game (Europe) (v1.12)").revision, 1012);
    assert_eq!(parse("FF7 (USA) (Disc 2)").disc, Some(2));
    assert_eq!(parse("Game (Disk B)").disc, Some(2));
    assert_eq!(parse("Game (USA)").disc, None);
    assert_eq!(parse("No tags").title, "No tags");
}
