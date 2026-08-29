//! `AbstractDatetime` family of `w:*` datatypes.
//!
//! Mirrors vnu's `nu.validator.datatype.AbstractDatetime` and its concrete
//! subclasses `Date`, `DatetimeLocal`, `DatetimeTz`, `Month`, `Time`,
//! `TimeDatetime`, `Week` (see `plan/05c-research-group-b.md`, items 1-7).
//! Per `plan/05c-datatype-library.md` ("Verbindliches Prinzip: vnu-Parität
//! als Default") the goal is vnu-parity, not an independent re-derivation
//! of the WHATWG date/time grammars — including a documented quirk where
//! `DatetimeTz.java`'s doc comment claims seconds are mandatory in the
//! timezone-qualified form while the actual regex (and thus the behaviour
//! implemented here) makes them optional, exactly like `DatetimeLocal`.
//!
//! All parsing is hand-written (no `regex` dependency), operating on
//! `&str`/`char` directly, in the style of a small recursive-descent lexer.

// ---------- shared helpers (mirrors vnu's `AbstractDatetime`) ----------

/// Proleptic Gregorian leap-year rule, exactly as used by vnu.
fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn days_in_month(year: u32, month: u32) -> u32 {
    if month == 2 && is_leap_year(year) {
        29
    } else {
        DAYS_IN_MONTH[(month - 1) as usize]
    }
}

/// Validates a parsed `(year, month, day)` triple. `month` must be 1-12,
/// `day` must be within the correct day count for `month`/`year` (leap
/// years accounted for), `year` must be >= 1.
fn check_date_parts(year: u32, month: u32, day: u32) -> Result<(), String> {
    if year < 1 {
        return Err(format!("invalid year {year}"));
    }
    // vnu's `AbstractDatetime.checkYear` (WARN-gated there, implemented as
    // a hard rejection here — same treatment as the timezone bounds
    // above): a year outside 1000..3000 is flagged as "may be mistyped".
    if !(1000..3000).contains(&year) {
        return Err(format!("year {year} may be mistyped (expected 1000-2999)"));
    }
    if !(1..=12).contains(&month) {
        return Err(format!("invalid month {month}"));
    }
    let max_day = days_in_month(year, month);
    if day < 1 || day > max_day {
        return Err(format!("invalid day {day} for {year:04}-{month:02}"));
    }
    Ok(())
}

/// Validates a parsed `(hour, minute, second)` triple. No leap seconds
/// anywhere in this family, matching vnu.
fn check_time_parts(hour: u32, minute: u32, second: Option<u32>) -> Result<(), String> {
    if hour > 23 {
        return Err(format!("invalid hour {hour}"));
    }
    if minute > 59 {
        return Err(format!("invalid minute {minute}"));
    }
    if let Some(s) = second
        && s > 59
    {
        return Err(format!("invalid second {s}"));
    }
    Ok(())
}

/// vnu's `isWhitespace` set: exactly these five ASCII characters, not
/// Rust's full-Unicode `char::is_whitespace()`.
fn is_vnu_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{0C}' | '\n' | '\r')
}

fn trim_vnu_whitespace(s: &str) -> &str {
    s.trim_matches(is_vnu_whitespace)
}

fn trim_vnu_whitespace_start(s: &str) -> &str {
    s.trim_start_matches(is_vnu_whitespace)
}

// ---------- low-level lexing primitives ----------

/// Consumes exactly `n` ASCII digits from the front of `s`, returning the
/// parsed value and the remainder. Fails if fewer than `n` digits are
/// available at the front (this is how fixed-width fields like `MM`/`DD`
/// are enforced: a run of more digits than expected leaves a digit in
/// front of the following literal separator, which then fails to match).
fn take_n_digits(s: &str, n: usize) -> Result<(u32, &str), String> {
    let mut chars = s.chars();
    let mut value: u32 = 0;
    for _ in 0..n {
        match chars.next() {
            Some(c) if c.is_ascii_digit() => {
                value = value * 10 + c.to_digit(10).expect("ascii digit");
            }
            _ => return Err(format!("expected {n} digits in {s:?}")),
        }
    }
    Ok((value, chars.as_str()))
}

/// Consumes a run of 4-or-more leading ASCII digits (a "year"), returning
/// the parsed value and the remainder.
fn take_year_digits(s: &str) -> Result<(u32, &str), String> {
    let digit_count = s.chars().take_while(char::is_ascii_digit).count();
    if digit_count < 4 {
        return Err(format!("expected a 4+ digit year in {s:?}"));
    }
    let (digits, rest) = s.split_at(digit_count);
    if digits.starts_with('0') {
        return Err(format!("year cannot start with leading zero: {digits:?}"));
    }
    let year: u32 = digits
        .parse()
        .map_err(|_| format!("year out of range: {digits:?}"))?;
    if year == 0 {
        return Err("year 0000 is invalid in Gregorian calendar".to_string());
    }
    Ok((year, rest))
}

/// Consumes a run of 1-3 leading ASCII digits (a fraction-of-a-second),
/// returning the remainder. Caps at 3 digits for every type in this
/// family: `DatetimeTz`'s regex capture group technically allows more,
/// but vnu's `checkMilliSecond` still rejects anything longer, so this
/// replicates vnu's actually-enforced behaviour rather than its regex.
fn take_fraction_digits(s: &str) -> Result<&str, String> {
    let digit_count = s.chars().take_while(char::is_ascii_digit).count();
    if digit_count == 0 {
        return Err(format!("expected fraction digits in {s:?}"));
    }
    if digit_count > 3 {
        return Err(format!("fraction has too many digits in {s:?}"));
    }
    Ok(s.split_at(digit_count).1)
}

fn expect_char(s: &str, expected: char) -> Result<&str, String> {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c == expected => Ok(chars.as_str()),
        _ => Err(format!("expected {expected:?} in {s:?}")),
    }
}

/// Case-insensitive single-character strip, used for duration designators.
fn strip_char_ci(s: &str, expected: char) -> Option<&str> {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.eq_ignore_ascii_case(&expected) => Some(chars.as_str()),
        _ => None,
    }
}

// ---------- date/time production parsers (shared between the 7 checks) ----------

/// `YYYY-MM-DD`.
fn parse_date_prefix(s: &str) -> Result<(u32, u32, u32, &str), String> {
    let (year, rest) = take_year_digits(s)?;
    let rest = expect_char(rest, '-')?;
    let (month, rest) = take_n_digits(rest, 2)?;
    let rest = expect_char(rest, '-')?;
    let (day, rest) = take_n_digits(rest, 2)?;

    if !(1..=12).contains(&month) {
        return Err(format!("invalid month {month:02}"));
    }
    let max_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => unreachable!(),
    };
    if day < 1 || day > max_days {
        return Err(format!(
            "invalid day {day:02} for month {month:02} in year {year}"
        ));
    }

    Ok((year, month, day, rest))
}

/// `MM-DD` (no year component — used only inside `check_time_datetime`'s
/// "yearless date" alternative).
fn parse_yearless_date_prefix(s: &str) -> Result<(u32, u32, &str), String> {
    let (month, rest) = take_n_digits(s, 2)?;
    let rest = expect_char(rest, '-')?;
    let (day, rest) = take_n_digits(rest, 2)?;
    Ok((month, day, rest))
}

/// `HH:MM` + optional `:SS` + optional `.` + 1-3 fraction digits (fraction
/// only permitted when `:SS` is present).
fn parse_time_prefix(s: &str) -> Result<(u32, u32, Option<u32>, &str), String> {
    let (hour, rest) = take_n_digits(s, 2)?;
    let rest = expect_char(rest, ':')?;
    let (minute, rest) = take_n_digits(rest, 2)?;

    let (second, rest) = match rest.strip_prefix(':') {
        Some(after_colon) => {
            let (second, rest) = take_n_digits(after_colon, 2)?;
            (Some(second), rest)
        }
        None => (None, rest),
    };

    let rest = match rest.strip_prefix('.') {
        Some(after_dot) => {
            if second.is_none() {
                return Err("fractional seconds require seconds to be present".to_string());
            }
            take_fraction_digits(after_dot)?
        }
        None => rest,
    };

    Ok((hour, minute, second, rest))
}

/// Single date/time separator: literal `T` or a literal space.
fn strip_datetime_separator(s: &str) -> Result<&str, String> {
    let mut chars = s.chars();
    match chars.next() {
        Some('T') | Some(' ') => Ok(chars.as_str()),
        Some(c) => Err(format!("invalid date/time separator {c:?}")),
        None => Err("missing date/time separator".to_string()),
    }
}

/// Timezone: `Z`, or `[+-]` + 2 digits + optional `:` + 2 digits. `-00:00`
/// is rejected — only `+00:00`/`Z` represent the zero offset.
fn parse_timezone(s: &str) -> Result<(), String> {
    if s == "Z" {
        return Ok(());
    }

    let mut chars = s.chars();
    let sign = match chars.next() {
        Some(c @ ('+' | '-')) => c,
        Some(c) => return Err(format!("invalid timezone sign {c:?}")),
        None => return Err("missing timezone".to_string()),
    };
    let rest = chars.as_str();

    let (tz_hour, rest) = take_n_digits(rest, 2)?;
    let rest = rest.strip_prefix(':').unwrap_or(rest);
    let (tz_minute, rest) = take_n_digits(rest, 2)?;

    if !rest.is_empty() {
        return Err(format!(
            "unexpected trailing characters in timezone: {rest:?}"
        ));
    }
    if tz_hour > 23 {
        return Err(format!("invalid timezone hour {tz_hour}"));
    }
    if tz_minute != 0 && tz_minute != 30 && tz_minute != 45 {
        return Err(format!(
            "invalid timezone minute offset {tz_minute:02}: must be 00, 30, or 45"
        ));
    }
    if sign == '+' && (tz_hour > 14 || (tz_hour == 14 && tz_minute > 0)) {
        return Err(format!(
            "timezone offset +{tz_hour:02}:{tz_minute:02} is out of bounds (+14:00 max)"
        ));
    }
    if sign == '-' && (tz_hour > 12 || (tz_hour == 12 && tz_minute > 0)) {
        return Err(format!(
            "timezone offset -{tz_hour:02}:{tz_minute:02} is out of bounds (-12:00 max)"
        ));
    }
    if sign == '-' && tz_hour == 0 && tz_minute == 0 {
        return Err("-00:00 is not a valid timezone offset (use +00:00 or Z)".to_string());
    }
    // vnu's `AbstractDatetime.checkTzd` (WARN-gated in vnu, implemented as
    // a hard rejection here like the existing hour bounds above): real
    // time-zone offsets only ever use :00, :30, or :45 minutes — :15 has
    // no current real-world zone, so this doesn't collide with e.g.
    // Nepal's +05:45 or India's +05:30.
    if tz_minute != 0 && tz_minute != 30 && tz_minute != 45 {
        return Err(format!(
            "timezone minutes should be 00, 30, or 45, not {tz_minute:02}"
        ));
    }

    Ok(())
}

// ---------- the 7 public checks ----------

/// `w:date` (`Date.java`): `YYYY-MM-DD`, strictly anchored, no whitespace.
pub(crate) fn check_date(value: &str) -> Result<(), String> {
    let (year, month, day, rest) = parse_date_prefix(value)?;
    if !rest.is_empty() {
        return Err(format!("unexpected trailing characters: {rest:?}"));
    }
    check_date_parts(year, month, day)
}

/// `w:datetime-local` (`DatetimeLocal.java`): date + `T`/space + time.
pub(crate) fn check_datetime_local(value: &str) -> Result<(), String> {
    let (year, month, day, rest) = parse_date_prefix(value)?;
    check_date_parts(year, month, day)?;
    let rest = strip_datetime_separator(rest)?;
    let (hour, minute, second, rest) = parse_time_prefix(rest)?;
    if !rest.is_empty() {
        return Err(format!("unexpected trailing characters: {rest:?}"));
    }
    check_time_parts(hour, minute, second)
}

/// `w:datetime-tz` (`DatetimeTz.java`): date + `T`/space + time + timezone.
///
/// Seconds are optional here, exactly like `check_datetime_local` — this
/// replicates `DatetimeTz.java`'s actual regex behaviour, which the
/// class's own doc comment (incorrectly) claims makes seconds mandatory
/// (see `plan/05c-research-group-b.md`, item 3).
pub(crate) fn check_datetime_tz(value: &str) -> Result<(), String> {
    let (year, month, day, rest) = parse_date_prefix(value)?;
    check_date_parts(year, month, day)?;
    let rest = strip_datetime_separator(rest)?;
    let (hour, minute, second, rest) = parse_time_prefix(rest)?;
    check_time_parts(hour, minute, second)?;
    if rest.is_empty() {
        return Err("missing timezone".to_string());
    }
    parse_timezone(rest)
}

/// `w:month` (`Month.java`): `YYYY-MM`. Does NOT share `check_date_parts`
/// (there is no day component).
pub(crate) fn check_month(value: &str) -> Result<(), String> {
    let (year, rest) = take_year_digits(value)?;
    let rest = expect_char(rest, '-')?;
    let (month, rest) = take_n_digits(rest, 2)?;
    if !rest.is_empty() {
        return Err(format!("unexpected trailing characters: {rest:?}"));
    }
    if year < 1 {
        return Err(format!("invalid year {year}"));
    }
    if !(1..=12).contains(&month) {
        return Err(format!("invalid month {month}"));
    }
    Ok(())
}

/// `w:time` (`Time.java`): `HH:MM` + optional `:SS` + optional fraction.
pub(crate) fn check_time(value: &str) -> Result<(), String> {
    let (hour, minute, second, rest) = parse_time_prefix(value)?;
    if !rest.is_empty() {
        return Err(format!("unexpected trailing characters: {rest:?}"));
    }
    check_time_parts(hour, minute, second)
}

/// ISO week-53 years, expressed as `year % 400`, reconstructed from vnu's
/// `Week.java` `SPECIAL_YEARS` table (J. R. Stockton's formula for years
/// with an ISO leap week). Exactly 71 entries.
const WEEK_53_SPECIAL_YEARS_MOD_400: [u32; 71] = [
    4, 9, 15, 20, 26, 32, 37, 43, 48, 54, 60, 65, 71, 76, 82, 88, 93, 99, 105, 111, 116, 122, 128,
    133, 139, 144, 150, 156, 161, 167, 172, 178, 184, 189, 195, 201, 207, 212, 218, 224, 229, 235,
    240, 246, 252, 257, 263, 268, 274, 280, 285, 291, 296, 303, 308, 314, 320, 325, 331, 336, 342,
    348, 353, 359, 364, 370, 376, 381, 387, 392, 398,
];

/// `w:week` (`Week.java`): `YYYY-Www`. Week 53 is only valid for years
/// whose `year % 400` is in `WEEK_53_SPECIAL_YEARS_MOD_400`.
pub(crate) fn check_week(value: &str) -> Result<(), String> {
    let (year, rest) = take_year_digits(value)?;
    let rest = expect_char(rest, '-')?;
    let rest = expect_char(rest, 'W')?;
    let (week, rest) = take_n_digits(rest, 2)?;
    if !rest.is_empty() {
        return Err(format!("unexpected trailing characters: {rest:?}"));
    }
    if year < 1 {
        return Err(format!("invalid year {year}"));
    }
    if week == 0 || week > 53 {
        return Err(format!("invalid week {week}"));
    }
    if week == 53 && !WEEK_53_SPECIAL_YEARS_MOD_400.contains(&(year % 400)) {
        return Err(format!("week 53 is not a valid ISO week for year {year}"));
    }
    Ok(())
}

// ---------- `w:time-datetime`: union of all productions above, plus duration ----------

/// `MM-DD`, no year (the "yearless date" alternative of `w:time-datetime`).
/// vnu's `TimeDatetime.java` accepts this form for elements like `<time>`
/// representing an annual recurring date. There is no year to check leap
/// years against, so February is treated as though it could be a leap
/// year (day 29 permitted), matching WHATWG's "valid month-day string"
/// definition.
fn check_yearless_date(value: &str) -> Result<(), String> {
    let (month, day, rest) = parse_yearless_date_prefix(value)?;
    if !rest.is_empty() {
        return Err(format!("unexpected trailing characters: {rest:?}"));
    }
    if !(1..=12).contains(&month) {
        return Err(format!("invalid month {month}"));
    }
    let max_day = if month == 2 {
        29
    } else {
        DAYS_IN_MONTH[(month - 1) as usize]
    };
    if day < 1 || day > max_day {
        return Err(format!("invalid day {day} for month {month}"));
    }
    Ok(())
}

/// A standalone timezone offset (`Z` or `[+-]HH:?MM`), nothing else.
fn check_tz_alone(value: &str) -> Result<(), String> {
    parse_timezone(value)
}

/// A bare 4+ digit year, nothing else.
fn check_bare_year(value: &str) -> Result<(), String> {
    let (_year, rest) = take_year_digits(value)?;
    if !rest.is_empty() {
        return Err(format!("unexpected trailing characters: {rest:?}"));
    }
    Ok(())
}

/// Consumes an optional `digits` + `unit` (case-insensitive) component
/// from `*cursor`, advancing it and returning `true` on success. Leaves
/// `*cursor` untouched and returns `false` if the component isn't present.
fn take_digit_unit_component(cursor: &mut &str, unit: char) -> bool {
    let digit_count = cursor.chars().take_while(char::is_ascii_digit).count();
    if digit_count == 0 {
        return false;
    }
    let (_digits, after_digits) = cursor.split_at(digit_count);
    match strip_char_ci(after_digits, unit) {
        Some(after_unit) => {
            *cursor = after_unit;
            true
        }
        None => false,
    }
}

/// Consumes an optional `digits ['.' digits] 'S'` (seconds) component from
/// `*cursor`, case-insensitive on `S`.
fn take_duration_seconds_component(cursor: &mut &str) -> bool {
    let digit_count = cursor.chars().take_while(char::is_ascii_digit).count();
    if digit_count == 0 {
        return false;
    }
    let (_digits, after_digits) = cursor.split_at(digit_count);
    let after_fraction = match after_digits.strip_prefix('.') {
        Some(after_dot) => {
            let frac_count = after_dot.chars().take_while(char::is_ascii_digit).count();
            if frac_count == 0 {
                return false;
            }
            after_dot.split_at(frac_count).1
        }
        None => after_digits,
    };
    match strip_char_ci(after_fraction, 'S') {
        Some(rest) => {
            *cursor = rest;
            true
        }
        None => false,
    }
}

/// ISO-8601-ish duration: `P` [digits `D`] [`T` [digits `H`] [digits `M`]
/// [(digits [`.` digits]) `S`]]. At least one of the D/H/M/S components
/// must be present — a bare `P` or `PT` is rejected.
fn check_iso_duration(value: &str) -> Result<(), String> {
    let mut cursor =
        strip_char_ci(value, 'P').ok_or_else(|| "duration must start with 'P'".to_string())?;
    let mut has_component = false;

    if take_digit_unit_component(&mut cursor, 'D') {
        has_component = true;
    }

    if let Some(after_t) = strip_char_ci(cursor, 'T') {
        cursor = after_t;
        if take_digit_unit_component(&mut cursor, 'H') {
            has_component = true;
        }
        if take_digit_unit_component(&mut cursor, 'M') {
            has_component = true;
        }
        if take_duration_seconds_component(&mut cursor) {
            has_component = true;
        }
    }

    if !cursor.is_empty() {
        return Err(format!(
            "unexpected trailing characters in duration: {cursor:?}"
        ));
    }
    if !has_component {
        return Err("duration must have at least one component".to_string());
    }
    Ok(())
}

/// vnu's own "word-style" duration liberalization, used only inside
/// `TimeDatetime.java`/`check_time_datetime` — this is **not** part of the
/// WHATWG duration-string grammar; it is a vnu-specific extension
/// (documented in `plan/05c-research-group-b.md`, item 6). One or more
/// repetitions of: optional whitespace, digits, optional whitespace, then
/// either a single unit letter from `W`/`D`/`H`/`M` (case-insensitive), or
/// an optional `.`+digits fraction followed by `S`/`s`.
fn check_word_duration(value: &str) -> Result<(), String> {
    let mut cursor = value;
    let mut has_component = false;

    loop {
        cursor = trim_vnu_whitespace_start(cursor);
        if cursor.is_empty() {
            break;
        }

        let digit_count = cursor.chars().take_while(char::is_ascii_digit).count();
        if digit_count == 0 {
            return Err(format!("expected digits in duration token: {cursor:?}"));
        }
        let (_digits, after_digits) = cursor.split_at(digit_count);
        let after_digits = trim_vnu_whitespace_start(after_digits);

        let mut chars = after_digits.chars();
        let unit = chars
            .next()
            .ok_or_else(|| "duration token missing unit".to_string())?;

        if matches!(unit.to_ascii_uppercase(), 'W' | 'D' | 'H' | 'M') {
            cursor = chars.as_str();
        } else if unit == '.' {
            let after_dot = chars.as_str();
            let frac_count = after_dot.chars().take_while(char::is_ascii_digit).count();
            if frac_count == 0 {
                return Err(format!("expected fraction digits in {after_dot:?}"));
            }
            let after_frac = after_dot.split_at(frac_count).1;
            let mut chars2 = after_frac.chars();
            match chars2.next() {
                Some('S') | Some('s') => cursor = chars2.as_str(),
                Some(c) => return Err(format!("expected 's' duration unit, found {c:?}")),
                None => return Err("duration token missing seconds unit".to_string()),
            }
        } else if unit == 'S' || unit == 's' {
            cursor = chars.as_str();
        } else {
            return Err(format!("unknown duration unit {unit:?}"));
        }

        has_component = true;
    }

    if !has_component {
        return Err("duration must have at least one component".to_string());
    }
    Ok(())
}

/// A duration: either the ISO-8601-ish form or vnu's looser word-style
/// form.
fn check_duration(value: &str) -> Result<(), String> {
    if check_iso_duration(value).is_ok() {
        return Ok(());
    }
    check_word_duration(value)
}

/// `w:time-datetime` (`TimeDatetime.java`) — the union of every other
/// production in this file, plus a duration. Unlike all 6 other checks in
/// this family, leading/trailing whitespace (vnu's 5-character
/// `isWhitespace` set) is explicitly permitted around the value.
///
/// Alternatives are tried strictly in the order below and the first one
/// that matches the *entire* trimmed string wins (each candidate parser is
/// itself fully anchored, so a partial match is rejected and falls
/// through to the next alternative — no alternative can silently match a
/// prefix of a different, longer production).
pub(crate) fn check_time_datetime(value: &str) -> Result<(), String> {
    let trimmed = trim_vnu_whitespace(value);

    if check_month(trimmed).is_ok() {
        return Ok(());
    }
    if check_date(trimmed).is_ok() {
        return Ok(());
    }
    if check_yearless_date(trimmed).is_ok() {
        return Ok(());
    }
    if check_time(trimmed).is_ok() {
        return Ok(());
    }
    if check_datetime_local(trimmed).is_ok() {
        return Ok(());
    }
    if check_tz_alone(trimmed).is_ok() {
        return Ok(());
    }
    if check_datetime_tz(trimmed).is_ok() {
        return Ok(());
    }
    if check_week(trimmed).is_ok() {
        return Ok(());
    }
    if check_bare_year(trimmed).is_ok() {
        return Ok(());
    }
    if check_duration(trimmed).is_ok() {
        return Ok(());
    }

    Err(format!(
        "{value:?} does not match any month, date, time, datetime, week, year, or duration production"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- check_date ----

    #[test]
    fn date_valid() {
        assert!(check_date("2024-01-01").is_ok());
    }

    #[test]
    fn date_year_below_1000_is_implausible() {
        assert!(check_date("0004-02-29").is_err());
        assert!(check_date("0214-09-29").is_err());
    }

    #[test]
    fn date_year_at_or_above_3000_is_implausible() {
        assert!(check_date("20014-09-29").is_err());
        assert!(check_date("12014-09-29").is_err());
        assert!(check_date("3000-01-01").is_err());
    }

    #[test]
    fn date_year_999_and_2999_are_plausibility_boundaries() {
        assert!(check_date("0999-01-01").is_err());
        assert!(check_date("2999-01-01").is_ok());
    }

    #[test]
    fn date_feb29_leap_year_valid() {
        assert!(check_date("2024-02-29").is_ok());
    }

    #[test]
    fn date_feb29_non_leap_year_invalid() {
        assert!(check_date("2023-02-29").is_err());
    }

    #[test]
    fn date_month_13_invalid() {
        assert!(check_date("2024-13-01").is_err());
    }

    #[test]
    fn date_rejects_leading_whitespace() {
        assert!(check_date(" 2024-01-01").is_err());
    }

    #[test]
    fn date_rejects_trailing_whitespace() {
        assert!(check_date("2024-01-01 ").is_err());
    }

    // ---- check_datetime_local ----

    #[test]
    fn datetime_local_valid_with_t_separator() {
        assert!(check_datetime_local("2024-01-01T10:30:00").is_ok());
    }

    #[test]
    fn datetime_local_valid_with_space_separator() {
        assert!(check_datetime_local("2024-01-01 10:30").is_ok());
    }

    #[test]
    fn datetime_local_valid_with_fraction() {
        assert!(check_datetime_local("2024-01-01T10:30:00.123").is_ok());
    }

    #[test]
    fn datetime_local_fraction_without_seconds_invalid() {
        assert!(check_datetime_local("2024-01-01T10:30.123").is_err());
    }

    #[test]
    fn datetime_local_rejects_whitespace() {
        assert!(check_datetime_local(" 2024-01-01T10:30").is_err());
        assert!(check_datetime_local("2024-01-01T10:30 ").is_err());
    }

    // ---- check_datetime_tz ----

    #[test]
    fn datetime_tz_minus_00_00_invalid() {
        assert!(check_datetime_tz("2024-01-01T10:30:00-00:00").is_err());
    }

    #[test]
    fn datetime_tz_plus_00_00_valid() {
        assert!(check_datetime_tz("2024-01-01T10:30:00+00:00").is_ok());
    }

    #[test]
    fn datetime_tz_offset_minutes_15_is_implausible() {
        assert!(check_datetime_tz("2011-11-12T00:00:00+08:15").is_err());
    }

    #[test]
    fn datetime_tz_offset_minutes_30_and_45_are_plausible() {
        assert!(check_datetime_tz("2024-01-01T10:30:00+05:30").is_ok());
        assert!(check_datetime_tz("2024-01-01T10:30:00+05:45").is_ok());
    }

    #[test]
    fn datetime_tz_z_valid() {
        assert!(check_datetime_tz("2024-01-01T10:30:00Z").is_ok());
    }

    #[test]
    fn datetime_tz_hour_25_invalid() {
        assert!(check_datetime_tz("2024-01-01T10:30:00+25:00").is_err());
    }

    #[test]
    fn datetime_tz_seconds_optional() {
        assert!(check_datetime_tz("2024-01-01T10:30Z").is_ok());
    }

    #[test]
    fn datetime_tz_rejects_whitespace() {
        assert!(check_datetime_tz(" 2024-01-01T10:30:00Z").is_err());
        assert!(check_datetime_tz("2024-01-01T10:30:00Z ").is_err());
    }

    // ---- check_month ----

    #[test]
    fn month_valid() {
        assert!(check_month("2024-01").is_ok());
    }

    #[test]
    fn month_13_invalid() {
        assert!(check_month("2024-13").is_err());
    }

    #[test]
    fn month_rejects_whitespace() {
        assert!(check_month(" 2024-01").is_err());
        assert!(check_month("2024-01 ").is_err());
    }

    // ---- check_time ----

    #[test]
    fn time_valid() {
        assert!(check_time("10:30:00.123").is_ok());
    }

    #[test]
    fn time_hour_24_invalid() {
        assert!(check_time("24:00").is_err());
    }

    #[test]
    fn time_rejects_whitespace() {
        assert!(check_time(" 10:30").is_err());
        assert!(check_time("10:30 ").is_err());
    }

    // ---- check_week ----

    #[test]
    fn week_53_valid_for_special_year() {
        // 2020 % 400 == 20, which is in WEEK_53_SPECIAL_YEARS_MOD_400.
        assert!(WEEK_53_SPECIAL_YEARS_MOD_400.contains(&20));
        assert!(check_week("2020-W53").is_ok());
    }

    #[test]
    fn week_53_invalid_for_non_special_year() {
        // 2021 % 400 == 21, which is not in WEEK_53_SPECIAL_YEARS_MOD_400.
        assert!(!WEEK_53_SPECIAL_YEARS_MOD_400.contains(&21));
        assert!(check_week("2021-W53").is_err());
    }

    #[test]
    fn week_54_always_invalid() {
        assert!(check_week("2020-W54").is_err());
    }

    #[test]
    fn week_53_special_years_table_has_71_entries() {
        assert_eq!(WEEK_53_SPECIAL_YEARS_MOD_400.len(), 71);
    }

    #[test]
    fn week_rejects_whitespace() {
        assert!(check_week(" 2020-W53").is_err());
        assert!(check_week("2020-W53 ").is_err());
    }

    // ---- check_time_datetime ----

    #[test]
    fn time_datetime_bare_year_valid() {
        assert!(check_time_datetime("2024").is_ok());
    }

    #[test]
    fn time_datetime_iso_duration_valid() {
        assert!(check_time_datetime("P1D").is_ok());
    }

    #[test]
    fn time_datetime_word_duration_valid() {
        assert!(check_time_datetime("1h30m").is_ok());
    }

    #[test]
    fn time_datetime_tolerates_surrounding_whitespace() {
        assert!(check_time_datetime(" 2024-01-01 ").is_ok());
        assert!(check_time_datetime("\t2024-01-01\n").is_ok());
    }

    #[test]
    fn time_datetime_yearless_date_valid() {
        assert!(check_time_datetime("02-29").is_ok());
    }

    #[test]
    fn time_datetime_bare_time_valid() {
        assert!(check_time_datetime(" 10:30 ").is_ok());
    }

    #[test]
    fn time_datetime_week_valid() {
        assert!(check_time_datetime("2020-W53").is_ok());
    }

    #[test]
    fn time_datetime_tz_alone_valid() {
        assert!(check_time_datetime("Z").is_ok());
        assert!(check_time_datetime("+05:00").is_ok());
    }

    #[test]
    fn time_datetime_global_datetime_valid() {
        assert!(check_time_datetime("2024-01-01T10:30:00+05:00").is_ok());
    }

    #[test]
    fn time_datetime_rejects_garbage() {
        assert!(check_time_datetime("not a date").is_err());
    }
}
