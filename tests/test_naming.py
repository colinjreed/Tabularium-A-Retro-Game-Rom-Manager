from rom_manager import naming


def test_regions_revision_disc():
    p = naming.parse("Final Fantasy VII (USA) (Disc 2)")
    assert p.title == "Final Fantasy VII"
    assert p.regions == ["USA"]
    assert p.disc == 2

    p = naming.parse("Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 2)")
    assert p.regions == ["USA", "Europe"]
    assert p.revision == (2,)


def test_article_and_key():
    p = naming.parse("Legend of Zelda, The - Link's Awakening DX (USA, Europe) (Rev 2) (SGB Enhanced)")
    assert p.title == "The Legend of Zelda - Link's Awakening DX"
    assert p.key == naming.parse("The Legend of Zelda - Link's Awakening DX (Japan)").key


def test_flags():
    assert "beta" in naming.parse("Sonic (Beta 2)").flags
    assert "proto" in naming.parse("Sonic (Proto)").flags
    assert "unlicensed" in naming.parse("Game (USA) (Unl)").flags
    assert "bios" in naming.parse("[BIOS] Game Boy Advance (World)").flags
    assert "translation" in naming.parse("Mother 3 (Japan) [T+Eng1.3]").flags
    assert "bad" in naming.parse("Contra (U) [b1]").flags
    p = naming.parse("Contra (U) [!]")
    assert p.verified and p.regions == ["USA"] and p.is_clean
    assert "virtualconsole" in naming.parse("Metroid (USA) (Virtual Console)").flags


def test_languages():
    p = naming.parse("Tintin (Europe) (En,Fr,De)")
    assert p.languages == ["en", "fr", "de"]


def test_libretro_safe():
    assert naming.libretro_safe("Q*bert: Arcade & More") == "Q_bert_ Arcade _ More"
