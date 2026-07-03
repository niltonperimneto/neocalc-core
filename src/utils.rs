use num::complex::Complex64;

pub const EPSILON: f64 = 1e-10;

// lock_mutex removed (moved to bindings)

/// Largest magnitude rendered via the integer path: beyond 2^53 an f64 no
/// longer represents every integer exactly, and casting to i64 would saturate
/// (e.g. 1e300 must not print as 9223372036854775807).
const MAX_EXACT_INT_FLOAT: f64 = 9_007_199_254_740_992.0;

pub fn format_float(val: f64) -> String {
    if val.is_finite() && val.abs() < MAX_EXACT_INT_FLOAT && val.fract().abs() < EPSILON {
        (val.round() as i64).to_string()
    } else {
        val.to_string()
    }
}

use crate::engine::types::Number;

pub fn format_complex(c: Complex64) -> String {
    let re = c.re;
    let im = c.im;

    if im.abs() < EPSILON {
        format_float(re)
    } else {
        let re_str = format_float(re);
        let im_abs = im.abs();
        let im_str = format_float(im_abs);

        if re.abs() < EPSILON {
            if im < 0.0 {
                format!("-{}i", im_str)
            } else {
                format!("{}i", im_str)
            }
        } else {
            format!(
                "{} {} {}i",
                re_str,
                if im < 0.0 { "-" } else { "+" },
                im_str
            )
        }
    }
}

pub fn format_number(n: Number, use_decimals: bool) -> String {
    match n {
        Number::Integer(i) => i.to_string(),
        Number::Rational(r) => {
            if r.is_integer() {
                r.to_integer().to_string()
            } else if use_decimals {
                use num::ToPrimitive;
                format_float(r.to_f64().unwrap_or(f64::NAN))
            } else {
                format!("{}/{}", r.numer(), r.denom())
            }
        }
        Number::Float(f) => format_float(f),
        Number::Complex(c) => format_complex(c),
    }
}

pub fn map_input_token(text: &str) -> &str {
    match text {
        "÷" => "/",
        "×" => "*",
        "−" => "-",
        "π" => "pi",
        "√" => "sqrt(",
        _ => text,
    }
}

pub fn should_auto_paren(token: &str) -> bool {
    matches!(
        token,
        "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "sinh"
            | "cosh"
            | "tanh"
            | "log"
            | "ln"
            | "sqrt"
            | "abs"
    )
}
