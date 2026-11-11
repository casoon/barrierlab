//! `xs:decimal` (XML Schema Part 2 §3.2.3) as an exact base-10 number: an
//! `i128` mantissa and a scale, `value = mantissa / 10^scale`. The XPath 2.0
//! functions-and-operators spec requires at least 18 decimal digits; this
//! type holds up to 38. An operation that does not fit returns `None`, which
//! the evaluator reports as `FOAR0002` (numeric overflow) instead of rounding
//! silently.

use std::cmp::Ordering;
use std::fmt;

/// Fractional digits kept by `div` (F&O §6.2.4: "at least 18 digits").
const DIVISION_DIGITS: u32 = 18;

/// Most fractional digits a value may have; also the digit limit of the mantissa.
const MAX_SCALE: u32 = 38;

/// Why a text or double is no `Decimal` of this implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecimalParseError {
    /// Not in the lexical space of `xs:decimal`.
    Invalid,
    /// Valid, but beyond the 38 digits this implementation holds.
    TooPrecise,
}

#[derive(Debug, Clone, Copy)]
pub struct Decimal {
    mantissa: i128,
    scale: u32,
}

fn pow10(n: u32) -> Option<i128> {
    10i128.checked_pow(n)
}

impl Decimal {
    pub const ZERO: Decimal = Decimal {
        mantissa: 0,
        scale: 0,
    };

    fn normalized(mut mantissa: i128, mut scale: u32) -> Decimal {
        while scale > 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        if mantissa == 0 {
            scale = 0;
        }
        Decimal { mantissa, scale }
    }

    pub(crate) fn from_i128(n: i128) -> Decimal {
        Decimal {
            mantissa: n,
            scale: 0,
        }
    }

    /// XML Schema `xs:decimal` lexical space: `[+-]? (digits ('.' digits?)? | '.' digits)`;
    /// no exponent, no whitespace (the caller collapses it).
    pub(crate) fn parse(text: &str) -> Result<Decimal, DecimalParseError> {
        let (negative, rest) = match text.as_bytes().first() {
            None => return Err(DecimalParseError::Invalid),
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            Some(_) => (false, text),
        };
        let (int_part, frac_part) = rest.split_once('.').unwrap_or((rest, ""));
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(DecimalParseError::Invalid);
        }
        if !int_part
            .bytes()
            .chain(frac_part.bytes())
            .all(|b| b.is_ascii_digit())
        {
            return Err(DecimalParseError::Invalid);
        }
        let mut mantissa: i128 = 0;
        for b in int_part.bytes().chain(frac_part.bytes()) {
            mantissa = mantissa
                .checked_mul(10)
                .and_then(|m| m.checked_add((b - b'0') as i128))
                .ok_or(DecimalParseError::TooPrecise)?;
        }
        if negative {
            mantissa = -mantissa;
        }
        let result = Decimal::normalized(mantissa, frac_part.len() as u32);
        if result.scale > MAX_SCALE {
            return Err(DecimalParseError::TooPrecise);
        }
        Ok(result)
    }

    /// The exact decimal expansion of the shortest decimal that round-trips
    /// `value` (what `xs:decimal(xs:double)` yields in practice). The caller
    /// excludes NaN and infinities; a value beyond 38 digits is `TooPrecise`.
    pub(crate) fn from_f64(value: f64) -> Result<Decimal, DecimalParseError> {
        // Rust's `Display` for f64 never uses an exponent.
        Decimal::parse(&format!("{value}"))
    }

    pub(crate) fn to_f64(self) -> f64 {
        self.to_string().parse().unwrap_or(f64::NAN)
    }

    pub(crate) fn is_zero(self) -> bool {
        self.mantissa == 0
    }

    pub(crate) fn signum(self) -> i32 {
        self.mantissa.signum() as i32
    }

    /// `Some(n)` when the value is a whole number.
    pub(crate) fn to_i128_exact(self) -> Option<i128> {
        if self.scale == 0 {
            Some(self.mantissa)
        } else {
            None
        }
    }

    /// The whole-number part, truncated toward zero.
    pub(crate) fn trunc(self) -> Decimal {
        if self.scale == 0 {
            return self;
        }
        match pow10(self.scale) {
            Some(p) => Decimal::normalized(self.mantissa / p, 0),
            None => Decimal::ZERO,
        }
    }

    pub(crate) fn negated(self) -> Decimal {
        Decimal {
            mantissa: -self.mantissa,
            scale: self.scale,
        }
    }

    pub(crate) fn abs(self) -> Decimal {
        Decimal {
            mantissa: self.mantissa.abs(),
            scale: self.scale,
        }
    }

    pub(crate) fn floor(self) -> Decimal {
        let t = self.trunc();
        if self.signum() < 0 && t != self {
            Decimal::normalized(t.mantissa - 1, 0)
        } else {
            t
        }
    }

    pub(crate) fn ceiling(self) -> Decimal {
        let t = self.trunc();
        if self.signum() > 0 && t != self {
            Decimal::normalized(t.mantissa + 1, 0)
        } else {
            t
        }
    }

    /// `fn:round` (F&O §6.4.4): nearest whole number, ties toward positive
    /// infinity.
    pub(crate) fn round(self) -> Decimal {
        let half = Decimal {
            mantissa: 5,
            scale: 1,
        };
        match self.checked_add(half) {
            Some(shifted) => shifted.floor(),
            None => self,
        }
    }

    fn aligned(self, other: Decimal) -> Option<(i128, i128, u32)> {
        let scale = self.scale.max(other.scale);
        let a = self.mantissa.checked_mul(pow10(scale - self.scale)?)?;
        let b = other.mantissa.checked_mul(pow10(scale - other.scale)?)?;
        Some((a, b, scale))
    }

    pub(crate) fn checked_add(self, other: Decimal) -> Option<Decimal> {
        let (a, b, scale) = self.aligned(other)?;
        Some(Decimal::normalized(a.checked_add(b)?, scale))
    }

    pub(crate) fn checked_sub(self, other: Decimal) -> Option<Decimal> {
        let (a, b, scale) = self.aligned(other)?;
        Some(Decimal::normalized(a.checked_sub(b)?, scale))
    }

    pub(crate) fn checked_mul(self, other: Decimal) -> Option<Decimal> {
        let mantissa = self.mantissa.checked_mul(other.mantissa)?;
        let result = Decimal::normalized(mantissa, self.scale + other.scale);
        (result.scale <= MAX_SCALE).then_some(result)
    }

    /// Division with 18 fractional digits, truncated. When the scaled dividend
    /// does not fit 38 digits, fewer fractional digits are kept (down to none).
    /// `None` for a zero
    /// divisor (the caller raises `FOAR0001`) and when not even the integer
    /// part fits.
    pub(crate) fn checked_div(self, other: Decimal) -> Option<Decimal> {
        if other.is_zero() {
            return None;
        }
        // quotient scale = self.scale - other.scale + k, aim for DIVISION_DIGITS.
        let mut k = (DIVISION_DIGITS + other.scale).saturating_sub(self.scale);
        loop {
            if let Some(numerator) = pow10(k).and_then(|p| self.mantissa.checked_mul(p)) {
                let quotient = numerator / other.mantissa;
                let scale = self.scale + k;
                // value = quotient / 10^(scale - other.scale)
                let result_scale = scale.checked_sub(other.scale)?;
                return Some(Decimal::normalized(quotient, result_scale));
            }
            if k == 0 {
                return None;
            }
            k -= 1;
        }
    }

    /// `idiv`: the quotient truncated toward zero, as a whole number.
    pub(crate) fn checked_idiv(self, other: Decimal) -> Option<Decimal> {
        if other.is_zero() {
            return None;
        }
        let (a, b, _) = self.aligned(other)?;
        Some(Decimal::normalized(a / b, 0))
    }

    /// `mod`: `self - other * trunc(self / other)`, the sign of the dividend.
    pub(crate) fn checked_rem(self, other: Decimal) -> Option<Decimal> {
        if other.is_zero() {
            return None;
        }
        let (a, b, scale) = self.aligned(other)?;
        Some(Decimal::normalized(a % b, scale))
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Decimal) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Decimal {}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Decimal) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Decimal) -> Ordering {
        match self.aligned(*other) {
            Some((a, b, _)) => a.cmp(&b),
            // Alignment overflowed: the one with the larger magnitude of
            // integer part decides; fall back to the sign, then to doubles.
            None => match self.signum().cmp(&other.signum()) {
                Ordering::Equal => self
                    .to_f64()
                    .partial_cmp(&other.to_f64())
                    .unwrap_or(Ordering::Equal),
                unequal => unequal,
            },
        }
    }
}

/// The canonical lexical form (XML Schema Part 2 §3.2.3.2): no exponent, no
/// superfluous zeros, no `.0` on whole numbers.
impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.mantissa.unsigned_abs().to_string();
        let sign = if self.mantissa < 0 { "-" } else { "" };
        if self.scale == 0 {
            return write!(f, "{sign}{digits}");
        }
        let scale = self.scale as usize;
        if digits.len() > scale {
            let (int_part, frac_part) = digits.split_at(digits.len() - scale);
            write!(f, "{sign}{int_part}.{frac_part}")
        } else {
            write!(f, "{sign}0.{}{digits}", "0".repeat(scale - digits.len()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::parse(s).unwrap()
    }

    // XML Schema Part 2 §3.2.3: decimal lexical space and canonical form.
    #[test]
    fn parses_and_prints_canonical_form() {
        assert_eq!(d("+001.500").to_string(), "1.5");
        assert_eq!(d("-.25").to_string(), "-0.25");
        assert_eq!(d("5.").to_string(), "5");
        assert_eq!(d("-0.0").to_string(), "0");
        assert!(Decimal::parse("1e3") == Err(DecimalParseError::Invalid));
        assert!(Decimal::parse(".") == Err(DecimalParseError::Invalid));
        assert!(Decimal::parse("") == Err(DecimalParseError::Invalid));
        assert!(Decimal::parse("1 2") == Err(DecimalParseError::Invalid));
    }

    // F&O §6.2: op:numeric-add on xs:decimal is exact (no binary rounding).
    #[test]
    fn arithmetic_is_exact() {
        assert_eq!(d("0.1").checked_add(d("0.2")).unwrap(), d("0.3"));
        assert_eq!(
            d("1.10").checked_mul(d("1.10")).unwrap().to_string(),
            "1.21"
        );
        assert_eq!(d("10").checked_sub(d("0.01")).unwrap().to_string(), "9.99");
    }

    // F&O §6.2.4 op:numeric-divide: decimal division keeps at least 18 digits.
    #[test]
    fn division_keeps_eighteen_digits() {
        assert_eq!(
            d("1").checked_div(d("3")).unwrap().to_string(),
            "0.333333333333333333"
        );
        assert_eq!(d("6").checked_div(d("4")).unwrap().to_string(), "1.5");
        assert!(d("1").checked_div(Decimal::ZERO).is_none());
    }

    // F&O §6.2.5 op:numeric-integer-divide and §6.2.6 op:numeric-mod.
    #[test]
    fn idiv_and_mod_follow_the_dividend() {
        assert_eq!(d("7.5").checked_idiv(d("2")).unwrap().to_string(), "3");
        assert_eq!(d("-7.5").checked_idiv(d("2")).unwrap().to_string(), "-3");
        assert_eq!(d("-7.5").checked_rem(d("2")).unwrap().to_string(), "-1.5");
    }

    // F&O §6.4.2-§6.4.4: ceiling, floor, round (ties toward +infinity).
    #[test]
    fn rounding_functions() {
        assert_eq!(d("2.5").round().to_string(), "3");
        assert_eq!(d("-2.5").round().to_string(), "-2");
        assert_eq!(d("-2.6").round().to_string(), "-3");
        assert_eq!(d("-1.2").floor().to_string(), "-2");
        assert_eq!(d("-1.2").ceiling().to_string(), "-1");
        assert_eq!(d("1.2").ceiling().to_string(), "2");
    }

    #[test]
    fn comparison_ignores_scale() {
        assert_eq!(d("1.0"), d("1"));
        assert!(d("-0.5") < d("0.25"));
    }

    // One correctly rounded conversion, not mantissa / 10^scale in two roundings.
    #[test]
    fn to_f64_is_correctly_rounded() {
        assert_eq!(d("0.000000000000000000000000000001").to_f64(), 1e-30);
        assert_eq!(d("0.1").to_f64(), 0.1);
        assert_eq!(d("-12345.678").to_f64(), -12345.678);
    }

    #[test]
    fn more_than_38_places_is_too_precise_not_invalid() {
        let places = format!("0.{}1", "0".repeat(38));
        assert_eq!(Decimal::parse(&places), Err(DecimalParseError::TooPrecise));
        assert_eq!(
            Decimal::parse(&"9".repeat(40)),
            Err(DecimalParseError::TooPrecise)
        );
        assert!(Decimal::parse(&format!("0.{}1", "0".repeat(37))).is_ok());
    }

    #[test]
    fn overflow_is_reported() {
        let big = d("99999999999999999999999999999999999999");
        assert!(big.checked_mul(d("10")).is_none());
        assert!(big.checked_add(big).is_none());
    }
}
