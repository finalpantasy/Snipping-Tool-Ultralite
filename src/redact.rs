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

/// Surrounding punctuation OCR keeps on a word: quotes, brackets, commas.
fn core(text: &str) -> &str {
    text.trim_matches(|c: char| "\"'`<>[](){},;:!?".contains(c))
}

fn is_email(text: &str) -> bool {
    let text = core(text).trim_end_matches('.');
    let Some((user, domain)) = text.split_once('@') else {
        return false;
    };
    let Some((name, tld)) = domain.rsplit_once('.') else {
        return false;
    };
    !user.is_empty()
        && !name.is_empty()
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

/// Areas to pixelate. Number runs may span several words on one line
/// ("(555) 123-4567", "4111 1111 1111 1111") and count from 7 digits.
pub fn private_areas(words: &[Word]) -> Vec<Area> {
    let mut areas = Vec::new();
    let mut run: Option<(usize, Area, usize)> = None; // line, area, digits
    let flush = |run: &mut Option<(usize, Area, usize)>, areas: &mut Vec<Area>| {
        if let Some((_, area, digits)) = run.take() {
            if (7..=19).contains(&digits) {
                areas.push(area);
            }
        }
    };
    for word in words {
        if run.is_some_and(|(line, _, _)| line != word.line) {
            flush(&mut run, &mut areas);
        }
        if is_email(&word.text) || is_key(&word.text) {
            flush(&mut run, &mut areas);
            areas.push(Area::of(word));
            continue;
        }
        match number_part(&word.text).filter(|_| !is_date(&word.text)) {
            Some(digits) => {
                // A wide gap or too many digits means a separate number,
                // such as two phone numbers side by side.
                if run.is_some_and(|(_, area, total)| {
                    total + digits > 19 || word.x - (area.x + area.w) > word.h * 1.5
                }) {
                    flush(&mut run, &mut areas);
                }
                run = Some(match run {
                    Some((line, area, total)) => (line, area.union(Area::of(word)), total + digits),
                    None => (word.line, Area::of(word), digits),
                });
            }
            None => flush(&mut run, &mut areas),
        }
    }
    flush(&mut run, &mut areas);
    areas
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: usize, texts: &[&str]) -> Vec<Word> {
        texts
            .iter()
            .enumerate()
            .map(|(i, t)| Word {
                text: t.to_string(),
                line: n,
                x: i as f32 * 100.,
                y: n as f32 * 20.,
                w: 90.,
                h: 16.,
            })
            .collect()
    }

    fn found(words: &[Word]) -> Vec<(f32, f32)> {
        private_areas(words).iter().map(|a| (a.x, a.w)).collect()
    }

    #[test]
    fn finds_emails_keys_and_number_runs() {
        assert_eq!(
            found(&line(0, &["Mail", "jane.doe@example.co.uk,"])),
            [(100., 90.)]
        );
        assert_eq!(
            found(&line(0, &["key:", "sk-proj-abc123DEF456ghi789"])),
            [(100., 90.)]
        );
        assert_eq!(
            found(&line(0, &["token", "9f86d081884c7d659a2feaa0c55ad015"])),
            [(100., 90.)]
        );
        // A phone number split over words is one area spanning them.
        assert_eq!(
            found(&line(0, &["Call", "(555)", "123-4567", "now"])),
            [(100., 190.)]
        );
        assert_eq!(
            found(&line(0, &["4111", "1111", "1111", "1111"])),
            [(0., 390.)]
        );
        assert_eq!(found(&line(0, &["Server", "192.168.1.100"])), [(100., 90.)]);
        // Two numbers side by side are two areas, not one rejected run.
        assert_eq!(
            found(&line(0, &["555-123-4567", "555-987-6543"])),
            [(0., 90.), (100., 90.)]
        );
        // Brackets around an address or key do not hide it.
        assert_eq!(
            found(&line(0, &["Ask", "(alice@example.com)"])),
            [(100., 90.)]
        );
        // Dashed and dotted phone numbers are not dates.
        assert_eq!(found(&line(0, &["Tel", "555-123-4567"])), [(100., 90.)]);
        assert_eq!(found(&line(0, &["Tel", "555.123.4567"])), [(100., 90.)]);
    }

    #[test]
    fn leaves_ordinary_text_dates_and_short_numbers() {
        for words in [
            line(0, &["Meeting", "on", "2026-10-14", "at", "10"]),
            line(0, &["Due", "14/10/2026", "or", "10.14.2026"]),
            line(0, &["Version", "0.3.26", "preview"]),
            line(0, &["Total:", "$1,234"]),
            line(0, &["https://example.com/docs/page"]),
            line(0, &["supercalifragilisticexpialidocious"]),
            line(0, &["user@localhost"]),
        ] {
            assert!(private_areas(&words).is_empty(), "{words:?}");
        }
    }

    #[test]
    fn number_runs_stop_at_line_breaks() {
        let mut words = line(0, &["555"]);
        words.extend(line(1, &["1234"]));
        assert!(private_areas(&words).is_empty());
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
