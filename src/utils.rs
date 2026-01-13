use num::ToPrimitive;
use num::complex::Complex64;

pub const EPSILON: f64 = 1e-10;

// lock_mutex removed (moved to bindings)

pub fn format_float(val: f64) -> String {
    if val.fract().abs() < EPSILON {
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

pub fn format_number(n: Number) -> String {
    match n {
        Number::Integer(i) => i.to_string(),
        Number::Rational(r) => {
            if r.is_integer() {
                r.to_integer().to_string()
            } else {
                format!("{}/{}", r.numer(), r.denom())
            }
        }
        Number::Float(f) => format_float(f),
        Number::Complex(c) => format_complex(c),
    }
}

pub fn format_number_decimal(n: Number) -> String {
    match n {
        Number::Rational(r) => {
            if r.is_integer() {
                r.to_integer().to_string()
            } else {
                let f = r.to_f64().unwrap_or(f64::NAN);
                format_float(f)
            }
        }
        // Delegate other types to the standard formatter or handle specifically if needed
        _ => format_number(n),
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
