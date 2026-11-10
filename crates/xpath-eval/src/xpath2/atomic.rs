//! The atomic types of the XPath 2.0 mode that the rule sets need
//! (`xs:untypedAtomic`, `xs:string`, `xs:boolean`, `xs:integer`,
//! `xs:decimal`, `xs:double`, `xs:date`) with casting (XPath 2.0 §17 of F&O,
//! "Casting"), value comparison (§3.5.1) and arithmetic (§3.4).

use super::Error;
use super::decimal::Decimal;
use std::cmp::Ordering;

/// An `xs:date` without timezone (XML Schema Part 2 §3.2.9). A timezone in
/// the lexical form is reported as an unsupported feature, because comparing
/// dates correctly then needs the implicit timezone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
    pub fn parse(text: &str) -> Result<Date, Error> {
        let invalid = || Error::dynamic("FORG0001", format!("`{text}` is not a valid xs:date"));
        let (negative, rest) = match text.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, text),
        };
        let parts: Vec<&str> = rest.splitn(3, '-').collect();
        let [year, month, day] = parts[..] else {
            return Err(invalid());
        };
        let day_digits = day.bytes().take_while(u8::is_ascii_digit).count();
        let (day, zone) = day.split_at(day_digits);
        if !zone.is_empty() {
            let looks_like_zone = zone == "Z"
                || (zone.len() == 6
                    && matches!(zone.as_bytes()[0], b'+' | b'-')
                    && zone.as_bytes()[3] == b':');
            return Err(if looks_like_zone {
                Error::Unsupported("xs:date with a timezone".into())
            } else {
                invalid()
            });
        }
        let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        if !(all_digits(year) && year.len() >= 4 && month.len() == 2 && day.len() == 2)
            || !all_digits(month)
            || !all_digits(day)
        {
            return Err(invalid());
        }
        let year: i32 = year.parse().map_err(|_| invalid())?;
        let month: u8 = month.parse().map_err(|_| invalid())?;
        let day: u8 = day.parse().map_err(|_| invalid())?;
        let year = if negative { -year } else { year };
        if year == 0 || !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return Err(invalid());
        }
        Ok(Date { year, month, day })
    }
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap { 29 } else { 28 }
        }
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.year < 0 { "-" } else { "" };
        write!(
            f,
            "{sign}{:04}-{:02}-{:02}",
            self.year.abs(),
            self.month,
            self.day
        )
    }
}

/// An atomic value. Values read from the document are `Untyped`: there is no
/// schema, so no node carries a type annotation (XPath 2.0 §2.5.2, untyped
/// data).
#[derive(Debug, Clone, PartialEq)]
pub enum Atomic {
    Untyped(String),
    String(String),
    Boolean(bool),
    /// `xs:integer`, limited to 64 bits; overflow raises `FOAR0002`.
    Integer(i64),
    Decimal(Decimal),
    Double(f64),
    Date(Date),
}

/// The target types of `cast as`/`castable as` and the `xs:` constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicType {
    UntypedAtomic,
    String,
    Boolean,
    Integer,
    Decimal,
    Double,
    Date,
}

impl AtomicType {
    pub(crate) fn from_local_name(name: &str) -> Option<AtomicType> {
        Some(match name {
            "untypedAtomic" => AtomicType::UntypedAtomic,
            "string" => AtomicType::String,
            "boolean" => AtomicType::Boolean,
            "integer" => AtomicType::Integer,
            "decimal" => AtomicType::Decimal,
            "double" => AtomicType::Double,
            "date" => AtomicType::Date,
            _ => return None,
        })
    }
}

impl Atomic {
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Atomic::Integer(_) | Atomic::Decimal(_) | Atomic::Double(_)
        )
    }

    /// The canonical string form (F&O §17.1.2, casting to `xs:string`).
    pub fn to_xpath_string(&self) -> String {
        match self {
            Atomic::Untyped(s) | Atomic::String(s) => s.clone(),
            Atomic::Boolean(b) => b.to_string(),
            Atomic::Integer(i) => i.to_string(),
            Atomic::Decimal(d) => d.to_string(),
            Atomic::Double(d) => double_to_string(*d),
            Atomic::Date(d) => d.to_string(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Atomic::Untyped(_) => "xs:untypedAtomic",
            Atomic::String(_) => "xs:string",
            Atomic::Boolean(_) => "xs:boolean",
            Atomic::Integer(_) => "xs:integer",
            Atomic::Decimal(_) => "xs:decimal",
            Atomic::Double(_) => "xs:double",
            Atomic::Date(_) => "xs:date",
        }
    }

    /// Effective boolean value of a single atomic (XPath 2.0 §2.4.3).
    pub fn effective_boolean(&self) -> Result<bool, Error> {
        match self {
            Atomic::Boolean(b) => Ok(*b),
            Atomic::Untyped(s) | Atomic::String(s) => Ok(!s.is_empty()),
            Atomic::Integer(i) => Ok(*i != 0),
            Atomic::Decimal(d) => Ok(!d.is_zero()),
            Atomic::Double(d) => Ok(*d != 0.0 && !d.is_nan()),
            Atomic::Date(_) => Err(Error::dynamic(
                "FORG0006",
                "xs:date has no effective boolean value",
            )),
        }
    }

    /// `cast as` (XPath 2.0 §3.10.2, F&O §17).
    pub fn cast(&self, to: AtomicType) -> Result<Atomic, Error> {
        let text_source = matches!(self, Atomic::Untyped(_) | Atomic::String(_));
        let lexical = || collapse(&self.to_xpath_string());
        let invalid = |what: &str| {
            Error::dynamic(
                "FORG0001",
                format!(
                    "cannot cast {} `{}` to {what}",
                    self.type_name(),
                    self.to_xpath_string()
                ),
            )
        };
        Ok(match to {
            AtomicType::UntypedAtomic => Atomic::Untyped(self.to_xpath_string()),
            AtomicType::String => Atomic::String(self.to_xpath_string()),
            AtomicType::Boolean => match self {
                Atomic::Boolean(_) => self.clone(),
                _ if text_source => match lexical().as_str() {
                    "true" | "1" => Atomic::Boolean(true),
                    "false" | "0" => Atomic::Boolean(false),
                    _ => return Err(invalid("xs:boolean")),
                },
                _ if self.is_numeric() => Atomic::Boolean(self.effective_boolean()?),
                _ => return Err(type_error(self, "xs:boolean")),
            },
            AtomicType::Double => match self {
                Atomic::Double(_) => self.clone(),
                Atomic::Integer(i) => Atomic::Double(*i as f64),
                Atomic::Decimal(d) => Atomic::Double(d.to_f64()),
                Atomic::Boolean(b) => Atomic::Double(if *b { 1.0 } else { 0.0 }),
                _ if text_source => {
                    Atomic::Double(parse_double(&lexical()).ok_or_else(|| invalid("xs:double"))?)
                }
                _ => return Err(type_error(self, "xs:double")),
            },
            AtomicType::Decimal => match self {
                Atomic::Decimal(_) => self.clone(),
                Atomic::Integer(i) => Atomic::Decimal(Decimal::from_i64(*i)),
                Atomic::Double(d) => Atomic::Decimal(Decimal::from_f64(*d).ok_or_else(|| {
                    Error::dynamic("FOCA0002", format!("cannot cast {d} to xs:decimal"))
                })?),
                Atomic::Boolean(b) => Atomic::Decimal(Decimal::from_i64(*b as i64)),
                _ if text_source => Atomic::Decimal(
                    Decimal::parse(&lexical()).ok_or_else(|| invalid("xs:decimal"))?,
                ),
                _ => return Err(type_error(self, "xs:decimal")),
            },
            AtomicType::Integer => match self {
                Atomic::Integer(_) => self.clone(),
                Atomic::Decimal(d) => Atomic::Integer(
                    d.trunc()
                        .to_i64_exact()
                        .ok_or_else(|| Error::dynamic("FOAR0002", "xs:integer overflow"))?,
                ),
                Atomic::Double(d) => {
                    if !d.is_finite() {
                        return Err(Error::dynamic(
                            "FOCA0002",
                            format!("cannot cast {} to xs:integer", double_to_string(*d)),
                        ));
                    }
                    let t = d.trunc();
                    if t >= i64::MAX as f64 || t <= i64::MIN as f64 {
                        return Err(Error::dynamic("FOAR0002", "xs:integer overflow"));
                    }
                    Atomic::Integer(t as i64)
                }
                Atomic::Boolean(b) => Atomic::Integer(*b as i64),
                _ if text_source => {
                    let s = lexical();
                    let digits = s.strip_prefix(['+', '-']).unwrap_or(&s);
                    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(invalid("xs:integer"));
                    }
                    Atomic::Integer(
                        s.parse()
                            .map_err(|_| Error::dynamic("FOAR0002", "xs:integer beyond 64 bits"))?,
                    )
                }
                _ => return Err(type_error(self, "xs:integer")),
            },
            AtomicType::Date => match self {
                Atomic::Date(_) => self.clone(),
                _ if text_source => Atomic::Date(Date::parse(&lexical())?),
                _ => return Err(type_error(self, "xs:date")),
            },
        })
    }
}

fn type_error(value: &Atomic, target: &str) -> Error {
    Error::dynamic(
        "XPTY0004",
        format!("cannot cast {} to {target}", value.type_name()),
    )
}

/// XML Schema `whiteSpace="collapse"`: trim, and fold inner runs of
/// whitespace to one space.
fn collapse(s: &str) -> String {
    s.split([' ', '\t', '\n', '\r'])
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// XML Schema `xs:double` lexical space: decimal or scientific notation plus
/// `INF`, `-INF`, `+INF` and `NaN` (Rust's own `parse` would also take
/// `inf`, `infinity` and `nan`, which are not valid).
fn parse_double(s: &str) -> Option<f64> {
    match s {
        "INF" | "+INF" => return Some(f64::INFINITY),
        "-INF" => return Some(f64::NEG_INFINITY),
        "NaN" => return Some(f64::NAN),
        _ => {}
    }
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (mantissa, exponent) = match body.find(['e', 'E']) {
        Some(i) => (&body[..i], Some(&body[i + 1..])),
        None => (body, None),
    };
    let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = |p: &str| p.bytes().all(|b| b.is_ascii_digit());
    if (int_part.is_empty() && frac_part.is_empty()) || !digits(int_part) || !digits(frac_part) {
        return None;
    }
    if let Some(e) = exponent {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        if e.is_empty() || !digits(e) {
            return None;
        }
    }
    s.parse().ok()
}

/// F&O §17.1.2, casting `xs:double` to `xs:string`: plain decimal notation
/// for magnitudes in `[1e-6, 1e6)`, scientific (`1.5E7`) outside.
pub fn double_to_string(d: f64) -> String {
    if d.is_nan() {
        return "NaN".into();
    }
    if d.is_infinite() {
        return if d > 0.0 { "INF" } else { "-INF" }.into();
    }
    if d == 0.0 {
        return if d.is_sign_negative() { "-0" } else { "0" }.into();
    }
    let magnitude = d.abs();
    if (1e-6..1e6).contains(&magnitude) {
        return format!("{d}");
    }
    let scientific = format!("{d:e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let mantissa = if mantissa.contains('.') {
        mantissa.to_owned()
    } else {
        format!("{mantissa}.0")
    };
    format!("{mantissa}E{exponent}")
}

fn promote_double(a: &Atomic) -> f64 {
    match a {
        Atomic::Integer(i) => *i as f64,
        Atomic::Decimal(d) => d.to_f64(),
        Atomic::Double(d) => *d,
        _ => f64::NAN,
    }
}

fn promote_decimal(a: &Atomic) -> Decimal {
    match a {
        Atomic::Integer(i) => Decimal::from_i64(*i),
        Atomic::Decimal(d) => *d,
        _ => Decimal::ZERO,
    }
}

/// Value comparison of two typed atomics (XPath 2.0 §3.5.1, F&O §10 and
/// §6.2). `Ok(None)` is "unordered" (a NaN is involved). Untyped operands are
/// cast to `xs:string` by the caller of `eq`; general comparison casts them
/// first (see `general_pair`).
pub fn compare_values(a: &Atomic, b: &Atomic) -> Result<Option<Ordering>, Error> {
    use Atomic::*;
    Ok(match (a, b) {
        (x, y) if x.is_numeric() && y.is_numeric() => match (x, y) {
            (Integer(i), Integer(j)) => Some(i.cmp(j)),
            (Double(_), _) | (_, Double(_)) => promote_double(x).partial_cmp(&promote_double(y)),
            _ => Some(promote_decimal(x).cmp(&promote_decimal(y))),
        },
        (Untyped(x) | String(x), Untyped(y) | String(y)) => Some(x.cmp(y)),
        (Boolean(x), Boolean(y)) => Some(x.cmp(y)),
        (Date(x), Date(y)) => Some(x.cmp(y)),
        _ => {
            return Err(Error::dynamic(
                "XPTY0004",
                format!("cannot compare {} with {}", a.type_name(), b.type_name()),
            ));
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Comparison {
    pub fn holds(self, ordering: Option<Ordering>) -> bool {
        match (self, ordering) {
            (Comparison::Ne, None) => true,
            (_, None) => false,
            (Comparison::Eq, Some(o)) => o == Ordering::Equal,
            (Comparison::Ne, Some(o)) => o != Ordering::Equal,
            (Comparison::Lt, Some(o)) => o == Ordering::Less,
            (Comparison::Le, Some(o)) => o != Ordering::Greater,
            (Comparison::Gt, Some(o)) => o == Ordering::Greater,
            (Comparison::Ge, Some(o)) => o != Ordering::Less,
        }
    }
}

/// One pair of a general comparison (XPath 2.0 §3.5.2): an untyped operand is
/// cast to the type of the other (to `xs:double` against a number, to
/// `xs:string` against a string or another untyped value), then the pair is
/// compared as typed values.
pub fn general_pair(a: &Atomic, b: &Atomic, op: Comparison) -> Result<bool, Error> {
    use Atomic::Untyped;
    let (a, b) = match (a, b) {
        (Untyped(_), Untyped(_)) => (
            Atomic::String(a.to_xpath_string()),
            Atomic::String(b.to_xpath_string()),
        ),
        (Untyped(_), other) => (cast_untyped_like(a, other)?, other.clone()),
        (other, Untyped(_)) => (other.clone(), cast_untyped_like(b, other)?),
        _ => (a.clone(), b.clone()),
    };
    Ok(op.holds(compare_values(&a, &b)?))
}

fn cast_untyped_like(untyped: &Atomic, other: &Atomic) -> Result<Atomic, Error> {
    match other {
        o if o.is_numeric() => untyped.cast(AtomicType::Double),
        Atomic::String(_) => untyped.cast(AtomicType::String),
        Atomic::Boolean(_) => untyped.cast(AtomicType::Boolean),
        Atomic::Date(_) => untyped.cast(AtomicType::Date),
        _ => Ok(untyped.clone()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arithmetic {
    Add,
    Sub,
    Mul,
    Div,
    IDiv,
    Mod,
}

fn overflow() -> Error {
    Error::dynamic("FOAR0002", "numeric overflow")
}

fn division_by_zero() -> Error {
    Error::dynamic("FOAR0001", "division by zero")
}

/// Binary arithmetic on two atomics (XPath 2.0 §3.4, F&O §6.2). Untyped
/// operands become `xs:double`; integer, decimal and double promote in that
/// order; `integer div integer` is a decimal.
pub fn arithmetic(a: &Atomic, op: Arithmetic, b: &Atomic) -> Result<Atomic, Error> {
    use Atomic::*;
    let a = match a {
        Untyped(_) => a.cast(AtomicType::Double)?,
        _ => a.clone(),
    };
    let b = match b {
        Untyped(_) => b.cast(AtomicType::Double)?,
        _ => b.clone(),
    };
    if matches!(a, Date(_)) || matches!(b, Date(_)) {
        return Err(Error::Unsupported(
            "arithmetic on xs:date (date and duration arithmetic)".into(),
        ));
    }
    if !a.is_numeric() || !b.is_numeric() {
        return Err(Error::dynamic(
            "XPTY0004",
            format!(
                "arithmetic needs numeric operands, got {} and {}",
                a.type_name(),
                b.type_name()
            ),
        ));
    }
    if matches!(a, Double(_)) || matches!(b, Double(_)) {
        let (x, y) = (promote_double(&a), promote_double(&b));
        return Ok(match op {
            Arithmetic::Add => Double(x + y),
            Arithmetic::Sub => Double(x - y),
            Arithmetic::Mul => Double(x * y),
            Arithmetic::Div => Double(x / y),
            Arithmetic::Mod => Double(x % y),
            Arithmetic::IDiv => {
                if y == 0.0 {
                    return Err(division_by_zero());
                }
                let q = (x / y).trunc();
                if !q.is_finite() {
                    return Err(Error::dynamic("FOAR0002", "idiv result is not finite"));
                }
                Atomic::Double(q).cast(AtomicType::Integer)?
            }
        });
    }
    if let (Integer(x), Integer(y)) = (&a, &b) {
        let (x, y) = (*x, *y);
        match op {
            Arithmetic::Add => return x.checked_add(y).map(Integer).ok_or_else(overflow),
            Arithmetic::Sub => return x.checked_sub(y).map(Integer).ok_or_else(overflow),
            Arithmetic::Mul => return x.checked_mul(y).map(Integer).ok_or_else(overflow),
            Arithmetic::IDiv => {
                if y == 0 {
                    return Err(division_by_zero());
                }
                return x.checked_div(y).map(Integer).ok_or_else(overflow);
            }
            Arithmetic::Mod => {
                if y == 0 {
                    return Err(division_by_zero());
                }
                return Ok(Integer(x.checked_rem(y).unwrap_or(0)));
            }
            Arithmetic::Div => {}
        }
    }
    let (x, y) = (promote_decimal(&a), promote_decimal(&b));
    let by_zero = y.is_zero();
    let result = match op {
        Arithmetic::Add => x.checked_add(y),
        Arithmetic::Sub => x.checked_sub(y),
        Arithmetic::Mul => x.checked_mul(y),
        Arithmetic::Div => {
            if by_zero {
                return Err(division_by_zero());
            }
            x.checked_div(y)
        }
        Arithmetic::IDiv => {
            if by_zero {
                return Err(division_by_zero());
            }
            return x
                .checked_idiv(y)
                .and_then(|d| d.to_i64_exact())
                .map(Integer)
                .ok_or_else(overflow);
        }
        Arithmetic::Mod => {
            if by_zero {
                return Err(division_by_zero());
            }
            x.checked_rem(y)
        }
    };
    result.map(Decimal).ok_or_else(overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Atomic {
        Atomic::Decimal(Decimal::parse(s).unwrap())
    }
    fn untyped(s: &str) -> Atomic {
        Atomic::Untyped(s.into())
    }

    // F&O §17.1.2: canonical xs:double -> xs:string.
    #[test]
    fn double_canonical_strings() {
        assert_eq!(double_to_string(100.0), "100");
        assert_eq!(double_to_string(0.5), "0.5");
        assert_eq!(double_to_string(1.5e7), "1.5E7");
        assert_eq!(double_to_string(1e6), "1.0E6");
        assert_eq!(double_to_string(1.25e-7), "1.25E-7");
        assert_eq!(double_to_string(f64::INFINITY), "INF");
        assert_eq!(double_to_string(f64::NAN), "NaN");
        assert_eq!(double_to_string(-0.0), "-0");
    }

    // XML Schema Part 2 §3.2.5: xs:double lexical space.
    #[test]
    fn double_lexical_space() {
        assert_eq!(parse_double("1e3"), Some(1000.0));
        assert_eq!(parse_double("-.5"), Some(-0.5));
        assert_eq!(parse_double("INF"), Some(f64::INFINITY));
        assert_eq!(parse_double("inf"), None);
        assert_eq!(parse_double("nan"), None);
        assert_eq!(parse_double("1e"), None);
        assert_eq!(parse_double(""), None);
    }

    // F&O §17.1.1 casting to xs:decimal / xs:integer.
    #[test]
    fn casts() {
        assert_eq!(
            untyped(" 12.50 ").cast(AtomicType::Decimal).unwrap(),
            dec("12.5")
        );
        assert!(untyped("1e2").cast(AtomicType::Decimal).is_err());
        assert_eq!(
            Atomic::Double(3.9).cast(AtomicType::Integer).unwrap(),
            Atomic::Integer(3)
        );
        assert_eq!(
            untyped("true").cast(AtomicType::Boolean).unwrap(),
            Atomic::Boolean(true)
        );
        assert!(untyped("yes").cast(AtomicType::Boolean).is_err());
        assert!(matches!(
            untyped("x").cast(AtomicType::Double),
            Err(Error::Dynamic {
                code: "FORG0001",
                ..
            })
        ));
    }

    // XML Schema Part 2 §3.2.9: xs:date lexical space and validity.
    #[test]
    fn date_parsing() {
        let ok = |s: &str| Date::parse(s).unwrap().to_string();
        assert_eq!(ok("2026-02-28"), "2026-02-28");
        assert!(Date::parse("2026-02-29").is_err());
        assert!(Date::parse("2024-02-29").is_ok());
        assert!(Date::parse("2026-13-01").is_err());
        assert!(Date::parse("26-01-01").is_err());
        assert!(Date::parse("2026-1-1").is_err());
        assert!(matches!(
            Date::parse("2026-01-01Z"),
            Err(Error::Unsupported(_))
        ));
    }

    // XPath 2.0 §3.5.2: general comparison casts untyped to the other operand's type.
    #[test]
    fn general_comparison_pairs() {
        let t = |a: Atomic, b: Atomic, op| general_pair(&a, &b, op).unwrap();
        assert!(t(untyped("10"), Atomic::Integer(10), Comparison::Eq));
        assert!(t(untyped("2"), dec("10"), Comparison::Lt));
        assert!(t(
            untyped("abc"),
            Atomic::String("abc".into()),
            Comparison::Eq
        ));
        assert!(t(untyped("10"), untyped("9"), Comparison::Lt)); // both strings
        assert!(t(untyped("false"), Atomic::Boolean(false), Comparison::Eq));
        assert!(general_pair(&untyped("x"), &Atomic::Integer(1), Comparison::Eq).is_err());
        assert!(
            general_pair(
                &Atomic::String("1".into()),
                &Atomic::Integer(1),
                Comparison::Eq
            )
            .is_err()
        );
    }

    // XPath 2.0 §3.4: numeric promotion and `div` on integers.
    #[test]
    fn arithmetic_promotion() {
        let a = |x, op, y| arithmetic(&x, op, &y).unwrap();
        assert_eq!(
            a(Atomic::Integer(1), Arithmetic::Div, Atomic::Integer(4)),
            dec("0.25")
        );
        assert_eq!(
            a(Atomic::Integer(7), Arithmetic::IDiv, Atomic::Integer(2)),
            Atomic::Integer(3)
        );
        assert_eq!(a(dec("0.1"), Arithmetic::Add, dec("0.2")), dec("0.3"));
        assert_eq!(
            a(dec("1.5"), Arithmetic::Mul, Atomic::Double(2.0)),
            Atomic::Double(3.0)
        );
        assert_eq!(
            a(untyped("2"), Arithmetic::Add, Atomic::Integer(1)),
            Atomic::Double(3.0)
        );
        assert!(matches!(
            arithmetic(&Atomic::Integer(1), Arithmetic::IDiv, &Atomic::Integer(0)),
            Err(Error::Dynamic {
                code: "FOAR0001",
                ..
            })
        ));
        assert!(
            arithmetic(
                &Atomic::String("a".into()),
                Arithmetic::Add,
                &Atomic::Integer(1)
            )
            .is_err()
        );
        assert!(matches!(
            arithmetic(
                &Atomic::Date(Date::parse("2026-01-01").unwrap()),
                Arithmetic::Sub,
                &Atomic::Integer(1)
            ),
            Err(Error::Unsupported(_))
        ));
    }
}
