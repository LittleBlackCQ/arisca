use crate::circuit::{NetId, NetLit};
use crate::bipoly::{Polynomial, VarId};

use std::str::FromStr;
use num_bigint::BigInt;
use regex::Regex;
use log::warn;

#[derive(Debug, Clone)]
enum SpecExpr {
    Const(BigInt),
    Var { width: usize, offset: usize },
    Add(Box<SpecExpr>, Box<SpecExpr>),
    Mul(Box<SpecExpr>, Box<SpecExpr>),
}

pub struct ArithmeticSpec {
    root: Option<SpecExpr>,
    total_width: usize,
    is_signed: bool,
}

impl ArithmeticSpec {
    fn bits_to_poly_signed(vars: &[VarId], is_signed: bool) -> Polynomial {
        if let Some((&msb, rest)) = vars.split_last() {
            let weight = if is_signed { -1 } else { 1 };
            let init = Polynomial::var(msb, BigInt::from(weight));

            rest.iter().rev().fold(init, |acc, &var| {
                acc * Polynomial::constant(BigInt::from(2)) + Polynomial::var(var, BigInt::from(1))
            })
        } else {
            Polynomial::zero()
        }
    }

    fn parse_recursive(
        tokens: &[&str],
        min_prec: u8,
        cursor: &mut usize,
        offset: &mut usize
    ) -> Result<SpecExpr, String>{
        let peek = |pos: usize| tokens.get(pos).copied();
        let next = |pos: &mut usize| {
            let t = tokens.get(*pos).copied();
            if t.is_some() { *pos += 1; }
            t
        };

        let token = next(cursor).ok_or("Unexpected end of expression")?;

        let mut lhs = if token == "(" {
            let node = Self::parse_recursive(tokens, 0, cursor, offset)?;
            if next(cursor) != Some(")") {
                return Err("Missing ')'".to_string());
            }
            node
        } else if token.starts_with("[") {
            let width: usize = token[1..token.len()-1].parse().map_err(|_| "Invalid variable width!")?;

            let node = SpecExpr::Var { width, offset: *offset };
            *offset += width;
            node
        } else {
            let val = BigInt::from_str(token).map_err(|_| "Invalid constant!")?;
            SpecExpr::Const(val)
        };

        loop {
            let prec = match peek(*cursor) {
                Some("+") => 1,
                Some("*") => 2,
                _ => break,
            };
            if prec < min_prec {
                break;
            }
            let op = next(cursor).unwrap();
            let rhs = Self::parse_recursive(tokens, prec + 1, cursor, offset)?;
            lhs = match op {
                "+" => SpecExpr::Add(Box::new(lhs), Box::new(rhs)),
                "*" => SpecExpr::Mul(Box::new(lhs), Box::new(rhs)),
                _ => {
                    return Err(format!("Invalid operator: {}", op));
                }
            };
        }
        Ok(lhs)
    }

    fn eval_ast(&self, inputs: &[NetId], node: &SpecExpr, var: &[VarId]) -> Polynomial {
        match node {
            SpecExpr::Const(v) => Polynomial::constant(v.clone()),
            SpecExpr::Var { width, offset } => {
                let vars: Vec<VarId> = inputs[*offset .. offset + width]
                    .iter()
                    .map(|&net| var[net])
                    .collect();
                Self::bits_to_poly_signed(&vars, self.is_signed)
            }
            SpecExpr::Add(l, r) => self.eval_ast(inputs, l, var) + self.eval_ast(inputs, r, var),
            SpecExpr::Mul(l, r) => self.eval_ast(inputs, l, var) * self.eval_ast(inputs, r, var),
        }
    }

    fn parse_str(s: &str) -> Result<(SpecExpr, usize), String> {
        let re = Regex::new(r"\[(\d+)\]|(\d+)|([+*()])").map_err(|e| e.to_string())?;

        let mut tokens = Vec::new();
        let mut last_end = 0;

        for m in re.find_iter(s) {
            let skipped = &s[last_end..m.start()];
            if !skipped.trim().is_empty() {
                return Err(format!("Unexpected character(s) at index {}: '{}'", last_end, skipped.trim()));
            }

            tokens.push(m.as_str());
            last_end = m.end();
        }
        let trailing = &s[last_end..];
        if !trailing.trim().is_empty() {
            return Err(format!("Unexpected character(s) at the end: '{}'", trailing.trim()));
        }

        let mut cursor = 0;
        let mut offset = 0;

        let root = ArithmeticSpec::parse_recursive(&tokens, 0, &mut cursor, &mut offset)?;

        if cursor < tokens.len() {
            return Err("Unexpected tokens remaining".to_string());
        }
        Ok((root, offset))
    }

    pub fn new(spec_str: Option<&str>, is_signed: bool) -> Result<Self, String> {
        match spec_str {
            Some(s) => {
                let (root, width) = Self::parse_str(s)?;
                Ok(Self {
                    root: Some(root),
                    total_width: width,
                    is_signed,
                })
            },
            None => {
                // Enable Default Mode
                Ok(Self {
                    root: None,
                    total_width: 0,
                    is_signed,
                })
            }
        }
    }

    pub fn build_golden(&self, inputs: &[NetId], outputs: &[NetLit], vars: &[VarId]) -> Polynomial {
        let expected_poly = if let Some(root) = &self.root {
            // --- Mode 1: Explicit (AST) ---
            if inputs.len() != self.total_width {
                warn!("Input mismatch: Spec expects {} bits, Circuit has {}", self.total_width, inputs.len());
            }
            self.eval_ast(inputs, root, vars)
        } else {
            // --- Mode 2: Default (Auto Multiplier) ---
            let half = inputs.len() / 2;
            let a_nets = &inputs[..half];
            let b_nets = &inputs[half..];

            let a_vars: Vec<VarId> = a_nets.iter().map(|&n| vars[n]).collect();
            let b_vars: Vec<VarId> = b_nets.iter().map(|&n| vars[n]).collect();

            let poly_a = Self::bits_to_poly_signed(&a_vars, self.is_signed);
            let poly_b = Self::bits_to_poly_signed(&b_vars, self.is_signed);
            
            poly_a * poly_b
        };

        // Output processing (Applied to both modes)
        let out_vars: Vec<VarId> = outputs.iter().map(|o| vars[o.net()]).collect();
        let mut actual_poly = Self::bits_to_poly_signed(&out_vars, self.is_signed);

        for out in outputs {
            if out.negative() {
                actual_poly.neg_var(&vars[out.net()]);
            }
        }

        actual_poly - expected_poly
    }

    pub fn modulus(&self, outputs: &[NetLit]) -> Option<BigInt> {
        if let Some(root) = &self.root && matches!(root, SpecExpr::Const(_)) {
            None
        } else {
            Some(BigInt::from(1) << outputs.len())
        }
    }
}
