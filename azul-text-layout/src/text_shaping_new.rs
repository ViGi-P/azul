use azul_core::app_resources::{
    FontMetrics, VariationSelector, Anchor,
    GlyphOrigin, RawGlyph, Placement, MarkPlacement,
    Info, Advance,
};
use tinyvec::tiny_vec;
use std::rc::Rc;

use allsorts::binary::read::ReadScope;
use allsorts::font_data::FontData;
use crate::allsorts::tables::FontTableProvider;
use allsorts::layout::{LayoutCache, GDEFTable, GPOS, GSUB};
use allsorts::tables::HheaTable;
use allsorts::tables::HeadTable;
use allsorts::tables::MaxpTable;
use allsorts::tables::cmap::CmapSubtable;
use allsorts::tables::cmap::owned::CmapSubtable as OwnedCmapSubtable;
use allsorts::tables::MaxpVersion1SubTable;

pub struct SequentialMapGroup {
    pub start_char_code: u32,
    pub end_char_code: u32,
    pub start_glyph_id: u32,
}

pub fn get_font_metrics(font_bytes: &[u8], font_index: usize) -> FontMetrics {

    use std::num::NonZeroU16;
    use allsorts::tag;

    #[derive(Default)]
    struct Os2Info {
        x_avg_char_width: i16,
        us_weight_class: u16,
        us_width_class: u16,
        fs_type: u16,
        y_subscript_x_size: i16,
        y_subscript_y_size: i16,
        y_subscript_x_offset: i16,
        y_subscript_y_offset: i16,
        y_superscript_x_size: i16,
        y_superscript_y_size: i16,
        y_superscript_x_offset: i16,
        y_superscript_y_offset: i16,
        y_strikeout_size: i16,
        y_strikeout_position: i16,
        s_family_class: i16,
        panose: [u8; 10],
        ul_unicode_range1: u32,
        ul_unicode_range2: u32,
        ul_unicode_range3: u32,
        ul_unicode_range4: u32,
        ach_vend_id: u32,
        fs_selection: u16,
        us_first_char_index: u16,
        us_last_char_index: u16,
        s_typo_ascender: Option<i16>,
        s_typo_descender: Option<i16>,
        s_typo_line_gap: Option<i16>,
        us_win_ascent: Option<u16>,
        us_win_descent: Option<u16>,
        ul_code_page_range1: Option<u32>,
        ul_code_page_range2: Option<u32>,
        sx_height: Option<i16>,
        s_cap_height: Option<i16>,
        us_default_char: Option<u16>,
        us_break_char: Option<u16>,
        us_max_context: Option<u16>,
        us_lower_optical_point_size: Option<u16>,
        us_upper_optical_point_size: Option<u16>,
    }

    let scope = ReadScope::new(font_bytes);
    let font_file = match scope.read::<FontData<'_>>() {
        Ok(o) => o,
        Err(_) => return FontMetrics::default(),
    };
    let provider = match font_file.table_provider(font_index) {
        Ok(o) => o,
        Err(_) => return FontMetrics::default(),
    };
    let font = match allsorts::font::Font::new(provider).ok() {
        Some(Some(s)) => s,
        _ => return FontMetrics::default(),
    };

    // read the HHEA table to get the metrics for horizontal layout
    let hhea_table = &font.hhea_table;
    let head_table = match font.head_table().ok() {
        Some(Some(s)) => s,
        _ => return FontMetrics::default(),
    };

    let os2_table = match font.os2_table().ok() {
        Some(Some(s)) => {
            Os2Info {
                x_avg_char_width: s.x_avg_char_width,
                us_weight_class: s.us_weight_class,
                us_width_class: s.us_width_class,
                fs_type: s.fs_type,
                y_subscript_x_size: s.y_subscript_x_size,
                y_subscript_y_size: s.y_subscript_y_size,
                y_subscript_x_offset: s.y_subscript_x_offset,
                y_subscript_y_offset: s.y_subscript_y_offset,
                y_superscript_x_size: s.y_superscript_x_size,
                y_superscript_y_size: s.y_superscript_y_size,
                y_superscript_x_offset: s.y_superscript_x_offset,
                y_superscript_y_offset: s.y_superscript_y_offset,
                y_strikeout_size: s.y_strikeout_size,
                y_strikeout_position: s.y_strikeout_position,
                s_family_class: s.s_family_class,
                panose: s.panose,
                ul_unicode_range1: s.ul_unicode_range1,
                ul_unicode_range2: s.ul_unicode_range2,
                ul_unicode_range3: s.ul_unicode_range3,
                ul_unicode_range4: s.ul_unicode_range4,
                ach_vend_id: s.ach_vend_id,
                fs_selection: s.fs_selection,
                us_first_char_index: s.us_first_char_index,
                us_last_char_index: s.us_last_char_index,

                s_typo_ascender: s.version0.as_ref().map(|q| q.s_typo_ascender),
                s_typo_descender: s.version0.as_ref().map(|q| q.s_typo_descender),
                s_typo_line_gap: s.version0.as_ref().map(|q| q.s_typo_line_gap),
                us_win_ascent: s.version0.as_ref().map(|q| q.us_win_ascent),
                us_win_descent: s.version0.as_ref().map(|q| q.us_win_descent),

                ul_code_page_range1: s.version1.as_ref().map(|q| q.ul_code_page_range1),
                ul_code_page_range2: s.version1.as_ref().map(|q| q.ul_code_page_range2),

                sx_height: s.version2to4.as_ref().map(|q| q.sx_height),
                s_cap_height: s.version2to4.as_ref().map(|q| q.s_cap_height),
                us_default_char: s.version2to4.as_ref().map(|q| q.us_default_char),
                us_break_char: s.version2to4.as_ref().map(|q| q.us_break_char),
                us_max_context: s.version2to4.as_ref().map(|q| q.us_max_context),

                us_lower_optical_point_size: s.version5.as_ref().map(|q| q.us_lower_optical_point_size),
                us_upper_optical_point_size: s.version5.as_ref().map(|q| q.us_upper_optical_point_size),
            }
        },
        _ => Os2Info::default(),
    };

    FontMetrics {

        // head table
        units_per_em: NonZeroU16::new(head_table.units_per_em).unwrap_or(unsafe { NonZeroU16::new_unchecked(1000) }),
        font_flags: head_table.flags,
        x_min: head_table.x_min,
        y_min: head_table.y_min,
        x_max: head_table.x_max,
        y_max: head_table.y_max,

        // hhea table
        ascender: hhea_table.ascender,
        descender: hhea_table.descender,
        line_gap: hhea_table.line_gap,
        advance_width_max: hhea_table.advance_width_max,
        min_left_side_bearing: hhea_table.min_left_side_bearing,
        min_right_side_bearing: hhea_table.min_right_side_bearing,
        x_max_extent: hhea_table.x_max_extent,
        caret_slope_rise: hhea_table.caret_slope_rise,
        caret_slope_run: hhea_table.caret_slope_run,
        caret_offset: hhea_table.caret_offset,
        num_h_metrics: hhea_table.num_h_metrics,

        // os/2 table

        x_avg_char_width: os2_table.x_avg_char_width,
        us_weight_class: os2_table.us_weight_class,
        us_width_class: os2_table.us_width_class,
        fs_type: os2_table.fs_type,
        y_subscript_x_size: os2_table.y_subscript_x_size,
        y_subscript_y_size: os2_table.y_subscript_y_size,
        y_subscript_x_offset: os2_table.y_subscript_x_offset,
        y_subscript_y_offset: os2_table.y_subscript_y_offset,
        y_superscript_x_size: os2_table.y_superscript_x_size,
        y_superscript_y_size: os2_table.y_superscript_y_size,
        y_superscript_x_offset: os2_table.y_superscript_x_offset,
        y_superscript_y_offset: os2_table.y_superscript_y_offset,
        y_strikeout_size: os2_table.y_strikeout_size,
        y_strikeout_position: os2_table.y_strikeout_position,
        s_family_class: os2_table.s_family_class,
        panose: os2_table.panose,
        ul_unicode_range1: os2_table.ul_unicode_range1,
        ul_unicode_range2: os2_table.ul_unicode_range2,
        ul_unicode_range3: os2_table.ul_unicode_range3,
        ul_unicode_range4: os2_table.ul_unicode_range4,
        ach_vend_id: os2_table.ach_vend_id,
        fs_selection: os2_table.fs_selection,
        us_first_char_index: os2_table.us_first_char_index,
        us_last_char_index: os2_table.us_last_char_index,
        s_typo_ascender: os2_table.s_typo_ascender,
        s_typo_descender: os2_table.s_typo_descender,
        s_typo_line_gap: os2_table.s_typo_line_gap,
        us_win_ascent: os2_table.us_win_ascent,
        us_win_descent: os2_table.us_win_descent,
        ul_code_page_range1: os2_table.ul_code_page_range1,
        ul_code_page_range2: os2_table.ul_code_page_range2,
        sx_height: os2_table.sx_height,
        s_cap_height: os2_table.s_cap_height,
        us_default_char: os2_table.us_default_char,
        us_break_char: os2_table.us_break_char,
        us_max_context: os2_table.us_max_context,
        us_lower_optical_point_size: os2_table.us_lower_optical_point_size,
        us_upper_optical_point_size: os2_table.us_upper_optical_point_size,
    }
}

pub struct ParsedFont {
    pub font_metrics: FontMetrics,
    pub num_glyphs: u16,
    pub hhea_table: HheaTable,
    pub hmtx_data: Box<[u8]>,
    pub vhea_table: HheaTable,
    pub vmtx_data: Box<[u8]>,
    pub maxp_table: MaxpTable,
    pub gsub_cache: LayoutCache<GSUB>,
    pub gpos_cache: LayoutCache<GPOS>,
    pub gdef_table: Rc<GDEFTable>,
    pub cmap_subtable: OwnedCmapSubtable,
}

impl ParsedFont {

    pub fn from_bytes(font_bytes: &[u8], font_index: usize) -> Option<Self> {

        use allsorts::tag;

        let scope = ReadScope::new(font_bytes);
        let font_file = scope.read::<FontData<'_>>().ok()?;
        let provider = font_file.table_provider(font_index).ok()?;

        let hmtx_data = provider.table_data(tag::HMTX).ok()??.into_owned().into_boxed_slice();
        let vmtx_data = provider.table_data(tag::VMTX).ok()??.into_owned().into_boxed_slice();

        let vhea_data = provider.table_data(tag::VHEA).ok()??.into_owned();
        let vhea_table = ReadScope::new(&vhea_data).read::<HheaTable>().ok()?;

        let hhea_data = provider.table_data(tag::HHEA).ok()??.into_owned();
        let hhea_table = ReadScope::new(&hhea_data).read::<HheaTable>().ok()?;

        let maxp_data = provider.table_data(tag::MAXP).ok()??.into_owned();
        let maxp_table = ReadScope::new(&maxp_data).read::<MaxpTable>().ok()?;

        let font_metrics = get_font_metrics(font_bytes, font_index);

        let mut font_data_impl = allsorts::font::Font::new(provider).ok()??;

        // required for font layout: gsub_cache, gpos_cache and gdef_table
        let gsub_cache = font_data_impl.gsub_cache().ok()??;
        let gpos_cache = font_data_impl.gpos_cache().ok()??;
        let gdef_table = font_data_impl.gdef_table().ok()??;
        let num_glyphs = font_data_impl.num_glyphs();

        let cmap_subtable = ReadScope::new(font_data_impl.cmap_subtable_data()).read::<CmapSubtable<'_>>().ok()?.to_owned()?;

        Some(ParsedFont {
            font_metrics,
            num_glyphs,
            hhea_table,
            hmtx_data,
            vhea_table,
            vmtx_data,
            maxp_table,
            gsub_cache,
            gpos_cache,
            gdef_table,
            cmap_subtable,
        })
    }

    // get horizontal and vertical advance
    pub fn get_advance(&mut self, glyph_index: u16) -> (u16, u16) {
        let horizontal_advance = allsorts::glyph_info::advance(&self.maxp_table, &self.hhea_table, &self.hmtx_data, glyph_index).unwrap();
        let vertical_advance = allsorts::glyph_info::advance(&self.maxp_table, &self.vhea_table, &self.vmtx_data, glyph_index).unwrap();
        (horizontal_advance, vertical_advance)
    }

    pub fn shape(&mut self, text: &[char], script: u32, lang: u32) -> ShapedTextBufferUnsized {
        shape(self, text, script, lang).unwrap_or_default()
    }

    pub fn lookup_glyph_index(&self, c: u32) -> u16 {
        match self.cmap_subtable.map_glyph(c) {
            Ok(Some(c)) => c,
            _ => 0,
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct ShapedTextBufferUnsized {
    pub infos: Vec<Info>,
}

impl ShapedTextBufferUnsized {
    /// Get the word width in unscaled units (respects kerning)
    pub fn get_word_visual_width_unscaled(&self) -> usize {
        self.infos.iter().map(|s| s.size.get_x_total_unscaled() as usize).sum()
    }
}

/// Generate a 4-byte font table tag from byte string
///
/// Example:
///
/// ```
/// assert_eq!(tag!(b"glyf"), 0x676C7966);
/// ```
macro_rules! tag {
    ($w:expr) => {
        tag(*$w)
    };
}

const fn tag(chars: [u8; 4]) -> u32 {
    ((chars[3] as u32) << 0)
        | ((chars[2] as u32) << 8)
        | ((chars[1] as u32) << 16)
        | ((chars[0] as u32) << 24)
}

/// Estimate the language and the script from the text (uses trigrams)
pub fn estimate_script_and_language(text: &str) -> (u32, u32) {

    use allsorts::tag;
    use whatlang::{Script, Lang};

    // https://docs.microsoft.com/en-us/typography/opentype/spec/scripttags

    const TAG_ADLM: u32 = tag!(b"adlm"); // Adlam
    const TAG_AHOM: u32 = tag!(b"ahom"); // Ahom
    const TAG_HLUW: u32 = tag!(b"hluw"); // Anatolian Hieroglyphs
    const TAG_ARAB: u32 = tag!(b"arab"); // Arabic
    const TAG_ARMN: u32 = tag!(b"armn"); // Armenian
    const TAG_AVST: u32 = tag!(b"avst"); // Avestan
    const TAG_BALI: u32 = tag!(b"bali"); // Balinese
    const TAG_BAMU: u32 = tag!(b"bamu"); // Bamum
    const TAG_BASS: u32 = tag!(b"bass"); // Bassa Vah
    const TAG_BATK: u32 = tag!(b"batk"); // Batak
    const TAG_BENG: u32 = tag!(b"beng"); // Bengali
    const TAG_BNG2: u32 = tag!(b"bng2"); // Bengali v.2
    const TAG_BHKS: u32 = tag!(b"bhks"); // Bhaiksuki
    const TAG_BOPO: u32 = tag!(b"bopo"); // Bopomofo
    const TAG_BRAH: u32 = tag!(b"brah"); // Brahmi
    const TAG_BRAI: u32 = tag!(b"brai"); // Braille
    const TAG_BUGI: u32 = tag!(b"bugi"); // Buginese
    const TAG_BUHD: u32 = tag!(b"buhd"); // Buhid
    const TAG_BYZM: u32 = tag!(b"byzm"); // Byzantine Music
    const TAG_CANS: u32 = tag!(b"cans"); // Canadian Syllabics
    const TAG_CARI: u32 = tag!(b"cari"); // Carian
    const TAG_AGHB: u32 = tag!(b"aghb"); // Caucasian Albanian
    const TAG_CAKM: u32 = tag!(b"cakm"); // Chakma
    const TAG_CHAM: u32 = tag!(b"cham"); // Cham
    const TAG_CHER: u32 = tag!(b"cher"); // Cherokee
    const TAG_CHRS: u32 = tag!(b"chrs"); // Chorasmian
    const TAG_HANI: u32 = tag!(b"hani"); // CJK Ideographic
    const TAG_COPT: u32 = tag!(b"copt"); // Coptic
    const TAG_CPRT: u32 = tag!(b"cprt"); // Cypriot Syllabary
    const TAG_CYRL: u32 = tag!(b"cyrl"); // Cyrillic
    const TAG_DFLT: u32 = tag!(b"DFLT"); // Default
    const TAG_DSRT: u32 = tag!(b"dsrt"); // Deseret
    const TAG_DEVA: u32 = tag!(b"deva"); // Devanagari
    const TAG_DEV2: u32 = tag!(b"dev2"); // Devanagari v.2
    const TAG_DIAK: u32 = tag!(b"diak"); // Dives Akuru
    const TAG_DOGR: u32 = tag!(b"dogr"); // Dogra
    const TAG_DUPL: u32 = tag!(b"dupl"); // Duployan
    const TAG_EGYP: u32 = tag!(b"egyp"); // Egyptian Hieroglyphs
    const TAG_ELBA: u32 = tag!(b"elba"); // Elbasan
    const TAG_ELYM: u32 = tag!(b"elym"); // Elymaic
    const TAG_ETHI: u32 = tag!(b"ethi"); // Ethiopic
    const TAG_GEOR: u32 = tag!(b"geor"); // Georgian
    const TAG_GLAG: u32 = tag!(b"glag"); // Glagolitic
    const TAG_GOTH: u32 = tag!(b"goth"); // Gothic
    const TAG_GRAN: u32 = tag!(b"gran"); // Grantha
    const TAG_GREK: u32 = tag!(b"grek"); // Greek
    const TAG_GUJR: u32 = tag!(b"gujr"); // Gujarati
    const TAG_GJR2: u32 = tag!(b"gjr2"); // Gujarati v.2
    const TAG_GONG: u32 = tag!(b"gong"); // Gunjala Gondi
    const TAG_GURU: u32 = tag!(b"guru"); // Gurmukhi
    const TAG_GUR2: u32 = tag!(b"gur2"); // Gurmukhi v.2
    const TAG_HANG: u32 = tag!(b"hang"); // Hangul
    const TAG_JAMO: u32 = tag!(b"jamo"); // Hangul Jamo
    const TAG_ROHG: u32 = tag!(b"rohg"); // Hanifi Rohingya
    const TAG_HANO: u32 = tag!(b"hano"); // Hanunoo
    const TAG_HATR: u32 = tag!(b"hatr"); // Hatran
    const TAG_HEBR: u32 = tag!(b"hebr"); // Hebrew
    const TAG_HIRG: u32 = tag!(b"kana"); // Hiragana
    const TAG_ARMI: u32 = tag!(b"armi"); // Imperial Aramaic
    const TAG_PHLI: u32 = tag!(b"phli"); // Inscriptional Pahlavi
    const TAG_PRTI: u32 = tag!(b"prti"); // Inscriptional Parthian
    const TAG_JAVA: u32 = tag!(b"java"); // Javanese
    const TAG_KTHI: u32 = tag!(b"kthi"); // Kaithi
    const TAG_KNDA: u32 = tag!(b"knda"); // Kannada
    const TAG_KND2: u32 = tag!(b"knd2"); // Kannada v.2
    const TAG_KANA: u32 = tag!(b"kana"); // Katakana
    const TAG_KALI: u32 = tag!(b"kali"); // Kayah Li
    const TAG_KHAR: u32 = tag!(b"khar"); // Kharosthi
    const TAG_KITS: u32 = tag!(b"kits"); // Khitan Small Script
    const TAG_KHMR: u32 = tag!(b"khmr"); // Khmer
    const TAG_KHOJ: u32 = tag!(b"khoj"); // Khojki
    const TAG_SIND: u32 = tag!(b"sind"); // Khudawadi
    const TAG_LAO: u32 = tag!(b"lao "); // Lao
    const TAG_LATN: u32 = tag!(b"latn"); // Latin
    const TAG_LEPC: u32 = tag!(b"lepc"); // Lepcha
    const TAG_LIMB: u32 = tag!(b"limb"); // Limbu
    const TAG_LINA: u32 = tag!(b"lina"); // Linear A
    const TAG_LINB: u32 = tag!(b"linb"); // Linear B
    const TAG_LISU: u32 = tag!(b"lisu"); // Lisu (Fraser)
    const TAG_LYCI: u32 = tag!(b"lyci"); // Lycian
    const TAG_LYDI: u32 = tag!(b"lydi"); // Lydian
    const TAG_MAHJ: u32 = tag!(b"mahj"); // Mahajani
    const TAG_MAKA: u32 = tag!(b"maka"); // Makasar
    const TAG_MLYM: u32 = tag!(b"mlym"); // Malayalam
    const TAG_MLM2: u32 = tag!(b"mlm2"); // Malayalam v.2
    const TAG_MAND: u32 = tag!(b"mand"); // Mandaic, Mandaean
    const TAG_MANI: u32 = tag!(b"mani"); // Manichaean
    const TAG_MARC: u32 = tag!(b"marc"); // Marchen
    const TAG_GONM: u32 = tag!(b"gonm"); // Masaram Gondi
    const TAG_MATH: u32 = tag!(b"math"); // Mathematical Alphanumeric Symbols
    const TAG_MEDF: u32 = tag!(b"medf"); // Medefaidrin (Oberi Okaime, Oberi kaim)
    const TAG_MTEI: u32 = tag!(b"mtei"); // Meitei Mayek (Meithei, Meetei)
    const TAG_MEND: u32 = tag!(b"mend"); // Mende Kikakui
    const TAG_MERC: u32 = tag!(b"merc"); // Meroitic Cursive
    const TAG_MERO: u32 = tag!(b"mero"); // Meroitic Hieroglyphs
    const TAG_PLRD: u32 = tag!(b"plrd"); // Miao
    const TAG_MODI: u32 = tag!(b"modi"); // Modi
    const TAG_MONG: u32 = tag!(b"mong"); // Mongolian
    const TAG_MROO: u32 = tag!(b"mroo"); // Mro
    const TAG_MULT: u32 = tag!(b"mult"); // Multani
    const TAG_MUSC: u32 = tag!(b"musc"); // Musical Symbols
    const TAG_MYMR: u32 = tag!(b"mymr"); // Myanmar
    const TAG_MYM2: u32 = tag!(b"mym2"); // Myanmar v.2
    const TAG_NBAT: u32 = tag!(b"nbat"); // Nabataean
    const TAG_NAND: u32 = tag!(b"nand"); // Nandinagari
    const TAG_NEWA: u32 = tag!(b"newa"); // Newa
    const TAG_TALU: u32 = tag!(b"talu"); // New Tai Lue
    const TAG_NKO: u32 = tag!(b"nko "); // N'Ko
    const TAG_NSHU: u32 = tag!(b"nshu"); // Nüshu
    const TAG_HMNP: u32 = tag!(b"hmnp"); // Nyiakeng Puachue Hmong
    const TAG_ORYA: u32 = tag!(b"orya"); // Odia (formerly Oriya)
    const TAG_ORY2: u32 = tag!(b"ory2"); // Odia v.2 (formerly Oriya v.2)
    const TAG_OGAM: u32 = tag!(b"ogam"); // Ogham
    const TAG_OLCK: u32 = tag!(b"olck"); // Ol Chiki
    const TAG_ITAL: u32 = tag!(b"ital"); // Old Italic
    const TAG_HUNG: u32 = tag!(b"hung"); // Old Hungarian
    const TAG_NARB: u32 = tag!(b"narb"); // Old North Arabian
    const TAG_PERM: u32 = tag!(b"perm"); // Old Permic
    const TAG_XPEO: u32 = tag!(b"xpeo"); // Old Persian Cuneiform
    const TAG_SOGO: u32 = tag!(b"sogo"); // Old Sogdian
    const TAG_SARB: u32 = tag!(b"sarb"); // Old South Arabian
    const TAG_ORKH: u32 = tag!(b"orkh"); // Old Turkic, Orkhon Runic
    const TAG_OSGE: u32 = tag!(b"osge"); // Osage
    const TAG_OSMA: u32 = tag!(b"osma"); // Osmanya
    const TAG_HMNG: u32 = tag!(b"hmng"); // Pahawh Hmong
    const TAG_PALM: u32 = tag!(b"palm"); // Palmyrene
    const TAG_PAUC: u32 = tag!(b"pauc"); // Pau Cin Hau
    const TAG_PHAG: u32 = tag!(b"phag"); // Phags-pa
    const TAG_PHNX: u32 = tag!(b"phnx"); // Phoenician
    const TAG_PHLP: u32 = tag!(b"phlp"); // Psalter Pahlavi
    const TAG_RJNG: u32 = tag!(b"rjng"); // Rejang
    const TAG_RUNR: u32 = tag!(b"runr"); // Runic
    const TAG_SAMR: u32 = tag!(b"samr"); // Samaritan
    const TAG_SAUR: u32 = tag!(b"saur"); // Saurashtra
    const TAG_SHRD: u32 = tag!(b"shrd"); // Sharada
    const TAG_SHAW: u32 = tag!(b"shaw"); // Shavian
    const TAG_SIDD: u32 = tag!(b"sidd"); // Siddham
    const TAG_SGNW: u32 = tag!(b"sgnw"); // Sign Writing
    const TAG_SINH: u32 = tag!(b"sinh"); // Sinhala
    const TAG_SOGD: u32 = tag!(b"sogd"); // Sogdian
    const TAG_SORA: u32 = tag!(b"sora"); // Sora Sompeng
    const TAG_SOYO: u32 = tag!(b"soyo"); // Soyombo
    const TAG_XSUX: u32 = tag!(b"xsux"); // Sumero-Akkadian Cuneiform
    const TAG_SUND: u32 = tag!(b"sund"); // Sundanese
    const TAG_SYLO: u32 = tag!(b"sylo"); // Syloti Nagri
    const TAG_SYRC: u32 = tag!(b"syrc"); // Syriac
    const TAG_TGLG: u32 = tag!(b"tglg"); // Tagalog
    const TAG_TAGB: u32 = tag!(b"tagb"); // Tagbanwa
    const TAG_TALE: u32 = tag!(b"tale"); // Tai Le
    const TAG_LANA: u32 = tag!(b"lana"); // Tai Tham (Lanna)
    const TAG_TAVT: u32 = tag!(b"tavt"); // Tai Viet
    const TAG_TAKR: u32 = tag!(b"takr"); // Takri
    const TAG_TAML: u32 = tag!(b"taml"); // Tamil
    const TAG_TML2: u32 = tag!(b"tml2"); // Tamil v.2
    const TAG_TANG: u32 = tag!(b"tang"); // Tangut
    const TAG_TELU: u32 = tag!(b"telu"); // Telugu
    const TAG_TEL2: u32 = tag!(b"tel2"); // Telugu v.2
    const TAG_THAA: u32 = tag!(b"thaa"); // Thaana
    const TAG_THAI: u32 = tag!(b"thai"); // Thai
    const TAG_TIBT: u32 = tag!(b"tibt"); // Tibetan
    const TAG_TFNG: u32 = tag!(b"tfng"); // Tifinagh
    const TAG_TIRH: u32 = tag!(b"tirh"); // Tirhuta
    const TAG_UGAR: u32 = tag!(b"ugar"); // Ugaritic Cuneiform
    const TAG_VAI: u32 = tag!(b"vai "); // Vai
    const TAG_WCHO: u32 = tag!(b"wcho"); // Wancho
    const TAG_WARA: u32 = tag!(b"wara"); // Warang Citi
    const TAG_YEZI: u32 = tag!(b"yezi"); // Yezidi
    const TAG_ZANB: u32 = tag!(b"zanb"); // Zanabazar Square
    // missing: Yi

    // auto-detect script + language from text (todo: performance!)
    let (lang, script) = whatlang::detect(text)
        .map(|info| (info.lang(), info.script()))
        .unwrap_or((Lang::Eng, Script::Latin));

    let lang = tag::from_string(&lang.code().to_string().to_uppercase()).unwrap();

    let script = match script {
        Script::Arabic          => TAG_ARAB,
        Script::Bengali         => TAG_BENG,
        Script::Cyrillic        => TAG_CYRL,
        Script::Devanagari      => TAG_DEVA,
        Script::Ethiopic        => TAG_ETHI,
        Script::Georgian        => TAG_GEOR,
        Script::Greek           => TAG_GREK,
        Script::Gujarati        => TAG_GUJR,
        Script::Gurmukhi        => TAG_GUR2,
        Script::Hangul          => TAG_HANG,
        Script::Hebrew          => TAG_HEBR,
        Script::Hiragana        => TAG_HIRG, // NOTE: tag = 'kana', probably error
        Script::Kannada         => TAG_KND2,
        Script::Katakana        => TAG_KANA,
        Script::Khmer           => TAG_KHMR,
        Script::Latin           => TAG_LATN,
        Script::Malayalam       => TAG_MLYM,
        Script::Mandarin        => TAG_MAND,
        Script::Myanmar         => TAG_MYM2,
        Script::Oriya           => TAG_ORYA,
        Script::Sinhala         => TAG_SINH,
        Script::Tamil           => TAG_TAML,
        Script::Telugu          => TAG_TELU,
        Script::Thai            => TAG_THAI,
    };

    (script, lang)
}

// shape_word(text: &str, &font) -> TextBuffer
// get_word_visual_width(word: &TextBuffer) ->
// get_glyph_instances(infos: &GlyphInfos, positions: &GlyphPositions) -> PositionedGlyphBuffer

fn shape<'a>(font: &mut ParsedFont, text: &[char], script: u32, lang: u32) -> Option<ShapedTextBufferUnsized> {

    use std::convert::TryFrom;
    use allsorts::gpos::apply as gpos_apply;
    use allsorts::gsub::apply as gsub_apply;

    // Map glyphs
    //
    // We look ahead in the char stream for variation selectors. If one is found it is used for
    // mapping the current glyph. When a variation selector is reached in the stream it is skipped
    // as it was handled as part of the preceding character.
    let mut chars_iter = text.iter().peekable();
    let mut glyphs = Vec::new();

    while let Some(ch) = chars_iter.next() {
        match allsorts::unicode::VariationSelector::try_from(*ch) {
            Ok(_) => {} // filter out variation selectors
            Err(()) => {
                let vs = chars_iter
                    .peek()
                    .and_then(|&next| allsorts::unicode::VariationSelector::try_from(*next).ok());

                let glyph_index = font.lookup_glyph_index(*ch as u32);
                let glyph = make_raw_glyph(*ch, glyph_index, vs);
                glyphs.push(glyph);
            }
        }
    }

    const DOTTED_CIRCLE: char = '\u{25cc}';
    // TODO: Remove cast when lookup_glyph_index returns u16
    let dotted_circle_index = font.lookup_glyph_index(DOTTED_CIRCLE as u32);

    // Apply glyph substitution if table is present
    gsub_apply(
        dotted_circle_index,
        &font.gsub_cache,
        Some(Rc::as_ref(&font.gdef_table)),
        script,
        Some(lang),
        &allsorts::gsub::Features::Mask(allsorts::gsub::GsubFeatureMask::default()),
        font.num_glyphs,
        &mut glyphs,
    ).ok()?;

    // Apply glyph positioning if table is present

    let kerning = true;
    let mut infos = allsorts::gpos::Info::init_from_glyphs(Some(&font.gdef_table), glyphs);
    gpos_apply(
        &font.gpos_cache,
        Some(Rc::as_ref(&font.gdef_table)),
        kerning,
        script,
        Some(lang),
        &mut infos,
    ).ok()?;

    // calculate the horizontal advance for each char
    let infos = infos.iter().filter_map(|info| {
        let glyph_index = info.glyph.glyph_index;
        let (adv_x, adv_y) = font.get_advance(glyph_index);
        let advance = Advance { x: adv_x, y: adv_y, kerning: info.kerning };
        let info = translate_info(&info, advance);
        Some(info)
    }).collect();

    Some(ShapedTextBufferUnsized { infos })
}

fn make_raw_glyph(ch: char, glyph_index: u16, variation: Option<allsorts::unicode::VariationSelector>) -> allsorts::gsub::RawGlyph<()> {
    allsorts::gsub::RawGlyph {
        unicodes: tiny_vec![[char; 1] => ch],
        glyph_index: glyph_index,
        liga_component_pos: 0,
        glyph_origin: allsorts::gsub::GlyphOrigin::Char(ch),
        small_caps: false,
        multi_subst_dup: false,
        is_vert_alt: false,
        fake_bold: false,
        fake_italic: false,
        extra_data: (),
        variation,
    }
}

#[inline]
fn translate_info(i: &allsorts::gpos::Info, size: Advance) -> Info {
    Info {
        glyph: translate_raw_glyph(&i.glyph),
        size,
        placement: translate_placement(&i.placement),
        mark_placement: translate_mark_placement(&i.mark_placement),
        is_mark: i.is_mark,
    }
}

#[inline]
fn translate_raw_glyph(rg: &allsorts::gsub::RawGlyph<()>) -> RawGlyph {
    RawGlyph {
        unicodes: [rg.unicodes[0]],
        glyph_index: rg.glyph_index,
        liga_component_pos: rg.liga_component_pos,
        glyph_origin: translate_glyph_origin(&rg.glyph_origin),
        small_caps: rg.small_caps,
        multi_subst_dup: rg.multi_subst_dup,
        is_vert_alt: rg.is_vert_alt,
        fake_bold: rg.fake_bold,
        fake_italic: rg.fake_italic,
        variation: rg.variation.as_ref().map(translate_variation_selector),
        extra_data: (),
    }
}

#[inline]
fn translate_glyph_origin(g: &allsorts::gsub::GlyphOrigin) -> GlyphOrigin {
    use allsorts::gsub::GlyphOrigin::*;
    match g {
        Char(c) => GlyphOrigin::Char(*c),
        Direct => GlyphOrigin::Direct,
    }
}

#[inline]
fn translate_placement(p: &allsorts::gpos::Placement) -> Placement {
    use allsorts::gpos::Placement::*;
    match p {
        None => Placement::None,
        Distance(x, y) => Placement::Distance(*x, *y),
        Anchor(a, b) => Placement::Anchor(translate_anchor(a), translate_anchor(b)),
    }
}

#[inline]
fn translate_mark_placement(mp: &allsorts::gpos::MarkPlacement) -> MarkPlacement {
    use allsorts::gpos::MarkPlacement::*;
    match mp {
        None => MarkPlacement::None,
        MarkAnchor(a, b, c) => MarkPlacement::MarkAnchor(*a, translate_anchor(b), translate_anchor(c)),
        MarkOverprint(a) => MarkPlacement::MarkOverprint(*a),
    }
}

fn translate_variation_selector(v: &allsorts::unicode::VariationSelector) -> VariationSelector {
    use allsorts::unicode::VariationSelector::*;
    match v {
        VS01 => VariationSelector::VS01,
        VS02 => VariationSelector::VS02,
        VS03 => VariationSelector::VS03,
        VS15 => VariationSelector::VS15,
        VS16 => VariationSelector::VS16,
    }
}

#[inline]
fn translate_anchor(anchor: &allsorts::layout::Anchor) -> Anchor { Anchor { x: anchor.x, y: anchor.y } }