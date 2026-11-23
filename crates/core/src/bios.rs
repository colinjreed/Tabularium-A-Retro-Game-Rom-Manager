//! BIOS / firmware files: what each system needs, checking and installing.
//!
//! Hashes come from libretro's System.dat (libretro-database). "Required"
//! means games won't boot without it in the usual RetroArch core on
//! handhelds; "optional" files have a built-in (HLE) substitute.

use crate::hashing::md5_file;
use crate::profiles::Profile;
use crate::systems;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq)]
pub struct BiosFile {
    /// Path relative to the BIOS folder, e.g. "dc/dc_boot.bin".
    pub name: &'static str,
    /// Accepted hashes; empty = any (name check only).
    pub md5: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Requirement {
    pub system: &'static str,
    pub files: &'static [BiosFile],
    pub required: bool,
    /// One of the files is enough (e.g. PS1 regions).
    pub any_of: bool,
    pub note: &'static str,
}

macro_rules! f {
    ($n:expr) => { BiosFile { name: $n, md5: &[] } };
    ($n:expr, $($m:expr),+) => { BiosFile { name: $n, md5: &[$($m),+] } };
}

// PlayStation 2 BIOS dumps known to libretro's System.dat (PCSX2 / LRPS2).
pub static PS2_BIOS: &[BiosFile] = &[
    BiosFile {
        name: "pcsx2/bios/ps2-0100jd-20000117.bin",
        md5: &["32f2e4d5ff5ee11072a6bc45530f5765"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0100j-20000117.bin",
        md5: &["acf4730ceb38ac9d8c7d8e21f2614600"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0101jd-20000217.bin",
        md5: &["acf9968c8f596d2b15f42272082513d1"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0101j-20000217.bin",
        md5: &["b1459d7446c69e3e97e6ace3ae23dd1c"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0101xd-20000224.bin",
        md5: &["d3f1853a16c2ec18f3cd1ae655213308"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0110ad-20000727.bin",
        md5: &["63e6fd9b3c72e0d7b920e80cf76645cd"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0110a-20000727.bin",
        md5: &["a20c97c02210f16678ca3010127caf36"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0120a-20000902.bin",
        md5: &["8db2fbbac7413bf3e7154c1e0715e565"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0120ed-20000902.bin",
        md5: &["91c87cb2f2eb6ce529a2360f80ce2457"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0120ed-20000902-20030110.bin",
        md5: &["3016b3dd42148a67e2c048595ca4d7ce"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0120e-20000902.bin",
        md5: &["b7fa11e87d51752a98b38e3e691cbf17"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0120j-20001027-185015.bin",
        md5: &["f63bc530bd7ad7c026fcd6f7bd0d9525"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0120j-20001027-191435.bin",
        md5: &["cee06bd68c333fc5768244eae77e4495"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0150ad-20001228-20030520.bin",
        md5: &["0bf988e9c7aaa4c051805b0fa6eb3387"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0150a-20001228.bin",
        md5: &["8accc3c49ac45f5ae2c5db0adc854633"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0150ed-20001228-20030520.bin",
        md5: &["6f9a6feb749f0533aaae2cc45090b0ed"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0150e-20001228.bin",
        md5: &["838544f12de9b0abc90811279ee223c8"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0150jd-20010118.bin",
        md5: &["bb6bbc850458fff08af30e969ffd0175"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0150j-20010118.bin",
        md5: &["815ac991d8bc3b364696bead3457de7d"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160a-20010427.bin",
        md5: &["b107b5710042abe887c0f6175f6e94bb"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160j-20010427.bin",
        md5: &["ab55cceea548303c22c72570cfd4dd71"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160a-20010704.bin",
        md5: &["18bcaadb9ff74ed3add26cdf709fff2e"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160e-20010704.bin",
        md5: &["491209dd815ceee9de02dbbc408c06d6"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160a-20011004.bin",
        md5: &["7200a03d51cacc4c14fcdfdbc4898431"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160e-20011004.bin",
        md5: &["8359638e857c8bc18c3c18ac17d9cc3c"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160h-20010730.bin",
        md5: &["352d2ff9b3f68be7e6fa7e6dd8389346"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160a-20020207.bin",
        md5: &["d5ce2c7d119f563ce04bc04dbc3a323e"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160e-20020319.bin",
        md5: &["0d2228e6fd4fb639c9c39d077a9ec10c"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160j-20020426.bin",
        md5: &["72da56fccb8fcd77bba16d1b6f479914"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160e-20020426.bin",
        md5: &["5b1f47fbeb277c6be2fccdd6344ff2fd"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0160h-20020426.bin",
        md5: &["315a4003535dfda689752cb25f24785c"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0170j-20030206.bin",
        md5: &["312ad4816c232a9606e56f946bc0678a"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0170ed-20030227.bin",
        md5: &["666018ffec65c5c7e04796081295c6c7"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0170e-20030227.bin",
        md5: &["6e69920fa6eef8522a1d688a11e41bc6"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0170ad-20030325.bin",
        md5: &["eb960de68f0c0f7f9fa083e9f79d0360"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0170a-20030325.bin",
        md5: &["8aa12ce243210128c5074552d3b86251"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0180cd-20030224.bin",
        md5: &["240d4c5ddd4b54069bdc4a3cd2faf99d"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0180j-20031028.bin",
        md5: &["1c6cd089e6c83da618fbf2a081eb4888"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0190j-20030623.bin",
        md5: &["463d87789c555a4a7604e97d7db545d1"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0190a-20030623.bin",
        md5: &["35461cecaa51712b300b2d6798825048"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0190e-20030623.bin",
        md5: &["bd6415094e1ce9e05daabe85de807666"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0190h-20030623.bin",
        md5: &["2e70ad008d4ec8549aada8002fdf42fb"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0190r-20030623.bin",
        md5: &["b53d51edc7fc086685e31b811dc32aad"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0190c-20030623.bin",
        md5: &["1b6e631b536247756287b916f9396872"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0190j-20030822.bin",
        md5: &["00da1b177096cfd2532c8fa22b43e667"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0190e-20030822.bin",
        md5: &["afde410bd026c16be605a1ae4bd651fd"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0190a-20040329.bin",
        md5: &["81f4336c1de607dd0865011c0447052e"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0200j-20040614.bin",
        md5: &["0eee5d1c779aa50e94edd168b4ebf42e"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0200a-20040614.bin",
        md5: &["d333558cc14561c1fdc334c75d5f37b7"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0200e-20040614.bin",
        md5: &["dc752f160044f2ed5fc1f4964db2a095"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0200ed-20040614.bin",
        md5: &["63ead1d74893bf7f36880af81f68a82d"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0200h-20040614.bin",
        md5: &["3e3e030c0f600442fa05b94f87a1e238"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0210j-20040917.bin",
        md5: &["1ad977bb539fc9448a08ab276a836bbc"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220j-20050620.bin",
        md5: &["eb4f40fcf4911ede39c1bbfe91e7a89a"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220ad-20050620.bin",
        md5: &["9959ad7a8685cad66206e7752ca23f8b"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220a-20050620.bin",
        md5: &["929a14baca1776b00869f983aa6e14d2"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220e-20050620.bin",
        md5: &["573f7d4a430c32b3cc0fd0c41e104bbd"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220h-20050620.bin",
        md5: &["df63a604e8bff5b0599bd1a6c2721bd0"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220j-20060210.bin",
        md5: &["5b1ba4bb914406fae75ab8e38901684d"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220a-20060210.bin",
        md5: &["cb801b7920a7d536ba07b6534d2433ca"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220e-20060210.bin",
        md5: &["af60e6d1a939019d55e5b330d24b1c25"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220h-20060210.bin",
        md5: &["549a66d0c698635ca9fa3ab012da7129"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220j-20060905.bin",
        md5: &["5de9d0d730ff1e7ad122806335332524"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220ad-20060905.bin",
        md5: &["21fe4cad111f7dc0f9af29477057f88d"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220a-20060905.bin",
        md5: &["40c11c063b3b9409aa5e4058e984e30c"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220e-20060905.bin",
        md5: &["80bbb237a6af9c611df43b16b930b683"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0220h-20060905.bin",
        md5: &["c37bce95d32b2be480f87dd32704e664"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0230j-20080220.bin",
        md5: &["80ac46fa7e77b8ab4366e86948e54f83"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0230a-20080220.bin",
        md5: &["21038400dc633070a78ad53090c53017"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0230e-20080220.bin",
        md5: &["dc69f0643a3030aaa4797501b483d6c4"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0230h-20080220.bin",
        md5: &["30d56e79d89fbddf10938fa67fe3f34e"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0250e-20100415.bin",
        md5: &["93ea3bcee4252627919175ff1b16a1d9"],
    },
    BiosFile {
        name: "pcsx2/bios/ps2-0250j-20100415.bin",
        md5: &["d3e81e95db25f5a86a7b7474550a2155"],
    },
];

pub static REQUIREMENTS: &[Requirement] = &[
    Requirement {
        system: "psx",
        required: true,
        any_of: true,
        files: &[
            f!("scph5501.bin", "490f666e1afb15b7362b406ed1cea246"),
            f!("scph5500.bin", "8dd7d5296a650fac7319bce665a6a53c"),
            f!("scph5502.bin", "32736f17079d0b2b7024407c39bd3050"),
            f!("scph1001.bin", "924e392ed05558ffdb115408c263dccf"),
        ],
        note: "PCSX ReARMed can run without one, but many games need a real BIOS. \
               scph5501 = USA, scph5500 = Japan, scph5502 = Europe.",
    },
    Requirement {
        system: "segacd",
        required: true,
        any_of: true,
        files: &[
            f!("bios_CD_U.bin", "2efd74e3232ff260e371b99f84024f7f"),
            f!("bios_CD_E.bin", "e66fa1dc5820d254611fdcdba0662372"),
            f!("bios_CD_J.bin", "278a9397d192149e84e820ac621a8edd"),
        ],
        note: "Genesis Plus GX / PicoDrive use the BIOS matching each game's region \
               (U = USA, E = Europe, J = Japan).",
    },
    Requirement {
        system: "saturn",
        required: true,
        any_of: true,
        files: &[
            f!("mpr-17933.bin", "3240872c70984b6cbfda1586cab68dbe"),
            f!("sega_101.bin", "85ec9ca47d8f6807718151cbcca8b964"),
            f!("saturn_bios.bin", "af5828fdff51384f99b3c4926be27762"),
        ],
        note: "Beetle Saturn: mpr-17933 (US/EU) or sega_101 (JP). YabaSanshiro: saturn_bios.bin.",
    },
    Requirement {
        system: "dreamcast",
        required: true,
        any_of: false,
        files: &[f!("dc/dc_boot.bin", "e10c53c2f8b90bab96ead2d368858623")],
        note: "Flycast. Some builds include an HLE BIOS, with lower compatibility.",
    },
    Requirement {
        system: "pcenginecd",
        required: true,
        any_of: false,
        files: &[f!(
            "syscard3.pce",
            "38179df8f4ac870017db21ebcbf53114",
            "0754f903b52e3b3342202bdafb13efa5"
        )],
        note: "System Card 3.0 for Beetle PCE / PCE Fast.",
    },
    Requirement {
        system: "fds",
        required: true,
        any_of: false,
        files: &[f!("disksys.rom", "ca30b50f880eb660a320674ed365ef7a")],
        note: "",
    },
    Requirement {
        system: "atarilynx",
        required: true,
        any_of: false,
        files: &[f!("lynxboot.img", "fcd403db69f54290b51035d82f835e7b")],
        note: "Handy core.",
    },
    Requirement {
        system: "colecovision",
        required: true,
        any_of: false,
        files: &[f!("colecovision.rom", "2c66f5911e5b42b8ebe113403548eee7")],
        note: "",
    },
    Requirement {
        system: "arcade",
        required: false,
        any_of: false,
        files: &[f!("neogeo.zip")],
        note: "Needed for Neo Geo games only. Must match your MAME/FBNeo romset version, \
               so only the name is checked.",
    },
    Requirement {
        system: "mame",
        required: false,
        any_of: false,
        files: &[f!("neogeo.zip")],
        note: "Needed for Neo Geo games only. Must match your MAME romset version, so only the \
               name is checked. Many cores also look for it next to the games.",
    },
    Requirement {
        system: "ps2",
        required: true,
        any_of: true,
        files: PS2_BIOS,
        note: "PCSX2 / LRPS2 / AetherSX2 need a real PS2 BIOS from your console. Any region works \
               for most games.",
    },
    Requirement {
        system: "xbox",
        required: true,
        any_of: false,
        files: &[
            f!("mcpx_1.0.bin"),
            f!("Complex_4627.bin"),
            f!("xbox_hdd.qcow2"),
        ],
        note: "xemu needs the MCPX boot ROM, a flash BIOS and a hard-disk image. Hashes vary \
               between dumps, so only names are checked.",
    },
    Requirement {
        system: "gamecube",
        required: false,
        any_of: true,
        files: &[
            f!(
                "dolphin-emu/Sys/GC/USA/IPL.bin",
                "b17148254a5799684c7d783206504926"
            ),
            f!(
                "dolphin-emu/Sys/GC/EUR/IPL.bin",
                "db92574caab77a7ec99d4605fd6f2450"
            ),
            f!(
                "dolphin-emu/Sys/GC/JAP/IPL.bin",
                "b17148254a5799684c7d783206504926"
            ),
        ],
        note: "Dolphin runs games without it; the IPL only adds the GameCube boot animation.",
    },
    Requirement {
        system: "dos",
        required: false,
        any_of: false,
        files: &[
            f!("MT32_CONTROL.ROM", "5626206284b22c2734f3e9efefcd2675"),
            f!("MT32_PCM.ROM", "89e42e386e82e0cacb4a2704a03706ca"),
        ],
        note: "Roland MT-32 ROMs for DOSBox Pure's MIDI music; games play without them.",
    },
    Requirement {
        system: "nds",
        required: false,
        any_of: false,
        files: &[
            f!("bios7.bin", "df692a80a5b1bc90728bc3dfc76cd948"),
            f!("bios9.bin", "a392174eb3e572fed6447e956bde4b25"),
            f!("firmware.bin", "e45033d9b0fa6b0de071292bba7c9d13"),
        ],
        note: "DeSmuME runs without them; melonDS / DraStic benefit.",
    },
    Requirement {
        system: "gba",
        required: false,
        any_of: false,
        files: &[f!("gba_bios.bin", "a860e8c0b6d573d191e4ec7db1b1e4f6")],
        note: "mGBA / gpSP have an HLE BIOS; a few games need the real one.",
    },
    Requirement {
        system: "gb",
        required: false,
        any_of: false,
        files: &[f!("gb_bios.bin", "32fbbd84168d3482956eb3c5051637f5")],
        note: "Boot logo only.",
    },
    Requirement {
        system: "gbc",
        required: false,
        any_of: false,
        files: &[f!("gbc_bios.bin", "dbfce9db9deaa2567f6a84fde55f9680")],
        note: "Boot logo only.",
    },
    Requirement {
        system: "pokemini",
        required: false,
        any_of: false,
        files: &[f!("bios.min", "1e4fb124a3a886865acb574f388c803d")],
        note: "",
    },
    Requirement {
        system: "atari7800",
        required: false,
        any_of: false,
        files: &[f!("7800 BIOS (U).rom", "0763f1ffb006ddbe32e52d497ee848ae")],
        note: "",
    },
    Requirement {
        system: "3do",
        required: true,
        any_of: true,
        files: &[f!("panafz10.bin", "51f2f43ae2f3508a14d9f56597e2d3ce"), f!("panafz1.bin", "f47264dd47fe30f73ab3c010015c155b"), f!("panafz10-norsa.bin", "1477bda80dc33731a65468c1f5bcbee9"), f!("panafz1j.bin", "a496cfdded3da562759be3561317b605"), f!("goldstar.bin", "8639fd5e549bd6238cfee79e3e749114"), f!("sanyotry.bin", "35fa1a1ebaaeea286dc5cd15487c13ea"), f!("3do_arcade_saot.bin", "8970fc987ab89a7f64da9f8a8c4333ff")],
        note: "Opera core: panafz10.bin is the usual choice.",
    },
    Requirement {
        system: "atari5200",
        required: true,
        any_of: false,
        files: &[f!("5200.rom", "281f20ea4320404ec820fb7ec0693b38")],
        note: "",
    },
    Requirement {
        system: "atari800",
        required: true,
        any_of: true,
        files: &[f!("ATARIXL.ROM", "06daac977823773a3eea3422fd26a703"), f!("ATARIOSB.ROM", "a3e8d617c95d08031fe1b20d541434b2"), f!("ATARIOSA.ROM", "eb1f32f5d9f382db1bbfb8d7f9cb343a"), f!("ATARIBAS.ROM", "0bac0c6a50104045d902df4503a4c30b")],
        note: "ATARIXL.ROM covers most XL/XE games; 400/800 games need ATARIOSB.ROM.",
    },
    Requirement {
        system: "atarist",
        required: true,
        any_of: false,
        files: &[f!("tos.img", "c1c57ce48e8ee4135885cee9e63a68a2")],
        note: "Hatari needs a TOS image named tos.img.",
    },
    Requirement {
        system: "amiga",
        required: true,
        any_of: true,
        files: &[f!("kick34005.A500", "82a21c1890cae844b3df741f2762d48d"), f!("kick37175.A500", "dc10d7bdd1b6f450773dfb558477c230"), f!("kick40063.A600", "e40a5dfb3d017ba8779faba30cbd1c8e"), f!("kick40068.A1200", "646773759326fbac3b2311fd8c8793ee"), f!("kick33180.A500", "85ad74194e87c08904327de1a9443b7a"), f!("kick37350.A600", "465646c9b6729f77eea5314d1f057951"), f!("kick39106.A1200", "b7cc148386aa631136f510cd29e42fc3")],
        note: "Kickstart 1.3 (kick34005.A500) for most A500 games; 3.1 (kick40068.A1200) for AGA games.",
    },
    Requirement {
        system: "cdtv",
        required: true,
        any_of: false,
        files: &[f!("kick34005.CDTV", "89da1838a24460e4b93f4f0c5d92d48d")],
        note: "",
    },
    Requirement {
        system: "amigacd32",
        required: true,
        any_of: false,
        files: &[f!("kick40060.CD32", "5f8924d013dd57a89cf349f4cdedc6b1"), f!("kick40060.CD32.ext", "bb72565701b1b6faece07d68ea5da639")],
        note: "",
    },
    Requirement {
        system: "scv",
        required: true,
        any_of: false,
        files: &[f!("upd7801g.s01", "635a978fd40db9a18ee44eff449fc126")],
        note: "",
    },
    Requirement {
        system: "channelf",
        required: true,
        any_of: false,
        files: &[f!("sl31253.bin", "ac9804d4c0e9d07e33472e3726ed15c3"), f!("sl31254.bin", "da98f4bb3242ab80d76629021bb27585")],
        note: "FreeChaF; sl90025.bin can replace sl31253.bin.",
    },
    Requirement {
        system: "doom",
        required: false,
        any_of: false,
        files: &[f!("prboom.wad", "72ae1b47820fcc93cc0df9c428d0face")],
        note: "PrBoom's resource file; games run without it in most setups.",
    },
    Requirement {
        system: "j2me",
        required: true,
        any_of: true,
        files: &[f!("freej2me-lr.jar", "ccd92e7156ce2f0ce14c88ffb68a16eb"), f!("freej2me.jar", "29a92d0867da2917275b7c6c805d256f"), f!("squirreljme.jar", "cf849d4b36cd8b7782de7887d318182a")],
        note: "The emulator itself ships as a .jar in the BIOS folder.",
    },
    Requirement {
        system: "macintosh",
        required: true,
        any_of: false,
        files: &[f!("MacII.ROM", "66223be1497460f1e60885eeb35e03cc")],
        note: "",
    },
    Requirement {
        system: "odyssey2",
        required: true,
        any_of: false,
        files: &[f!("o2rom.bin", "562d5ebf9e030a40d6fabfc2f33139fd")],
        note: "",
    },
    Requirement {
        system: "videopac",
        required: true,
        any_of: true,
        files: &[f!("o2rom.bin", "562d5ebf9e030a40d6fabfc2f33139fd"), f!("g7400.bin", "c500ff71236068e0dc0d0603d265ae76"), f!("c52.bin", "f1071cdb0b6b10dde94d3bc8a6146387"), f!("jopac.bin", "279008e4a0db2dc5f1c048853b033828")],
        note: "o2rom.bin for Videopac G7000 games; g7400.bin for Videopac+ games.",
    },
    Requirement {
        system: "intellivision",
        required: true,
        any_of: false,
        files: &[f!("exec.bin", "62e761035cb657903761800f4437b8af"), f!("grom.bin", "0cd5946c6473e42e8e4c2137785e427f")],
        note: "",
    },
    Requirement {
        system: "pc98",
        required: true,
        any_of: false,
        files: &[f!("np2kai/bios.rom", "cd237e16e7e77c06bb58540e9e9fca68"), f!("np2kai/itf.rom", "72ea51443070f0e9212bfc9b793ee28e"), f!("np2kai/FONT.ROM", "38d32748ae49d1815b0614970849fd40"), f!("np2kai/sound.rom", "524473c1a5a03b17e21d86a0408ff827")],
        note: "Neko Project II Kai.",
    },
    Requirement {
        system: "pc88",
        required: true,
        any_of: false,
        files: &[f!("quasi88/n88.rom", "4f984e04a99d56c4cfe36115415d6eb8"), f!("quasi88/n88_0.rom", "e28fe3f520bea594350ea8fb00395370"), f!("quasi88/n88_1.rom", "a8e298da7ac947669bcb1ff25cee0a83"), f!("quasi88/n88_2.rom", "9d03154fd9abfc28c4e6d4dc705e6e23"), f!("quasi88/n88_3.rom", "e1791f8154f1cdf22b576a1a365b6e1f"), f!("quasi88/n88n.rom", "2ff07b8769367321128e03924af668a0"), f!("quasi88/disk.rom", "793f86784e5608352a5d7f03f03e0858")],
        note: "QUASI88.",
    },
    Requirement {
        system: "pcfx",
        required: true,
        any_of: false,
        files: &[f!("pcfx.rom", "08e36edbea28a017f79f8d4f7ff9b6d7")],
        note: "",
    },
    Requirement {
        system: "n64dd",
        required: true,
        any_of: false,
        files: &[f!("64DD_IPL.bin", "8d3d9f294b6e174bc7b1d2fd1c727530")],
        note: "",
    },
    Requirement {
        system: "satellaview",
        required: true,
        any_of: true,
        files: &[f!("BS-X.bin", "fed4d8242cfbed61343d53d48432aced"), f!("BS-X (En).bin", "33b62505da6ca4525e3839db4e1a7bca"), f!("BS-X (En) (DRM-Free).bin", "4ed9648505ab33a4daec93707b16caba")],
        note: "",
    },
    Requirement {
        system: "sufami",
        required: true,
        any_of: false,
        files: &[f!("STBIOS.bin", "d3a44ba7d42a74d3ac58cb9c14c6a5ca")],
        note: "",
    },
    Requirement {
        system: "cdimono1",
        required: true,
        any_of: true,
        files: &[f!("cdimono1.zip", "801c529739b83e1490b897f2d3ffbb3a"), f!("same_cdi/bios/cdimono1.zip", "c59f92647701428bc453976740eb75cf"), f!("cdi220b.rom", "ac0d468be366779c9df633be98da250a"), f!("cdi200.rom", "2969341396aa61e0143dc2351aaa6ef6")],
        note: "",
    },
    Requirement {
        system: "palm",
        required: false,
        any_of: true,
        files: &[f!("palmos41-en-m515.rom", "83cb1d1c76e568b916dc2e7c0bf669f6"), f!("palmos40-en-m500.rom", "abed11421f47bbf3f654af618e0c6a8a"), f!("palmos52-en-t3.rom", "a46861541c63b20aba7bf172b0d853e1"), f!("palmos60-en-t3.rom", "c2fae3ff41cc4f94be0fdaed1523ea99")],
        note: "",
    },
    Requirement {
        system: "naomi",
        required: true,
        any_of: false,
        files: &[f!("dc/naomi.zip", "526eda1e2a7920c92c88178789d71d84")],
        note: "Flycast.",
    },
    Requirement {
        system: "naomi2",
        required: true,
        any_of: false,
        files: &[f!("dc/naomi2.zip", "c50072cbab75673e1b1a6b94355e6fa8")],
        note: "Flycast.",
    },
    Requirement {
        system: "atomiswave",
        required: true,
        any_of: false,
        files: &[f!("dc/awbios.zip", "85254fbe320ca82a768ec2c26bb08def")],
        note: "Flycast.",
    },
    Requirement {
        system: "x1",
        required: true,
        any_of: false,
        files: &[f!("xmil/IPLROM.X1", "eeeea1cd29c6e0e8b094790ae969bfa7")],
        note: "",
    },
    Requirement {
        system: "x68000",
        required: true,
        any_of: false,
        files: &[f!("keropi/iplrom.dat", "7fd4caabac1d9169e289f0f7bbf71d8e"), f!("keropi/cgrom.dat", "cb0a5cfcf7247a7eab74bb2716260269")],
        note: "PX68k.",
    },
    Requirement {
        system: "neogeocd",
        required: true,
        any_of: true,
        files: &[f!("neocd/neocd.bin", "f39572af7584cb5b3f70ae8cc848aba2"), f!("neocd/neocd_z.rom", "11526d58d4c524daef7d5d677dc6b004"), f!("neocd/neocd_f.rom", "8834880c33164ccbe6476b559f3e37de"), f!("neocd/neocd_sz.rom", "971ee8a36fb72da57aed01758f0a37f5"), f!("neocd/uni-bioscd.rom", "08ca8b2dba6662e8024f9e789711c6fc"), f!("neocdz.zip", "66bf2aeb1b870eac603925601e48af82")],
        note: "NeoCD core; also wants neocd/000-lo.lo.",
    },
    Requirement {
        system: "vircon32",
        required: true,
        any_of: false,
        files: &[f!("Vircon32Bios.v32", "73f04397cbb2dafcf2047b19a856153a")],
        note: "",
    },
    Requirement {
        system: "neogeo",
        required: true,
        any_of: false,
        files: &[f!("neogeo.zip")],
        note: "Must match your MAME/FBNeo romset version, so only the name is checked. Many cores also look for it next to the games.",
    },
    Requirement {
        system: "switch",
        required: true,
        any_of: false,
        files: &[f!("prod.keys")],
        note: "Switch emulators need keys dumped from your own console; only the name is checked.",
    },
];

/// Requirements for these systems (None = all).
pub fn requirements_for(systems: Option<&[String]>) -> Vec<&'static Requirement> {
    REQUIREMENTS
        .iter()
        .filter(|r| systems.is_none_or(|s| s.iter().any(|x| x == r.system)))
        .collect()
}

fn base_name(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    Ok,
    Missing,
    BadHash(String),
    /// Right file, wrong place (card checks only).
    Misplaced(PathBuf),
}

#[derive(Debug, Clone)]
pub struct FileStatus {
    pub file: &'static BiosFile,
    pub state: FileState,
    pub found: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ReqStatus {
    pub req: &'static Requirement,
    pub files: Vec<FileStatus>,
}

impl ReqStatus {
    pub fn satisfied(&self) -> bool {
        let mut ok = self.files.iter().map(|f| f.state == FileState::Ok);
        if self.req.any_of {
            ok.any(|x| x)
        } else {
            ok.all(|x| x)
        }
    }
}

const MAX_BIOS: u64 = 64 * 1024 * 1024;

#[derive(Default)]
struct FolderIndex {
    by_md5: HashMap<String, PathBuf>,
    by_name: HashMap<String, PathBuf>,
}

fn index_folder(folder: &Path) -> FolderIndex {
    let mut idx = FolderIndex::default();
    for e in WalkDir::new(folder)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
    {
        let p = e.path().to_path_buf();
        idx.by_name
            .entry(e.file_name().to_string_lossy().to_lowercase())
            .or_insert(p.clone());
        if e.metadata().map(|m| m.len() <= MAX_BIOS).unwrap_or(false) {
            if let Ok(h) = md5_file(&p) {
                idx.by_md5.entry(h).or_insert(p);
            }
        }
    }
    idx
}

/// Check BIOS files in `folder`.
///
/// With a profile, `folder` is an SD card root and each system's BIOS
/// folder for that firmware is checked; otherwise files are looked for
/// anywhere under `folder`.
pub fn check(
    folder: &Path,
    systems: Option<&[String]>,
    layout: Option<&Profile>,
) -> Vec<ReqStatus> {
    let mut cache: HashMap<PathBuf, FolderIndex> = HashMap::new();
    let card_index = layout.map(|_| index_folder(folder));
    let mut out = vec![];
    for req in requirements_for(systems) {
        let base = match (layout, systems::get(req.system)) {
            (Some(p), Some(s)) => folder.join(p.bios_dir(s)),
            _ => folder.to_path_buf(),
        };
        let idx = cache
            .entry(base.clone())
            .or_insert_with(|| index_folder(&base));
        let mut files = vec![];
        for bf in req.files {
            let expected = base.join(bf.name);
            let (state, found) = if expected.is_file() {
                if bf.md5.is_empty() {
                    (FileState::Ok, Some(expected))
                } else {
                    match md5_file(&expected) {
                        Ok(h) if bf.md5.contains(&h.as_str()) => (FileState::Ok, Some(expected)),
                        Ok(h) => (FileState::BadHash(h), Some(expected)),
                        Err(_) => (FileState::Missing, None),
                    }
                }
            } else if layout.is_none() {
                let hit = bf
                    .md5
                    .iter()
                    .find_map(|m| idx.by_md5.get(*m))
                    .cloned()
                    .or_else(|| {
                        let named = idx.by_name.get(&base_name(bf.name).to_lowercase())?;
                        (bf.md5.is_empty()
                            || md5_file(named).is_ok_and(|h| bf.md5.contains(&h.as_str())))
                        .then(|| named.clone())
                    });
                match hit {
                    Some(p) => (FileState::Ok, Some(p)),
                    None => (FileState::Missing, None),
                }
            } else {
                // On a card: is the file somewhere else on it?
                let ci = card_index.as_ref().unwrap();
                match bf.md5.iter().find_map(|m| ci.by_md5.get(*m)) {
                    Some(p) => (FileState::Misplaced(p.clone()), Some(p.clone())),
                    None => (FileState::Missing, None),
                }
            };
            files.push(FileStatus {
                file: bf,
                state,
                found,
            });
        }
        out.push(ReqStatus { req, files });
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOutcome {
    Imported(String),
    AlreadyPresent(String),
    BadHash(String),
}

fn known_by_md5(h: &str) -> Option<&'static BiosFile> {
    REQUIREMENTS
        .iter()
        .flat_map(|r| r.files.iter())
        .find(|f| f.md5.contains(&h))
}

fn known_by_name(name: &str) -> Option<&'static BiosFile> {
    let n = name.to_lowercase();
    REQUIREMENTS
        .iter()
        .flat_map(|r| r.files.iter())
        .find(|f| base_name(f.name).to_lowercase() == n)
}

/// Copy recognised BIOS files (by hash, or by name for name-only entries)
/// into `dest`, under their correct names. Unknown files are ignored.
pub fn import_files(
    sources: &[PathBuf],
    dest: &Path,
    dry_run: bool,
) -> Vec<(PathBuf, ImportOutcome)> {
    let mut out = vec![];
    for src in sources {
        for e in WalkDir::new(src)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
        {
            if e.metadata().map(|m| m.len() > MAX_BIOS).unwrap_or(true) {
                continue;
            }
            let p = e.path().to_path_buf();
            let Ok(h) = md5_file(&p) else { continue };
            let file_name = e.file_name().to_string_lossy();
            let bf = match known_by_md5(&h) {
                Some(bf) => bf,
                None => match known_by_name(&file_name) {
                    Some(bf) if bf.md5.is_empty() => bf,
                    Some(bf) => {
                        out.push((
                            p,
                            ImportOutcome::BadHash(format!(
                                "named like {} but its MD5 {h} isn't a known good dump",
                                bf.name
                            )),
                        ));
                        continue;
                    }
                    None => continue,
                },
            };
            let target = dest.join(bf.name);
            if target.is_file() && md5_file(&target).is_ok_and(|t| t == h) {
                out.push((p, ImportOutcome::AlreadyPresent(bf.name.into())));
                continue;
            }
            if !dry_run {
                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&p, &target).is_err() {
                    continue;
                }
            }
            out.push((p, ImportOutcome::Imported(bf.name.into())));
        }
    }
    out
}

#[derive(Debug, Clone, Default)]
pub struct InstallReport {
    pub copied: Vec<String>,
    pub missing_required: Vec<String>,
    pub missing_optional: Vec<String>,
}

/// Copy the BIOS files these systems use from the library to a card.
pub fn install(
    library_bios: &Path,
    card: &Path,
    profile: &Profile,
    systems_list: &[String],
    dry_run: bool,
) -> InstallReport {
    let mut rep = InstallReport::default();
    if systems_list.is_empty() {
        return rep;
    }
    for rs in check(library_bios, Some(systems_list), None) {
        let Some(sys) = systems::get(rs.req.system) else {
            continue;
        };
        let dest_dir = card.join(profile.bios_dir(sys));
        for fs_ in &rs.files {
            if let (FileState::Ok, Some(found)) = (&fs_.state, &fs_.found) {
                let target = dest_dir.join(fs_.file.name);
                let same = fs::metadata(&target).ok().map(|m| m.len())
                    == fs::metadata(found).ok().map(|m| m.len());
                if !same {
                    if !dry_run {
                        if let Some(p) = target.parent() {
                            let _ = fs::create_dir_all(p);
                        }
                        let _ = fs::copy(found, &target);
                    }
                    rep.copied
                        .push(format!("{}: {}", rs.req.system, fs_.file.name));
                }
            }
        }
        if !rs.satisfied() {
            let names: Vec<&str> = rs
                .files
                .iter()
                .filter(|f| f.state != FileState::Ok)
                .map(|f| f.file.name)
                .collect();
            let joined = if rs.req.any_of && names.len() > 6 {
                format!("any one of {} known BIOS dumps", names.len())
            } else {
                names.join(if rs.req.any_of { " or " } else { ", " })
            };
            let label = format!("{}: {joined}", systems::name_of(rs.req.system));
            if rs.req.required {
                rep.missing_required.push(label)
            } else {
                rep.missing_optional.push(label)
            }
        }
    }
    rep
}
