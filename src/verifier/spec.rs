use crate::bipoly::{Polynomial, VarId};
use crate::circuit::{NetId, NetLit};

use log::warn;
use regex::Regex;
use rug::Integer;

#[derive(Debug, Clone)]
enum SpecExpr {
    Const(Integer),
    Var {
        width: usize,
        offset: usize,
        is_output: bool,
    },
    Add(Box<SpecExpr>, Box<SpecExpr>),
    Mul(Box<SpecExpr>, Box<SpecExpr>),
}

pub struct ArithmeticSpec {
    minuend: Option<SpecExpr>,
    subtrahend: Option<SpecExpr>,
    total_input_width: usize,
    total_output_width: usize,
    is_signed: bool,
}

impl ArithmeticSpec {
    fn bits_to_poly_signed(vars: &[VarId], is_signed: bool) -> Polynomial {
        if let Some((&msb, rest)) = vars.split_last() {
            let weight = if is_signed { -1 } else { 1 };
            let init = Polynomial::from_var(msb, Integer::from(weight));

            rest.iter().rev().fold(init, |acc, &var| {
                acc * Polynomial::from_constant(Integer::from(2))
                    + Polynomial::from_var(var, Integer::from(1))
            })
        } else {
            Polynomial::new()
        }
    }

    fn build_output_poly(outputs: &[NetLit], vars: &[VarId], is_signed: bool) -> Polynomial {
        let out_vars: Vec<VarId> = outputs.iter().map(|o| vars[o.net()]).collect();
        let mut poly = Self::bits_to_poly_signed(&out_vars, is_signed);
        for out in outputs {
            if out.negative() {
                poly.neg_var(&vars[out.net()]);
            }
        }
        poly
    }

    fn parse_recursive(
        tokens: &[&str],
        min_prec: u8,
        cursor: &mut usize,
        input_offset: &mut usize,
        max_input_offset: &mut usize,
        output_offset: &mut usize,
        max_output_offset: &mut usize,
    ) -> Result<SpecExpr, String> {
        let peek = |pos: usize| tokens.get(pos).copied();
        let next = |pos: &mut usize| {
            let t = tokens.get(*pos).copied();
            if t.is_some() {
                *pos += 1;
            }
            t
        };

        let token = next(cursor).ok_or("Unexpected end of expression")?;

        let mut lhs = if token == "(" {
            let node = Self::parse_recursive(
                tokens, 0, cursor,
                input_offset, max_input_offset,
                output_offset, max_output_offset,
            )?;
            if next(cursor) != Some(")") {
                return Err("Missing ')'".to_string());
            }
            node
        } else if token == "=" {
            return Err("Unexpected '=' in expression".to_string());
        } else if let Some(inner) = token.strip_prefix('[') {
            let inner = &inner[..inner.len() - 1];

            let (width_str, offset_str) = match inner.split_once(':') {
                Some((w, o)) => (w, Some(o)),
                None => (inner, None),
            };

            let width: usize = width_str.parse().map_err(|_| "Invalid variable width!")?;

            let current_offset = match offset_str {
                Some(o_str) => o_str.parse().map_err(|_| "Invalid explicit offset!")?,
                None => *input_offset,
            };

            let node = SpecExpr::Var {
                width,
                offset: current_offset,
                is_output: false,
            };

            *input_offset = current_offset + width;
            *max_input_offset = (*max_input_offset).max(*input_offset);

            node
        } else if let Some(inner) = token.strip_prefix("o[") {
            let inner = &inner[..inner.len() - 1];

            let (width_str, offset_str) = match inner.split_once(':') {
                Some((w, o)) => (w, Some(o)),
                None => (inner, None),
            };

            let width: usize = width_str.parse().map_err(|_| "Invalid variable width!")?;

            let current_offset = match offset_str {
                Some(o_str) => o_str.parse().map_err(|_| "Invalid explicit offset!")?,
                None => *output_offset,
            };

            let node = SpecExpr::Var {
                width,
                offset: current_offset,
                is_output: true,
            };

            *output_offset = current_offset + width;
            *max_output_offset = (*max_output_offset).max(*output_offset);

            node
        } else {
            let val = token.parse().map_err(|_| "Invalid constant!")?;
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
            let rhs = Self::parse_recursive(
                tokens, prec + 1, cursor,
                input_offset, max_input_offset,
                output_offset, max_output_offset,
            )?;
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

    fn tokenize(s: &str) -> Result<Vec<&str>, String> {
        let re = Regex::new(r"o\[\d+(?::\d+)?\]|\[\d+(?::\d+)?\]|\d+|[+*()=]")
            .map_err(|e| e.to_string())?;

        let mut tokens = Vec::new();
        let mut last_end = 0;

        for m in re.find_iter(s) {
            let skipped = &s[last_end..m.start()];
            if !skipped.trim().is_empty() {
                return Err(format!(
                    "Unexpected character(s) at index {}: '{}'",
                    last_end,
                    skipped.trim()
                ));
            }

            tokens.push(m.as_str());
            last_end = m.end();
        }
        let trailing = &s[last_end..];
        if !trailing.trim().is_empty() {
            return Err(format!(
                "Unexpected character(s) at the end: '{}'",
                trailing.trim()
            ));
        }
        Ok(tokens)
    }

    fn has_output_var(expr: &SpecExpr) -> bool {
        match expr {
            SpecExpr::Const(_) => false,
            SpecExpr::Var { is_output, .. } => *is_output,
            SpecExpr::Add(l, r) | SpecExpr::Mul(l, r) => {
                Self::has_output_var(l) || Self::has_output_var(r)
            }
        }
    }

    fn split_io_expr(expr: &SpecExpr) -> (Option<SpecExpr>, Option<SpecExpr>) {
        match expr {
            SpecExpr::Const(c) => (None, Some(SpecExpr::Const(c.clone()))),
            SpecExpr::Var { is_output: true, .. } => (Some(expr.clone()), None),
            SpecExpr::Var { is_output: false, .. } => (None, Some(expr.clone())),
            SpecExpr::Add(l, r) => {
                let (lo, li) = Self::split_io_expr(l);
                let (ro, ri) = Self::split_io_expr(r);
                let out = match (lo, ro) {
                    (Some(a), Some(b)) => Some(SpecExpr::Add(Box::new(a), Box::new(b))),
                    (x, None) | (None, x) => x,
                };
                let inp = match (li, ri) {
                    (Some(a), Some(b)) => Some(SpecExpr::Add(Box::new(a), Box::new(b))),
                    (x, None) | (None, x) => x,
                };
                (out, inp)
            }
            SpecExpr::Mul(_, _) => {
                if Self::has_output_var(expr) {
                    (Some(expr.clone()), None)
                } else {
                    (None, Some(expr.clone()))
                }
            }
        }
    }

    fn eval_ast(
        &self,
        inputs: &[NetId],
        outputs: &[NetLit],
        node: &SpecExpr,
        vars: &[VarId],
    ) -> Polynomial {
        match node {
            SpecExpr::Const(v) => Polynomial::from_constant(v.clone()),
            SpecExpr::Var {
                width,
                offset,
                is_output,
            } => {
                if *is_output {
                    let slice: Vec<VarId> = outputs[*offset..offset + width]
                        .iter()
                        .map(|o| vars[o.net()])
                        .collect();
                    let mut poly = Self::bits_to_poly_signed(&slice, self.is_signed);
                    for out in &outputs[*offset..offset + width] {
                        if out.negative() {
                            poly.neg_var(&vars[out.net()]);
                        }
                    }
                    poly
                } else {
                    let slice: Vec<VarId> = inputs[*offset..offset + width]
                        .iter()
                        .map(|&net| vars[net])
                        .collect();
                    Self::bits_to_poly_signed(&slice, self.is_signed)
                }
            }
            SpecExpr::Add(l, r) => {
                self.eval_ast(inputs, outputs, l, vars)
                    + self.eval_ast(inputs, outputs, r, vars)
            }
            SpecExpr::Mul(l, r) => {
                self.eval_ast(inputs, outputs, l, vars)
                    * self.eval_ast(inputs, outputs, r, vars)
            }
        }
    }

    fn parse_str(s: &str) -> Result<(Option<SpecExpr>, Option<SpecExpr>, usize, usize), String> {
        let tokens = Self::tokenize(s)?;

        if let Some(eq_pos) = tokens.iter().position(|&t| t == "=") {
            let (left_tokens, right_tokens) = (&tokens[..eq_pos], &tokens[eq_pos + 1..]);

            if left_tokens.is_empty() || right_tokens.is_empty() {
                return Err("Empty side in equation".to_string());
            }

            let mut input_offset = 0;
            let mut max_input_offset = 0;
            let mut output_offset = 0;
            let mut max_output_offset = 0;

            let mut cursor = 0;
            let minu = Self::parse_recursive(
                left_tokens, 0, &mut cursor,
                &mut input_offset, &mut max_input_offset,
                &mut output_offset, &mut max_output_offset,
            )?;
            if cursor < left_tokens.len() {
                return Err(format!(
                    "Unexpected token '{}' on left side of =",
                    left_tokens[cursor]
                ));
            }

            cursor = 0;
            let sub = Self::parse_recursive(
                right_tokens, 0, &mut cursor,
                &mut input_offset, &mut max_input_offset,
                &mut output_offset, &mut max_output_offset,
            )?;
            if cursor < right_tokens.len() {
                return Err(format!(
                    "Unexpected token '{}' on right side of =",
                    right_tokens[cursor]
                ));
            }

            Ok((Some(minu), Some(sub), max_input_offset, max_output_offset))
        } else {
            let any_o = tokens.iter().any(|t| t.starts_with("o["));

            let mut input_offset = 0;
            let mut max_input_offset = 0;
            let mut output_offset = 0;
            let mut max_output_offset = 0;

            let mut cursor = 0;
            let expr = Self::parse_recursive(
                &tokens, 0, &mut cursor,
                &mut input_offset, &mut max_input_offset,
                &mut output_offset, &mut max_output_offset,
            )?;

            if cursor < tokens.len() {
                return Err(format!("Unexpected token '{}'", tokens[cursor]));
            }

            if any_o {
                warn!(
                    "Output variables (o[...]) used without '='; expression will be split into \
                     output and input parts. Consider adding '=' for clarity."
                );

                let (minu, sub) = Self::split_io_expr(&expr);

                if minu.is_some() && sub.is_none() {
                    warn!("Only output variables found without '='; input side will use default interpretation.");
                }

                Ok((minu, sub, max_input_offset, max_output_offset))
            } else {
                Ok((None, Some(expr), max_input_offset, 0))
            }
        }
    }

    pub fn new(spec_str: Option<&str>, is_signed: bool) -> Result<Self, String> {
        match spec_str {
            Some(s) => {
                let (minuend, subtrahend, input_width, output_width) = Self::parse_str(s)?;
                Ok(Self {
                    minuend,
                    subtrahend,
                    total_input_width: input_width,
                    total_output_width: output_width,
                    is_signed,
                })
            }
            None => Ok(Self {
                minuend: None,
                subtrahend: None,
                total_input_width: 0,
                total_output_width: 0,
                is_signed,
            }),
        }
    }

    pub fn build_golden(
        &self,
        inputs: &[NetId],
        outputs: &[NetLit],
        vars: &[VarId],
    ) -> Polynomial {
        let minu_poly = match &self.minuend {
            Some(minu) => {
                if self.total_output_width > 0
                    && self.total_output_width != outputs.len()
                {
                    warn!(
                        "Output width mismatch: spec expects {} output bits, circuit has {}.",
                        self.total_output_width,
                        outputs.len()
                    );
                }
                self.eval_ast(inputs, outputs, minu, vars)
            }
            None => Self::build_output_poly(outputs, vars, self.is_signed),
        };

        let sub_poly = match &self.subtrahend {
            Some(sub) => {
                if self.total_input_width > 0
                    && self.total_input_width != inputs.len()
                {
                    warn!(
                        "Input width mismatch: spec expects {} input bits, circuit has {}.",
                        self.total_input_width,
                        inputs.len()
                    );
                }
                self.eval_ast(inputs, outputs, sub, vars)
            }
            None => {
                let half = inputs.len() / 2;
                let a_nets = &inputs[..half];
                let b_nets = &inputs[half..];

                let a_vars: Vec<VarId> = a_nets.iter().map(|&n| vars[n]).collect();
                let b_vars: Vec<VarId> = b_nets.iter().map(|&n| vars[n]).collect();

                let poly_a = Self::bits_to_poly_signed(&a_vars, self.is_signed);
                let poly_b = Self::bits_to_poly_signed(&b_vars, self.is_signed);

                poly_a * poly_b
            }
        };

        minu_poly - sub_poly
    }

    pub fn modulus(&self, outputs: &[NetLit]) -> Option<Integer> {
        let effective_width = if self.total_output_width > 0 {
            self.total_output_width
        } else {
            outputs.len()
        };

        if effective_width == 0 {
            return None;
        }

        let is_const_sub = match &self.subtrahend {
            Some(sub) => matches!(sub, SpecExpr::Const(_)),
            None => false,
        };

        if is_const_sub {
            None
        } else {
            Some(Integer::from(1) << effective_width)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_old_style() {
        let spec = ArithmeticSpec::new(Some("[8]*[8]"), false).unwrap();
        assert!(spec.minuend.is_none());
        assert!(spec.subtrahend.is_some());
        assert_eq!(spec.total_input_width, 16);
        assert_eq!(spec.total_output_width, 0);
    }

    #[test]
    fn test_equation_simple() {
        let spec = ArithmeticSpec::new(Some("o[250] = [127]*[129]"), true).unwrap();
        assert!(spec.minuend.is_some());
        assert!(spec.subtrahend.is_some());
        assert_eq!(spec.total_input_width, 256);
        assert_eq!(spec.total_output_width, 250);
    }

    #[test]
    fn test_output_without_eq() {
        let spec = ArithmeticSpec::new(Some("o[16]+o[16]"), false).unwrap();
        assert!(spec.minuend.is_some());
        assert!(spec.subtrahend.is_none());
        assert_eq!(spec.total_output_width, 32);
    }

    #[test]
    fn test_divider_spec() {
        let spec =
            ArithmeticSpec::new(Some("[32] = o[32]*[32] + o[32]"), false).unwrap();
        assert!(spec.minuend.is_some());
        assert!(spec.subtrahend.is_some());
        assert_eq!(spec.total_input_width, 64);
        assert_eq!(spec.total_output_width, 64);
    }

    #[test]
    fn test_no_spec() {
        let spec = ArithmeticSpec::new(None, false).unwrap();
        assert!(spec.minuend.is_none());
        assert!(spec.subtrahend.is_none());
    }

    #[test]
    fn test_explicit_offsets() {
        let spec =
            ArithmeticSpec::new(Some("o[32:32] + o[16:0] = [16:0]*[16:16]"), false).unwrap();
        assert!(spec.minuend.is_some());
        assert!(spec.subtrahend.is_some());
        assert_eq!(spec.total_output_width, 64);
        assert_eq!(spec.total_input_width, 32);
    }
}
