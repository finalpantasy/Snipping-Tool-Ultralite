use std::borrow::Cow;
use std::collections::BTreeSet;
use std::sync::{Arc, LazyLock, OnceLock, RwLock};

/// Open Sans, the one typeface for the interface and text annotations.
pub(crate) static OPEN_SANS: &[u8] = include_bytes!("../assets/font.ttf");

/// GSUB preloading only helps indexed rasterization, which UL never uses.
/// Skipping it renders identically with less memory.
pub(crate) const SETTINGS: fontdue::FontSettings = fontdue::FontSettings {
    collection_index: 0,
    scale: 40.0,
    load_substitutions: false,
};

/// Characters are loaded in aligned blocks of this many code points.
const BLOCK: u32 = 128;

/// A fontdue font that outlines only the characters UL actually draws.
///
/// fontdue precomputes geometry for every glyph in the character map when it
/// parses a font. This keeps the original bytes and
/// hands fontdue a copy whose character map lists only the blocks seen so far.
/// Glyph outlines and every other table are untouched, and the character map
/// is resolved exactly as fontdue resolves it, so metrics and coverage are
/// bit-identical to a full parse.
pub struct LazyFont {
    data: Cow<'static, [u8]>,
    /// Final code point to glyph mapping, sorted by code point.
    map: Vec<(u32, u16)>,
    loaded: RwLock<Loaded>,
}
struct Loaded {
    font: Arc<fontdue::Font>,
    blocks: BTreeSet<u32>,
}

impl LazyFont {
    /// Parses `data` and preloads Basic Latin and Latin-1.
    pub fn new(data: impl Into<Cow<'static, [u8]>>) -> Option<Self> {
        let data = data.into();
        let map = char_map(&data)?;
        let blocks: BTreeSet<u32> = [0, 1].into();
        let font = Arc::new(build(&data, &map, &blocks)?);
        Some(Self {
            data,
            map,
            loaded: RwLock::new(Loaded { font, blocks }),
        })
    }

    fn mapped(&self, cp: u32) -> bool {
        self.map.binary_search_by_key(&cp, |&(c, _)| c).is_ok()
    }

    /// A parsed font covering every character in `text`.
    pub fn for_text(&self, text: &str) -> Arc<fontdue::Font> {
        let missing = |loaded: &Loaded| {
            text.chars()
                .map(|ch| ch as u32)
                .filter(|cp| !loaded.blocks.contains(&(cp / BLOCK)) && self.mapped(*cp))
                .map(|cp| cp / BLOCK)
                .collect::<BTreeSet<u32>>()
        };
        {
            let loaded = self.loaded.read().unwrap_or_else(|e| e.into_inner());
            if missing(&loaded).is_empty() {
                return loaded.font.clone();
            }
        }
        let mut loaded = self.loaded.write().unwrap_or_else(|e| e.into_inner());
        let add = missing(&loaded);
        if !add.is_empty() {
            let mut blocks = loaded.blocks.clone();
            blocks.extend(add);
            if let Some(font) = build(&self.data, &self.map, &blocks) {
                loaded.font = Arc::new(font);
                loaded.blocks = blocks;
            }
        }
        loaded.font.clone()
    }

    pub fn metrics(&self, ch: char, px: f32) -> fontdue::Metrics {
        self.for_text(ch.encode_utf8(&mut [0; 4])).metrics(ch, px)
    }

    pub fn rasterize(&self, ch: char, px: f32) -> (fontdue::Metrics, Vec<u8>) {
        self.for_text(ch.encode_utf8(&mut [0; 4])).rasterize(ch, px)
    }

    pub fn horizontal_line_metrics(&self, px: f32) -> Option<fontdue::LineMetrics> {
        self.for_text("").horizontal_line_metrics(px)
    }
}

/// Resolve the character map the way `fontdue::Font::from_bytes` does: every
/// subtable in order, later mappings replacing earlier ones, glyph 0 skipped.
fn char_map(data: &[u8]) -> Option<Vec<(u32, u16)>> {
    let face = ttf_parser::Face::parse(data, SETTINGS.collection_index).ok()?;
    let mut map = std::collections::HashMap::new();
    for subtable in face.tables().cmap?.subtables {
        subtable.codepoints(|cp| {
            if let Some(glyph) = subtable.glyph_index(cp) {
                if glyph.0 != 0 && char::from_u32(cp).is_some() {
                    map.insert(cp, glyph.0);
                }
            }
        });
    }
    let mut map: Vec<(u32, u16)> = map.into_iter().collect();
    map.sort_unstable();
    Some(map)
}

/// Parse a copy of `data` whose cmap holds only `blocks`, as one format 12
/// subtable appended after the original tables.
fn build(data: &[u8], map: &[(u32, u16)], blocks: &BTreeSet<u32>) -> Option<fontdue::Font> {
    let be16 = |at: usize| Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?));
    let be32 = |at: usize| Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?));
    // A collection's first font keeps its table directory where the header
    // says; its table offsets still count from the start of the file.
    let directory = if data.get(0..4) == Some(b"ttcf".as_slice()) {
        be32(12)? as usize
    } else {
        0
    };
    let tables = be16(directory + 4)? as usize;
    let record = (0..tables)
        .map(|i| directory + 12 + i * 16)
        .find(|&at| data.get(at..at + 4) == Some(b"cmap".as_slice()))?;

    let mut groups: Vec<(u32, u32, u32)> = Vec::new();
    for &(cp, glyph) in map.iter().filter(|(cp, _)| blocks.contains(&(cp / BLOCK))) {
        match groups.last_mut() {
            Some((start, end, first_glyph))
                if *end + 1 == cp && *first_glyph + (cp - *start) == glyph as u32 =>
            {
                *end = cp
            }
            _ => groups.push((cp, cp, glyph as u32)),
        }
    }

    let mut out = Vec::with_capacity(data.len() + 32 + groups.len() * 12);
    out.extend_from_slice(data);
    out.resize(out.len().next_multiple_of(4), 0);
    let offset = out.len() as u32;
    let subtable_len = 16 + groups.len() as u32 * 12;
    out.extend_from_slice(&0u16.to_be_bytes()); // cmap version
    out.extend_from_slice(&1u16.to_be_bytes()); // one encoding record
    out.extend_from_slice(&3u16.to_be_bytes()); // Windows
    out.extend_from_slice(&10u16.to_be_bytes()); // Unicode full repertoire
    out.extend_from_slice(&12u32.to_be_bytes()); // subtable offset
    out.extend_from_slice(&12u16.to_be_bytes()); // format 12
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&subtable_len.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes()); // language
    out.extend_from_slice(&(groups.len() as u32).to_be_bytes());
    for (start, end, glyph) in &groups {
        out.extend_from_slice(&start.to_be_bytes());
        out.extend_from_slice(&end.to_be_bytes());
        out.extend_from_slice(&glyph.to_be_bytes());
    }
    let length = out.len() as u32 - offset;
    out[record + 8..record + 12].copy_from_slice(&offset.to_be_bytes());
    out[record + 12..record + 16].copy_from_slice(&length.to_be_bytes());
    fontdue::Font::from_bytes(out, SETTINGS).ok()
}

pub static FONT: LazyLock<LazyFont> =
    LazyLock::new(|| LazyFont::new(OPEN_SANS).expect("Failed to load embedded font"));

/// A Windows font that is read from disk only once text needs a character
/// Open Sans lacks, such as Japanese, Chinese or Korean typed through an IME.
pub struct Fallback {
    path: std::path::PathBuf,
    font: OnceLock<Option<LazyFont>>,
}

impl Fallback {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            font: OnceLock::new(),
        }
    }

    fn get(&self) -> Option<&LazyFont> {
        self.font
            .get_or_init(|| {
                let font = std::fs::read(&self.path).ok().and_then(LazyFont::new);
                if font.is_none() {
                    tracing::debug!("Fallback font unavailable: {}", self.path.display());
                }
                font
            })
            .as_ref()
    }
}

/// Han characters look different in Japanese, Chinese and Korean, so the
/// system language picks which font draws them first.
fn fallback_order(locale: &str) -> [&'static str; 4] {
    let locale = locale.to_ascii_lowercase();
    if locale.starts_with("zh") {
        ["msyh.ttc", "YuGothM.ttc", "malgun.ttf", "seguisym.ttf"]
    } else if locale.starts_with("ko") {
        ["malgun.ttf", "YuGothM.ttc", "msyh.ttc", "seguisym.ttf"]
    } else {
        ["YuGothM.ttc", "msyh.ttc", "malgun.ttf", "seguisym.ttf"]
    }
}

static FALLBACKS: LazyLock<Vec<Fallback>> = LazyLock::new(|| {
    let fonts = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:\\Windows".into())
        .join("Fonts");
    fallback_order(&user_locale())
        .iter()
        .map(|name| Fallback::new(fonts.join(name)))
        .collect()
});

#[cfg(target_os = "windows")]
fn user_locale() -> String {
    let mut name = [0u16; 85];
    let len = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut name) };
    String::from_utf16_lossy(&name[..(len.max(1) as usize - 1).min(name.len())])
}

#[cfg(not(target_os = "windows"))]
fn user_locale() -> String {
    std::env::var("LANG").unwrap_or_default()
}

/// The fonts that draw one piece of text: Open Sans, plus any fallback that
/// supplies characters Open Sans lacks. Same methods as a single font.
pub struct TextFonts {
    primary: Arc<fontdue::Font>,
    extra: Vec<(&'static LazyFont, Arc<fontdue::Font>)>,
}

impl TextFonts {
    fn pick(&self, ch: char) -> &fontdue::Font {
        if !self.extra.is_empty() && !FONT.mapped(ch as u32) {
            if let Some((_, font)) = self.extra.iter().find(|(lazy, _)| lazy.mapped(ch as u32)) {
                return font;
            }
        }
        &self.primary
    }

    pub fn metrics(&self, ch: char, px: f32) -> fontdue::Metrics {
        self.pick(ch).metrics(ch, px)
    }

    pub fn rasterize(&self, ch: char, px: f32) -> (fontdue::Metrics, Vec<u8>) {
        self.pick(ch).rasterize(ch, px)
    }

    /// Line metrics stay Open Sans's so a line never shifts when a fallback
    /// character is typed into it.
    pub fn horizontal_line_metrics(&self, px: f32) -> Option<fontdue::LineMetrics> {
        self.primary.horizontal_line_metrics(px)
    }
}

pub fn text_fonts(text: &str) -> TextFonts {
    text_fonts_from(text, &FALLBACKS)
}

pub fn text_fonts_from(text: &str, fallbacks: &'static [Fallback]) -> TextFonts {
    let primary = FONT.for_text(text);
    let mut missing: Vec<char> = text
        .chars()
        // Whitespace counts too: an IME's full-width space (U+3000) is not in Open Sans.
        .filter(|ch| !ch.is_control() && !FONT.mapped(*ch as u32))
        .collect();
    let mut extra = Vec::new();
    for fallback in fallbacks {
        if missing.is_empty() {
            break;
        }
        let Some(lazy) = fallback.get() else { continue };
        let (mine, rest): (Vec<char>, Vec<char>) =
            missing.iter().partition(|ch| lazy.mapped(**ch as u32));
        if !mine.is_empty() {
            extra.push((lazy, lazy.for_text(&mine.iter().collect::<String>())));
        }
        missing = rest;
    }
    TextFonts { primary, extra }
}
