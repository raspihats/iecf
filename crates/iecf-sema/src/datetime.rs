//! DATE, TOD and DT literals (Ed.3 6.3.5), read into iecf's
//! representation (docs/dependencies.md): a DATE and a DT in seconds
//! since 1970-01-01, a TOD in milliseconds since midnight. The checker
//! refuses one that names no day or time, or that the type cannot hold
//! (E0020); the generator writes the value.

use crate::types::Elem;

/// The value of a DATE, TOD or DT literal, `None` for the other types'
/// (durations, the L types), `Some(Err)` with the reason for one iecf
/// cannot hold.
pub fn literal(text: &str) -> Option<Result<(Elem, i64), String>> {
    let (prefix, body) = text.split_once('#')?;
    let ty = match prefix.to_ascii_uppercase().as_str() {
        "D" | "DATE" => Elem::Date,
        "TOD" | "TIME_OF_DAY" => Elem::Tod,
        "DT" | "DATE_AND_TIME" => Elem::Dt,
        _ => return None,
    };
    let body = body.replace('_', "");
    Some(value(ty, &body).map(|v| (ty, v)))
}

fn value(ty: Elem, body: &str) -> Result<i64, String> {
    let v = match ty {
        Elem::Date => date(body)? * 86_400,
        Elem::Tod => {
            let (ms, frac) = tod(body)?;
            if frac {
                return Err("a TOD counts whole milliseconds (docs/dependencies.md)".into());
            }
            ms
        }
        _ => {
            // DT#2026-11-07-12:34:56: the date's three parts, then the time.
            let mut dashes = body.match_indices('-').map(|(i, _)| i);
            let Some(at) = dashes.nth(2) else { return Err(format!("{body} is no date and time")) };
            let (ms, frac) = tod(&body[at + 1..])?;
            if frac || ms % 1000 != 0 {
                return Err("a DT counts whole seconds (docs/dependencies.md)".into());
            }
            date(&body[..at])? * 86_400 + ms / 1000
        }
    };
    if !(0..=u32::MAX as i64).contains(&v) {
        let (first, last) = match ty {
            Elem::Date => ("D#1970-01-01", "D#2106-02-07"),
            _ => ("DT#1970-01-01-00:00:00", "DT#2106-02-07-06:28:15"),
        };
        return Err(format!("{} holds {first} to {last} (docs/dependencies.md)", ty.name()));
    }
    Ok(v)
}

/// Days since 1970-01-01 of `Y-M-D`, a day of the proleptic Gregorian
/// calendar.
fn date(s: &str) -> Result<i64, String> {
    let bad = || format!("{s} is no date");
    let mut parts = s.split('-');
    let (Some(y), Some(m), Some(d), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(bad());
    };
    let num = |p: &str| p.parse::<i64>().ok().filter(|_| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    let (Some(y), Some(m), Some(d)) = (num(y), num(m), num(d)) else { return Err(bad()) };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let len = match m {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return Err(bad()),
    };
    if !(1..=len).contains(&d) {
        return Err(bad());
    }
    // H. Hinnant's days_from_civil, as runtime/iecf.h's iecf_days.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146_097 + doe - 719_468)
}

/// Milliseconds since midnight of `h:m:s[.f]`, and whether `f` goes past
/// the milliseconds.
fn tod(s: &str) -> Result<(i64, bool), String> {
    let bad = || format!("{s} is no time of day");
    let (hms, frac) = s.split_once('.').unwrap_or((s, ""));
    let mut parts = hms.split(':');
    let (Some(h), Some(m), Some(sec), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(bad());
    };
    let num = |p: &str| p.parse::<i64>().ok().filter(|_| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    let (Some(h), Some(m), Some(sec)) = (num(h), num(m), num(sec)) else { return Err(bad()) };
    if h > 23 || m > 59 || sec > 59 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let mut ms = 0;
    for (i, b) in frac.bytes().take(3).enumerate() {
        ms += i64::from(b - b'0') * [100, 10, 1][i];
    }
    let past = frac.bytes().skip(3).any(|b| b != b'0');
    Ok((((h * 60 + m) * 60 + sec) * 1000 + ms, past))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values() {
        assert_eq!(literal("D#2026-11-07"), Some(Ok((Elem::Date, 1_794_009_600))));
        assert_eq!(literal("date#1970-01-01"), Some(Ok((Elem::Date, 0))));
        assert_eq!(literal("TOD#12:34:56.007"), Some(Ok((Elem::Tod, 45_296_007))));
        assert_eq!(literal("TIME_OF_DAY#23:59:59.9990"), Some(Ok((Elem::Tod, 86_399_999))));
        assert_eq!(literal("DT#2026-11-07-12:34:56"), Some(Ok((Elem::Dt, 1_794_009_600 + 45_296))));
        assert_eq!(literal("DT#2106-02-07-06:28:15"), Some(Ok((Elem::Dt, u32::MAX as i64))));
        assert_eq!(literal("D#2024-02-29"), Some(Ok((Elem::Date, 1_709_164_800))));
        assert_eq!(literal("T#5s"), None);
        assert_eq!(literal("LDT#2026-11-07-00:00:00"), None);
    }

    #[test]
    fn refused() {
        for bad in [
            "D#2023-02-29",
            "D#2026-13-01",
            "D#1969-12-31",
            "D#2106-02-08",
            "TOD#24:00:00",
            "TOD#12:00",
            "TOD#12:00:00.0005",
            "DT#2026-11-07-12:34:56.5",
            "DT#2106-02-07-06:28:16",
            "DT#2026-11-07",
        ] {
            assert!(matches!(literal(bad), Some(Err(_))), "{bad}");
        }
    }
}
