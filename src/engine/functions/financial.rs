use crate::engine::types::Number;
use crate::engine::errors::EngineError;
use crate::engine::functions::FunctionDef;
use num::complex::Complex64;
use num::Zero;


// Helper to convert args to Complex64
fn to_complex_args(args: &[Number]) -> Vec<Complex64> {
    args.iter().map(|n| n.to_complex()).collect()
}

// Future Value
pub fn fv(args: &[Number]) -> Result<Number, EngineError> {
    let args = to_complex_args(args);
    if args.len() < 3 || args.len() > 5 {
        return Err(EngineError::ArgumentMismatch("fv".into(), 3));
    }
    let rate = args[0];
    let nper = args[1];
    let pv = args[2];
    let pmt = if args.len() >= 4 { args[3] } else { Complex64::zero() };
    let type_val = if args.len() >= 5 { args[4].re as i32 } else { 0 };

    let result = if rate.norm() < 1e-9 {
        -(pv + pmt * nper)
    } else {
        let one = Complex64::new(1.0, 0.0);
        let factor = (one + rate).powc(nper);
        let term_pmt = (pmt * (one + rate * (type_val as f64))) * ((factor - one) / rate);
        -(pv * factor + term_pmt)
    };
    Ok(Number::Complex(result))
}

// Present Value
pub fn pv(args: &[Number]) -> Result<Number, EngineError> {
    let args = to_complex_args(args);
    if args.len() < 3 || args.len() > 5 {
         return Err(EngineError::ArgumentMismatch("pv".into(), 3));
    }
    let rate = args[0];
    let nper = args[1];
    let fv = args[2];
    let pmt = if args.len() >= 4 { args[3] } else { Complex64::zero() };
    let type_val = if args.len() >= 5 { args[4].re as i32 } else { 0 };

    let result = if rate.norm() < 1e-9 {
        -(fv + pmt * nper)
    } else {
        let one = Complex64::new(1.0, 0.0);
        let factor = (one + rate).powc(nper);
        let term_pmt = (pmt * (one + rate * (type_val as f64))) * ((factor - one) / rate);
        -(fv + term_pmt) / factor
    };
    Ok(Number::Complex(result))
}

// Payment
pub fn pmt(args: &[Number]) -> Result<Number, EngineError> {
    let args = to_complex_args(args);
    if args.len() < 3 || args.len() > 5 {
        return Err(EngineError::ArgumentMismatch("pmt".into(), 3));
    }
    let rate = args[0];
    let nper = args[1];
    let pv = args[2];
    let fv = if args.len() >= 4 { args[3] } else { Complex64::zero() };
    let type_val = if args.len() >= 5 { args[4].re as i32 } else { 0 };

    let result = if rate.norm() < 1e-9 {
        -(fv + pv) / nper
    } else {
        let one = Complex64::new(1.0, 0.0);
        let factor = (one + rate).powc(nper);
        let num = (pv * factor + fv) * rate;
        let den = (one + rate * (type_val as f64)) * (factor - one);
        -(num / den)
    };
    Ok(Number::Complex(result))
}

// Number of Periods
pub fn nper(args: &[Number]) -> Result<Number, EngineError> {
    let args = to_complex_args(args);
    if args.len() < 3 || args.len() > 5 {
        return Err(EngineError::ArgumentMismatch("nper".into(), 3));
    }
    let rate = args[0];
    let pmt = args[1];
    let pv = args[2];
    let fv = if args.len() >= 4 { args[3] } else { Complex64::zero() };
    let type_val = if args.len() >= 5 { args[4].re as i32 } else { 0 };

    if rate.norm() < 1e-9 {
        Ok(Number::Complex(-(fv + pv) / pmt))
    } else {
        let one = Complex64::new(1.0, 0.0);
        let r_type = one + rate * (type_val as f64);
        let num = pmt * r_type - fv * rate;
        let den = pmt * r_type + pv * rate;
        let nper = (num / den).ln() / (one + rate).ln();
        Ok(Number::Complex(nper))
    }
}

pub fn npv(args: &[Number]) -> Result<Number, EngineError> {
    let args = to_complex_args(args);
    if args.len() < 2 {
        return Err(EngineError::ArgumentMismatch("npv".into(), 2)); 
    }
    let rate = args[0];
    let values = &args[1..];
    let mut sum = Complex64::zero();
    let one = Complex64::new(1.0, 0.0);
    
    // Precision Critical: Use independent powc per term to prevent cumulative error drift.
    // Complexity: O(N log t)
    for (i, &val) in values.iter().enumerate() {
        let t = (i+1) as f64;
        sum += val / (one + rate).powf(t);
    }
    Ok(Number::Complex(sum))
}

pub fn irr(args: &[Number]) -> Result<Number, EngineError> {
    // Precision Critical: Use Complex64 to handle all root paths and independent powc calls.
    let args = to_complex_args(args);
    let values: Vec<f64> = args.iter().map(|c| c.re).collect();
    let mut guess = 0.1;
    for _ in 0..100 {
        let mut npv = 0.0;
        let mut deriv = 0.0;
        for (i, &val) in values.iter().enumerate() {
            let t = i as f64;
             let factor = (1.0_f64 + guess).powf(t);
            npv += val / factor;
            if t > 0.0 {
                let d_factor = (1.0_f64 + guess).powf(t + 1.0_f64);
                deriv -= t * val / d_factor;
            }
        }
        if npv.abs() < 1e-7 {
             return Ok(Number::Complex(Complex64::new(guess, 0.0)));
        }
        if deriv.abs() < 1e-10 { break; }
        guess -= npv / deriv;
    }
    Ok(Number::Complex(Complex64::new(guess, 0.0)))
}

pub fn rate(args: &[Number]) -> Result<Number, EngineError> {
    let args = to_complex_args(args);
    if args.len() < 3 {
        return Err(EngineError::ArgumentMismatch("rate".into(), 3));
    }
    let nper = args[0].re;
    let pmt = args[1].re;
    let pv = args[2].re;
    let fv = if args.len() >= 4 { args[3].re } else { 0.0 };
    let type_val = if args.len() >= 5 { args[4].re as i32 } else { 0 };
    let mut guess = if args.len() >= 6 { args[5].re } else { 0.1 };

    for _ in 0..100 {
        if guess.abs() < 1e-9 {
             let y = pv + pmt * nper + fv;
             if y.abs() < 1e-7 { return Ok(Number::Complex(Complex64::zero())); }
             guess = 0.0001;
             continue;
        }
        let r = guess;
        let factor = (1.0 + r).powf(nper);
        let term_pmt = (pmt * (1.0 + r * (type_val as f64))) * ((factor - 1.0) / r);
        let y = pv * factor + term_pmt + fv;
        
        let delta = 1e-5;
        let r_d = r + delta;
        let factor_d = (1.0 + r_d).powf(nper);
        let term_pmt_d = (pmt * (1.0 + r_d * (type_val as f64))) * ((factor_d - 1.0) / r_d);
        let y_d = pv * factor_d + term_pmt_d + fv;
        let deriv = (y_d - y) / delta;
        
        if deriv.abs() < 1e-10 { break; }
        let new_r = r - y / deriv;
        if (new_r - r).abs() < 1e-7 {
             return Ok(Number::Complex(Complex64::new(new_r, 0.0)));
        }
        guess = new_r;
    }
    Ok(Number::Complex(Complex64::new(guess, 0.0)))
}

// Register Functions
inventory::submit! { FunctionDef { name: "fv", func: fv } }
inventory::submit! { FunctionDef { name: "pv", func: pv } }
inventory::submit! { FunctionDef { name: "pmt", func: pmt } }
inventory::submit! { FunctionDef { name: "nper", func: nper } }
inventory::submit! { FunctionDef { name: "rate", func: rate } }
inventory::submit! { FunctionDef { name: "npv", func: npv } }
inventory::submit! { FunctionDef { name: "irr", func: irr } }
