//! Finds private details in OCR words so they can be pixelated in one click:
//! email addresses, long digit runs (phone, card, account numbers and IP
//! addresses) and key-like tokens. Plain rules, no pattern engine.
use crate::ocr::Word;

/// A box to pixelate, in pixels of the image given to OCR.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Area {
    fn of(word: &Word) -> Self {
        Self {
            x: word.x,
            y: word.y,
            w: word.w,
            h: word.h,
        }
    }
    fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Self {
            x,
            y,
            w: (self.x + self.w).max(other.x + other.w) - x,
            h: (self.y + self.h).max(other.y + other.h) - y,
        }
    }
    /// Grown by `pad` on every side and kept inside a `width` by `height` image.
    pub fn padded(self, pad: f32, width: f32, height: f32) -> Self {
        let x = (self.x - pad).max(0.);
        let y = (self.y - pad).max(0.);
        Self {
            x,
            y,
            w: (self.x + self.w + pad).min(width) - x,
            h: (self.y + self.h + pad).min(height) - y,
        }
    }
}

/// Surrounding punctuation OCR keeps on a word: quotes, brackets, commas and
/// a sentence's closing full stop.
fn core(text: &str) -> &str {
    text.trim_matches(|c: char| "\"'`<>[](){},;:!?.".contains(c))
}

/// Pieces of a word between separators, so a token inside a link
/// ("?token=..."), a quoted setting (KEY="...") or an address with a port
/// ("203.0.113.7:8080") is tested on its own.
fn segments(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| ":?&\"'=/\\<>()[]{};".contains(c))
        .map(core)
        .filter(|s| !s.is_empty())
}

fn is_email(text: &str) -> bool {
    let text = core(text);
    let Some((user, domain)) = text.split_once('@') else {
        return false;
    };
    let Some((name, tld)) = domain.rsplit_once('.') else {
        return false;
    };
    let label = name.rsplit('.').next().unwrap_or(name);
    !user.is_empty()
        && label
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && tld.len() >= 2
        && tld.chars().all(|c| c.is_ascii_alphabetic())
}

/// API keys, tokens and hashes: long runs of key characters mixing letters
/// and digits, or a well-known key prefix.
fn is_key(text: &str) -> bool {
    let text = core(text);
    let key_chars = text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_+/=.".contains(c));
    if !key_chars {
        return false;
    }
    const PREFIXES: [&str; 8] = [
        "sk-",
        "ghp_",
        "gho_",
        "github_pat_",
        "xox",
        "AKIA",
        "AIza",
        "eyJ",
    ];
    if text.len() >= 16 && PREFIXES.iter().any(|p| text.starts_with(p)) {
        return true;
    }
    text.len() >= 20
        && text.chars().filter(|c| "-_+/=.".contains(*c)).count() <= 2
        && text.chars().any(|c| c.is_ascii_digit())
        && text.chars().any(|c| c.is_ascii_alphabetic())
}

/// A word that can be part of a phone, card or account number or an IP
/// address: digits and separators only, with at least one digit.
fn number_part(text: &str) -> Option<usize> {
    let text = core(text);
    let ok = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || "+()-./#".contains(c));
    let digits = text.chars().filter(char::is_ascii_digit).count();
    (ok && digits > 0).then_some(digits)
}

/// Dates such as 2026-10-14 or 14/10/2026 are not private: a four-digit
/// year first or last, and a month and day of one or two digits in range.
/// Phone numbers like 555-123-4567 do not fit that shape.
fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = core(text).split(['-', '/', '.']).collect();
    if parts.len() != 3
        || !parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    let (a, b) = match (parts[0].len(), parts[2].len()) {
        (4, 1..=2) => (parts[1], parts[2]),
        (1..=2, 4) => (parts[0], parts[1]),
        _ => return false,
    };
    if a.len() > 2 || b.len() > 2 {
        return false;
    }
    let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) else {
        return false;
    };
    (1..=31).contains(&a) && (1..=31).contains(&b) && (a <= 12 || b <= 12)
}

/// Money is not private: a currency sign, or thousands grouped by commas
/// ("1,234,567.89").
fn is_amount(text: &str) -> bool {
    let text = core(text);
    if text.contains(['$', '€', '£', '¥']) {
        return true;
    }
    // Drop a decimal part of one or two digits after the last . or , then
    // the rest must be thousands groups split by one kind of separator.
    let whole = match text.rfind(['.', ',']) {
        Some(i) if (1..=2).contains(&(text.len() - i - 1)) => &text[..i],
        _ => text,
    };
    [',', '.'].iter().any(|sep| {
        let groups: Vec<&str> = whole.split(*sep).collect();
        groups.len() >= 2
            && (1..=3).contains(&groups[0].len())
            && groups[1..].iter().all(|g| g.len() == 3)
            && groups.iter().all(|g| g.chars().all(|c| c.is_ascii_digit()))
    })
}

/// Timestamps ("2026-10-14T22") and sizes ("1920x1080") are not private.
fn is_time_or_size(text: &str) -> bool {
    if let Some((date, _)) = text.split_once(['T', 't']) {
        if is_date(date) {
            return true;
        }
    }
    text.split_once(['x', 'X', '\u{d7}']).is_some_and(|(a, b)| {
        !a.is_empty()
            && !b.is_empty()
            && a.chars().all(|c| c.is_ascii_digit())
            && b.chars().all(|c| c.is_ascii_digit())
    })
}

/// Account numbers, references and IDs in one piece: seven or more digits,
/// alone or with a short letter prefix ("ACC12345678", "INV-2026-0001234").
fn is_long_number(text: &str) -> bool {
    let text = core(text);
    if is_date(text) || is_amount(text) || is_time_or_size(text) {
        return false;
    }
    let digits = text.chars().filter(char::is_ascii_digit).count();
    let alnum = text.chars().filter(char::is_ascii_alphanumeric).count();
    digits >= 7
        && digits * 2 >= alnum
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+()-./#_".contains(c))
}

/// A word that is private on its own (not as part of a number run).
fn private_word(text: &str) -> bool {
    is_email(text) || segments(text).any(|s| is_email(s) || is_key(s) || is_long_number(s))
}

/// A number run in progress: line, covered area, digits, words, and the
/// most digits in any one word.
#[derive(Clone, Copy)]
struct Run {
    line: usize,
    area: Area,
    digits: usize,
    words: usize,
    longest: usize,
    /// Every part looked like a year (1900 to 2099).
    years: bool,
}

/// Areas to pixelate. Number runs may span several words on one line
/// ("(555) 123-4567", "4111 1111 1111 1111") and count from 7 digits; a
/// run of single digits ("1 2 3 4 5 6 7", page numbers) does not count.
pub fn private_areas(words: &[Word]) -> Vec<Area> {
    let mut areas = Vec::new();
    let mut run: Option<Run> = None;
    let flush = |run: &mut Option<Run>, areas: &mut Vec<Area>| {
        if let Some(r) = run.take() {
            if r.digits >= 7 && !r.years && (r.words == 1 || r.longest >= 4) {
                areas.push(r.area);
            }
        }
    };
    for word in words {
        let number =
            number_part(&word.text).filter(|_| !is_date(&word.text) && !is_amount(&word.text));
        if let Some(digits) = number {
            // A gap wider than about one character starts a separate
            // number, such as the next column of a table. OCR boxes hug the
            // digits, so one space can be most of a box height.
            if run.is_some_and(|r| r.line != word.line || word.x - (r.area.x + r.area.w) > word.h) {
                flush(&mut run, &mut areas);
            }
            let year = core(&word.text)
                .parse::<u32>()
                .is_ok_and(|y| (1900..=2099).contains(&y));
            run = Some(match run {
                Some(r) => Run {
                    area: r.area.union(Area::of(word)),
                    digits: r.digits + digits,
                    words: r.words + 1,
                    longest: r.longest.max(digits),
                    years: r.years && year,
                    ..r
                },
                None => Run {
                    line: word.line,
                    area: Area::of(word),
                    digits,
                    words: 1,
                    longest: digits,
                    years: year,
                },
            });
            // A comma or semicolon ends a list item, not a phone number group.
            if word.text.trim_end().ends_with([',', ';']) {
                flush(&mut run, &mut areas);
            }
            continue;
        }
        flush(&mut run, &mut areas);
        if private_word(&word.text) {
            areas.push(Area::of(word));
        }
    }
    flush(&mut run, &mut areas);
    areas
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Words 95 pixels wide, 100 apart: a 5 pixel gap, like one space.
    fn line(n: usize, texts: &[&str]) -> Vec<Word> {
        texts
            .iter()
            .enumerate()
            .map(|(i, t)| Word {
                text: t.to_string(),
                line: n,
                x: i as f32 * 100.,
                y: n as f32 * 20.,
                w: 95.,
                h: 16.,
            })
            .collect()
    }

    fn found(words: &[Word]) -> Vec<(f32, f32)> {
        private_areas(words).iter().map(|a| (a.x, a.w)).collect()
    }

    #[test]
    fn finds_emails_keys_and_number_runs() {
        let second = [(100., 95.)];
        for words in [
            ["Mail", "jane.doe@example.co.uk,"],
            ["key:", "sk-proj-abc123DEF456ghi789"],
            ["token", "9f86d081884c7d659a2feaa0c55ad015"],
            ["Server", "192.168.1.100"],
            ["Tel", "555-123-4567"],
            ["Tel", "555.123.4567"],
            ["Ask", "(alice@example.com)"],
            ["Ask", "(support@example.com)."],
            [
                "Reset",
                "https://app.example.com/reset?token=8f7d6c5b4a39281716f5e4d3c2b1a099",
            ],
            ["Set", "API_KEY=\"Zk3a9fQ2xL7mP0vB8nR4tY6u\""],
            ["Host", "203.0.113.7:8080"],
            ["Account", "ACC12345678"],
            ["Invoice", "INV-2026-0001234"],
            ["Ref", "12345678901234567890"],
        ] {
            assert_eq!(found(&line(0, &words)), second, "{words:?}");
        }
        // A number split over words is one area spanning them, whatever its length.
        assert_eq!(
            found(&line(0, &["Call", "(555)", "123-4567", "now"])),
            [(100., 195.)]
        );
        assert_eq!(
            found(&line(0, &["4111", "1111", "1111", "1111"])),
            [(0., 395.)]
        );
        assert_eq!(
            found(&line(0, &["1234", "5678", "9012", "3456", "7890"])),
            [(0., 495.)]
        );
        assert_eq!(
            found(&line(0, &["+44", "20", "7946", "0958"])),
            [(0., 395.)]
        );
        assert_eq!(
            found(&line(0, &["555-123-4567", "555-987-6543"])),
            [(0., 195.)]
        );
    }

    #[test]
    fn leaves_ordinary_text_dates_amounts_and_short_numbers() {
        for words in [
            line(0, &["Meeting", "on", "2026-10-14", "at", "10"]),
            line(0, &["Due", "14/10/2026", "or", "10.14.2026"]),
            line(0, &["Released", "2026-10-14."]),
            line(0, &["Version", "0.3.26", "preview"]),
            line(0, &["Total:", "$1,234"]),
            line(0, &["Total:", "$1,234,567.89"]),
            line(0, &["Revenue", "1,234,567"]),
            line(0, &["Pages", "1", "2", "3", "4", "5", "6", "7"]),
            line(0, &["At", "10:45:30"]),
            line(0, &["https://example.com/docs/page"]),
            line(
                0,
                &["https://site.com/blog/10-best-practices-for-cloud-security-in-2026"],
            ),
            line(0, &["Logged", "2026-10-14T22:11:13Z"]),
            line(0, &["In", "2019,", "2020,", "2021"]),
            line(0, &["FY", "2023", "2024"]),
            line(0, &["Scores", "100", "250", "300"]),
            line(0, &["Summe", "1.234.567,89"]),
            line(0, &["Display", "1920x1080"]),
            line(0, &["File", "logo@2x.png"]),
            line(0, &["supercalifragilisticexpialidocious"]),
            line(0, &["user@localhost"]),
        ] {
            assert!(private_areas(&words).is_empty(), "{words:?}");
        }
    }

    #[test]
    fn monospace_spacing_still_joins_a_card_number() {
        // Wider gaps (14 px at 16 px height), as in a terminal font.
        let mut words = line(0, &["4111", "1111", "1111", "1111"]);
        for w in &mut words {
            w.w = 86.;
        }
        assert_eq!(found(&words), [(0., 386.)]);
    }

    #[test]
    fn number_runs_stop_at_line_breaks_and_column_gaps() {
        let mut words = line(0, &["555"]);
        words.extend(line(1, &["1234"]));
        assert!(private_areas(&words).is_empty());
        // Table columns far apart are separate small numbers.
        let mut columns = line(0, &["150", "200", "450"]);
        for (i, w) in columns.iter_mut().enumerate() {
            w.x = i as f32 * 200.;
        }
        assert!(private_areas(&columns).is_empty());
    }

    #[test]
    fn padding_stays_inside_the_image() {
        let a = Area {
            x: 1.,
            y: 2.,
            w: 10.,
            h: 5.,
        };
        assert_eq!(
            a.padded(3., 12., 100.),
            Area {
                x: 0.,
                y: 0.,
                w: 12.,
                h: 10.,
            }
        );
    }
}
