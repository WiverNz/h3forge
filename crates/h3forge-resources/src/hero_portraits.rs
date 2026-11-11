//! Hero portrait identity: portrait ID → resource file → archive entry.
//!
//! A portrait ID (the `hero_portrait` byte of a save record, or the
//! portrait byte of an `.h3m` hero definition) is an index into the game's
//! portrait **name table**, not a number embedded in a file name:
//!
//! - IDs 0-162 are the Shadow of Death table. IDs 0-127 follow
//!   `HPL{id:03}{class}.pcx`; 128-143 are the Conflux heroes
//!   (`HPL000pl`..`HPL007el`, numbered per class); 144-162 are campaign
//!   heroes and extra portraits with irregular names. Evidence: the name
//!   strings of the installed `h3hota.exe` read in reverse storage order
//!   reproduce 0-127 exactly and continue with the 35 names below, and the
//!   class suffixes agree with the sample save (Roland 152 = `HPL009Sh`,
//!   Xeron 155 = `HPL131Dm`).
//! - IDs 163-244 are HotA additions, listed in order in the `newport0`
//!   record of the installed `HotA.dat` (whose comment gives the first ID
//!   as 163). The sample save agrees: Cove hero 156 carries portrait 163.
//!
//! HotA 1.8 archives replace entry names with opaque 16-byte IDs, so the
//! HotA names cannot be looked up directly. The table below pins each name
//! to its entry ID in `HotA.lod`. How the pairing was established is
//! recorded in `docs/formats/GAME_RESOURCES_FORMAT.md` section F: the
//! archive directory is in file-name order, replacement art for base
//! portraits content-matches named `H3bitmap.lod` entries around every
//! HotA name, and the large and small portrait runs line up one to one.
//! Lookups are by entry ID only; an archive without the ID resolves
//! nothing rather than something else.
//!
//! Confidence: IDs 163-186 are pinned exactly. IDs 187-244 (`HPL179` to
//! `HPL236`) are **inferred, not conclusively verified**: 58 names share a
//! run of 59 images, and the first image is assumed to be the redrawn
//! `HPL130Kn`. Were that wrong, this whole range would be off by one entry.

use crate::ResourceError;
use crate::lod::ResourceLimits;
use crate::pcx::RgbaImage;

/// Class suffixes of the 16 base classes, eight heroes each (IDs 0-127).
const BASE_CLASS_SUFFIXES: [&str; 16] = [
    "Kn", "Cl", "Rn", "Dr", "Al", "Wz", "Hr", "Dm", "Dk", "Nc", "Ov", "Wl", "Br", "Bm", "Bs", "Wh",
];

/// Large-portrait stems for IDs 144-162, in table order.
const SOD_SPECIAL_STEMS: [&str; 19] = [
    "130Kn", "000Sh", "128Qc", "003Sh", "004Sh", "005Sh", "006Sh", "007Sh", "009Sh", "008Sh",
    "001Sh", "131Dm", "129Mk", "002Sh", "132Wl", "133Nc", "134Nc", "135Wi", "136Wi",
];

/// Highest portrait ID of the Shadow of Death table.
pub const SOD_PORTRAIT_MAX: u16 = 162;
/// First portrait ID of the HotA table.
pub const HOTA_PORTRAIT_MIN: u16 = 163;
/// Last portrait ID of the HotA table.
pub const HOTA_PORTRAIT_MAX: u16 = 244;

/// `(portrait ID, file name in HotA.dat, HotA.lod entry ID)`.
const HOTA_PORTRAITS: [(u16, &str, &str); 82] = [
    (163, "HPLP01.pcx", "2976411ac68da2f7387dd4fc2b450a52"),
    (164, "HPLP02.pcx", "3ca4c50b6069739cd61368b73ef66028"),
    (165, "HPLP03.pcx", "11d10ff382aa30e0a2c03765cd85e632"),
    (166, "HPLP05.pcx", "ef5561e49b4655e2310e1e36af455c32"),
    (167, "HPLP06.pcx", "081b9dec9ac05d0fbed369a6a2178588"),
    (168, "HPLP13.pcx", "930df1901a862a495dd840ee44185949"),
    (169, "HPLP15.pcx", "5b191284b0d7b983d603654d52685d62"),
    (170, "HPLP16.pcx", "9822b58eabce39daee90ab77dfd72439"),
    (171, "HPLP04.pcx", "e5c1ad6147824a3254d5aa6da82f962a"),
    (172, "HPLP07.pcx", "b82185f9f6e4b6480b027d0641bd6c82"),
    (173, "HPLP08.pcx", "628952f7b1e0a33a82bcacaa0c8e7bbe"),
    (174, "HPLP09.pcx", "8c8a5490ecd3643a0349c10323eae081"),
    (175, "HPLP10.pcx", "755359dc247af1d38dd5af523dd5eb09"),
    (176, "HPLP11.pcx", "29d8235f3636a541bed18785ae415e00"),
    (177, "HPLP12.pcx", "c48bc87a7b505db757a8ec3e5bb17f40"),
    (178, "HPLP14.pcx", "c920d3cf4fadf4581d73d1d3b12ebd72"),
    (179, "HCVC01.pcx", "42e1ff63c3549e684275eb77b13926ac"),
    (180, "HCVC02.pcx", "8b2393a763b46e2f708255ddd8a67452"),
    (181, "HPLP17.pcx", "1d592a9e8ce7031b6d091c67d89b0b19"),
    (182, "HPL003KX.pcx", "d97f033b5d384d8f76419709f6a8249f"),
    (183, "HPL122WX.pcx", "c52ae63373b1ab75e3cdf41bb83f8b8e"),
    (184, "HPL071DX.pcx", "86d5778e4666658cb2ec2cd3b0606ab7"),
    (185, "HPL116BX.pcx", "c8173aa8626b289e0998d87dce105ad9"),
    (186, "HPL020RX.pcx", "1fd376f7af592d7f91b0cfa65419c704"),
    (187, "HPL179.pcx", "48cbcb95abc14073b249253c9b7ab1f4"),
    (188, "HPL180.pcx", "70e98c84ba08388425f5e0b02b33d756"),
    (189, "HPL181.pcx", "27555b58275e28201245752d5410be46"),
    (190, "HPL182.pcx", "356ebd4957670505052880bbcd73b337"),
    (191, "HPL183.pcx", "38d3b696e985ed6edd3823283ee42d2d"),
    (192, "HPL184.pcx", "5316772ff3b4a699e0b89a54af332937"),
    (193, "HPL185.pcx", "d3cca8d6ee3eba0fca26a463a81d632f"),
    (194, "HPL186.pcx", "190b2b5e950c76994635605341ab3987"),
    (195, "HPL187.pcx", "0aa30a461782f768bcc6d925a205528d"),
    (196, "HPL188.pcx", "9732a1f3988edd45b3ab4cce23d8ad86"),
    (197, "HPL189.pcx", "3036dcba16f70f837673a1c60c7c48c3"),
    (198, "HPL190.pcx", "548d6ad9694cb486b493c5077299dc9e"),
    (199, "HPL191.pcx", "4e87ab0eba6a28c176f7fba4d1aaef95"),
    (200, "HPL192.pcx", "11d491587bcda5fba8c053d3f8350558"),
    (201, "HPL193.pcx", "6df7b775375a2fb3a0268cb67f5e3946"),
    (202, "HPL194.pcx", "45ee8fd3fc5261fe60274933cedb573f"),
    (203, "HPL195.pcx", "8851e9ad7224922bbc77c32d5d066b46"),
    (204, "HPL196.pcx", "2d076b05b682ce1f55826a4fe4c0b86f"),
    (205, "HPL197.pcx", "b11f9d972d97aad871f285b6fbece066"),
    (206, "HPL198.pcx", "c639f22ed47e3291a803bbc1aa0f4905"),
    (207, "HPL199.pcx", "4514b98280fafd50bf8b392b09215cfc"),
    (208, "HPL200.pcx", "6185b9c8e8ebcade120c16bece64b896"),
    (209, "HPL201.pcx", "fb4a9259474c63d4b20f79895d8fcb9d"),
    (210, "HPL202.pcx", "7227a026dce543c9b3f90cc8e44919c7"),
    (211, "HPL203.pcx", "9d85f5542d92cc579a3fe6a5fb7541be"),
    (212, "HPL204.pcx", "1f7ad4b26e711d13e374a38b72223df6"),
    (213, "HPL205.pcx", "3d264e62a65e734eef6b6ff8d13350ed"),
    (214, "HPL206.pcx", "eef7393f80c23f908a0cca32f8be65af"),
    (215, "HPL207.pcx", "58291d7aa0268a406f86a46a7fe7999d"),
    (216, "HPL208.pcx", "c81d982425ce6453875646c626df0264"),
    (217, "HPL209.pcx", "34e56b0d72581d008eb431e315be94e6"),
    (218, "HPL210.pcx", "0b1ef21ef5f5be7fb522f78eb7008a20"),
    (219, "HPL211.pcx", "9064d61873374729e7761958704e6319"),
    (220, "HPL212.pcx", "8983bb3c15a4cf20ed3b1cece90014f0"),
    (221, "HPL213.pcx", "92795a1fdd82e00f3c2458f08aef00f9"),
    (222, "HPL214.pcx", "a6c488e9d8ceb3f2b92e6e1ed32ac4a2"),
    (223, "HPL215.pcx", "e0d0399e91d8f5eeb1b0e2923c91f2af"),
    (224, "HPL216.pcx", "3a86bdbef0ef85f2019846cc751fce6c"),
    (225, "HPL217.pcx", "592df8958cc523846aacbd2f067ea819"),
    (226, "HPL218.pcx", "8365f88ef752122c432f1aec5ff33d53"),
    (227, "HPL219.pcx", "475b6b0e5e55bf793939699858b72433"),
    (228, "HPL220.pcx", "f6f6c931a23afadc2e9dde2d6c66e2a0"),
    (229, "HPL221.pcx", "f524ef26953603741f03857d0300b493"),
    (230, "HPL222.pcx", "a272cc53ad7ee389b0a0e57bb66506f1"),
    (231, "HPL223.pcx", "24fda1d2c8af5eda741860a0e527a45a"),
    (232, "HPL224.pcx", "6f42bf9d89193cf8dce46bf6e0bfb309"),
    (233, "HPL225.pcx", "2cf797aba235c3c83a3e05d9e7d98110"),
    (234, "HPL226.pcx", "86b2e2b5480e12060a119ccbfa8ad8e6"),
    (235, "HPL227.pcx", "125f0453e4a24adcad805f4c59015ee1"),
    (236, "HPL228.pcx", "9195d280cba3d5068bb8545d3438c453"),
    (237, "HPL229.pcx", "2b9f6d7df35224dfab46f92a8bb4f631"),
    (238, "HPL230.pcx", "8d8d11ecb8d3db8896a96d6d551a1df5"),
    (239, "HPL231.pcx", "5bdc8c8cf034ad637524ce5566f0d78b"),
    (240, "HPL232.pcx", "e68e0bc59fdc237791c6b6ea33066e17"),
    (241, "HPL233.pcx", "834a7d8f1151cca2655047f51caa0854"),
    (242, "HPL234.pcx", "1d73e589fed0acc081c403bc49910a7e"),
    (243, "HPL235.pcx", "132a35930b7c430422fae15aea61306b"),
    (244, "HPL236.pcx", "6a64219e8612267271d6c5d2976453ab"),
];

/// Where one portrait's artwork lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeroPortraitFile {
    /// Named entries of the base bitmap archive (`H3bitmap.lod`).
    Base { large: String, small: String },
    /// One entry of `HotA.lod`, addressed by its opaque ID. `name` is the
    /// game's file name for it, for messages only.
    HotA {
        name: &'static str,
        entry_id: &'static str,
    },
}

/// Resolves a portrait ID through the name tables. `None` for IDs outside
/// 0-244: no table entry exists, so no artwork is ever substituted.
pub fn hero_portrait_file(portrait_id: u16) -> Option<HeroPortraitFile> {
    let stem = match portrait_id {
        0..=127 => format!(
            "{portrait_id:03}{}",
            BASE_CLASS_SUFFIXES[(portrait_id / 8) as usize]
        ),
        128..=135 => format!("{:03}pl", portrait_id - 128),
        136..=143 => format!("{:03}el", portrait_id - 136),
        144..=SOD_PORTRAIT_MAX => SOD_SPECIAL_STEMS[(portrait_id - 144) as usize].to_string(),
        HOTA_PORTRAIT_MIN..=HOTA_PORTRAIT_MAX => {
            let (id, name, entry_id) = HOTA_PORTRAITS[(portrait_id - HOTA_PORTRAIT_MIN) as usize];
            debug_assert_eq!(id, portrait_id);
            return Some(HeroPortraitFile::HotA { name, entry_id });
        }
        _ => return None,
    };
    Some(HeroPortraitFile::Base {
        large: format!("HPL{stem}.pcx"),
        small: format!("HPS{stem}.pcx"),
    })
}

const P32_MAGIC: &[u8; 4] = b"P32F";
const P32_HEADER: usize = 40;

/// Whether the bytes are a HotA `P32F` true-colour image.
pub fn is_p32(data: &[u8]) -> bool {
    data.len() >= 4 && &data[..4] == P32_MAGIC
}

/// Decodes a HotA `P32F` image: a 40-byte header (`P32F`, 0, 32, total
/// size, pixel offset 40, pixel byte count, width, height, two zero words)
/// followed by `width * height` BGRA quads stored bottom-up.
pub fn decode_p32(data: &[u8], limits: ResourceLimits) -> Result<RgbaImage, ResourceError> {
    let invalid = |reason: &'static str| ResourceError::Hota32Invalid { reason };
    if !is_p32(data) {
        return Err(invalid("not a P32F image"));
    }
    if data.len() < P32_HEADER {
        return Err(invalid("P32F header truncated"));
    }
    let word = |at: usize| {
        u32::from_le_bytes(data[at..at + 4].try_into().expect("header bounded")) as usize
    };
    let (pixel_offset, pixel_bytes) = (word(16), word(20));
    let (width, height) = (word(24), word(28));
    if width == 0 || height == 0 {
        return Err(invalid("P32F dimensions are zero"));
    }
    let pixels = width
        .checked_mul(height)
        .filter(|&count| count <= limits.max_image_pixels)
        .ok_or(ResourceError::PixelLimit {
            limit: limits.max_image_pixels,
            claimed: width.saturating_mul(height),
        })?;
    if pixel_offset != P32_HEADER || pixel_bytes != pixels * 4 {
        return Err(invalid("P32F header does not match its dimensions"));
    }
    let raw = data
        .get(P32_HEADER..P32_HEADER + pixel_bytes)
        .ok_or(invalid("P32F pixels truncated"))?;
    let mut rgba = Vec::with_capacity(pixel_bytes);
    for row in (0..height).rev() {
        for quad in raw[row * width * 4..(row + 1) * width * 4]
            .as_chunks::<4>()
            .0
        {
            rgba.extend_from_slice(&[quad[2], quad[1], quad[0], quad[3]]);
        }
    }
    Ok(RgbaImage {
        width: width as u32,
        height: height as u32,
        data: rgba,
    })
}

/// Decodes portrait entry bytes of either supported encoding (`P32F` or
/// Heroes PCX) to RGBA.
pub fn decode_portrait_image(
    data: &[u8],
    limits: ResourceLimits,
) -> Result<RgbaImage, ResourceError> {
    if is_p32(data) {
        decode_p32(data, limits)
    } else {
        Ok(crate::pcx::decode_pcx(data, limits)?.to_rgba())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_names_follow_the_sod_table() {
        let name = |id| match hero_portrait_file(id) {
            Some(HeroPortraitFile::Base { large, .. }) => large,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(name(0), "HPL000Kn.pcx");
        assert_eq!(name(127), "HPL127Wh.pcx");
        assert_eq!(name(128), "HPL000pl.pcx");
        assert_eq!(name(143), "HPL007el.pcx");
        assert_eq!(name(144), "HPL130Kn.pcx");
        assert_eq!(name(152), "HPL009Sh.pcx");
        assert_eq!(name(162), "HPL136Wi.pcx");
    }

    #[test]
    fn hota_table_is_contiguous_and_bounded() {
        for (offset, (id, name, entry_id)) in HOTA_PORTRAITS.iter().enumerate() {
            assert_eq!(*id as usize, HOTA_PORTRAIT_MIN as usize + offset);
            assert!(name.ends_with(".pcx"));
            assert_eq!(entry_id.len(), 32);
        }
        assert!(matches!(
            hero_portrait_file(163),
            Some(HeroPortraitFile::HotA {
                name: "HPLP01.pcx",
                ..
            })
        ));
        assert!(matches!(
            hero_portrait_file(244),
            Some(HeroPortraitFile::HotA {
                name: "HPL236.pcx",
                ..
            })
        ));
        assert_eq!(hero_portrait_file(245), None);
    }

    #[test]
    fn p32_decodes_bottom_up_bgra() {
        let mut data = b"P32F".to_vec();
        for word in [0u32, 32, 48, 40, 8, 1, 2, 0, 0] {
            data.extend_from_slice(&word.to_le_bytes());
        }
        data.extend_from_slice(&[1, 2, 3, 255]); // bottom row, BGRA
        data.extend_from_slice(&[4, 5, 6, 255]); // top row
        let image = decode_p32(&data, ResourceLimits::default()).unwrap();
        assert_eq!((image.width, image.height), (1, 2));
        assert_eq!(image.data, vec![6, 5, 4, 255, 3, 2, 1, 255]);
        assert!(decode_p32(&data[..44], ResourceLimits::default()).is_err());
    }
}
