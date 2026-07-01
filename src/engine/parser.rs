use super::types::Number;
use super::tokens::Token;
use super::errors::EngineError;
use super::ast::{Expr, BinaryOp, UnaryOp};
use logos::Logos;
use std::ops::Range;

/// Maximum nesting depth for the recursive-descent parser. Guards against stack
/// overflow from pathological input like `((((…))))` or `2^2^2^…`. Tunable.
const MAX_PARSE_DEPTH: usize = 256;

/// Parses the expression into an Abstract Syntax Tree (AST).
/// Does NOT evaluate it.
pub fn parse(expression: &str) -> Result<Expr, EngineError> {
    /* Initialize the parser with the lexer directly */
    let mut parser = Parser::new(Token::lexer(expression).spanned());
    let result = parser.parse_bp(0)?;

    /* Ensure all tokens were consumed */
    if parser.current() != &Token::Eof {
        return Err(EngineError::ParserError(format!(
            "Unexpected token '{}' at {}",
            describe(parser.current()),
            pos_str(&parser.span)
        )));
    }

    Ok(result)
}

struct Parser<'a> {
    lexer: logos::SpannedIter<'a, Token<'a>>,
    current: Token<'a>,
    /// Span of `current` in the source; used to report error positions.
    span: Range<usize>,
    /// End offset of the last token consumed, used to give EOF a sensible position.
    last_end: usize,
    /// Current recursion depth of `parse_bp`, capped at `MAX_PARSE_DEPTH`.
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(mut lexer: logos::SpannedIter<'a, Token<'a>>) -> Self {
        let (current, span, last_end) = fetch_next_token(&mut lexer, 0);
        Parser { lexer, current, span, last_end, depth: 0 }
    }

    fn current(&self) -> &Token<'a> {
        &self.current
    }

    fn advance(&mut self) {
        let (token, span, last_end) = fetch_next_token(&mut self.lexer, self.last_end);
        self.current = token;
        self.span = span;
        self.last_end = last_end;
    }

    /// Consume the current token, returning it together with its span.
    fn advance_with_token(&mut self) -> (Token<'a>, Range<usize>) {
        let (token, span, last_end) = fetch_next_token(&mut self.lexer, self.last_end);
        self.last_end = last_end;
        let old_span = std::mem::replace(&mut self.span, span);
        let old_token = std::mem::replace(&mut self.current, token);
        (old_token, old_span)
    }

    /* Pratt parsing algorithm: Parse with a minimum binding power */
    fn parse_bp(&mut self, min_bp: u8) -> Result<Expr, EngineError> {
        // Bound recursion depth to keep deeply nested input from overflowing the stack.
        self.depth += 1;
        if self.depth > MAX_PARSE_DEPTH {
            self.depth -= 1;
            return Err(EngineError::ParserError(
                "expression nesting too deep".to_string(),
            ));
        }
        let result = self.parse_bp_inner(min_bp);
        self.depth -= 1;
        result
    }

    fn parse_bp_inner(&mut self, min_bp: u8) -> Result<Expr, EngineError> {
        let (token, token_span) = self.advance_with_token();

        /* Handle the prefix part (numbers, identifiers, parentheses, unary ops) */
        let mut lhs = match token {
            Token::Float(f) => Expr::Literal(Number::Float(f)),
            Token::Integer(i) => Expr::Literal(Number::Integer(i)),
            Token::Identifier(s) => self.handle_identifier(s.to_string())?,
            Token::LParen => {
                let val = self.parse_bp(0)?;
                if let Token::RParen = self.current() {
                    self.advance();
                    val
                } else {
                    return Err(EngineError::ParserError(format!(
                        "Expected ')' at {}",
                        pos_str(&self.span)
                    )));
                }
            }
            Token::Minus => {
                let ((), r_bp) = prefix_binding_power(&Token::Minus)?;
                let rhs = self.parse_bp(r_bp)?;
                Expr::UnaryOp(UnaryOp::Neg, Box::new(rhs))
            }
            Token::Eof => {
                return Err(EngineError::ParserError(format!(
                    "Unexpected end of input at {}",
                    pos_str(&token_span)
                )))
            }
            Token::Error => {
                return Err(EngineError::ParserError(format!(
                    "Invalid syntax at {}",
                    pos_str(&token_span)
                )))
            }
            t => {
                return Err(EngineError::ParserError(format!(
                    "Unexpected token '{}' at {}",
                    describe(&t),
                    pos_str(&token_span)
                )))
            }
        };

        /* Handle infix and postfix operators while their binding power is high enough */
        loop {
            let op = self.current();
            if let Token::Eof = op {
                break;
            }

            // Handle Postfix operators (Factorial)
            if let Token::Factorial = op {
                let l_bp = 11; // Postfix binding power
                if l_bp < min_bp {
                    break;
                }
                self.advance();
                lhs = Expr::UnaryOp(UnaryOp::Factorial, Box::new(lhs));
                continue;
            }

            // Check for explicit Infix or Implicit Multiplication
            let (is_explicit, l_bp, r_bp) = match infix_binding_power(op) {
                Some((l, r)) => (true, l, r),
                None => {
                    // Check for Implicit Multiplication:
                    if matches!(op, Token::LParen | Token::Identifier(_)) {
                        (false, 3, 4)
                    } else {
                        break;
                    }
                }
            };

            /* Stop if upcoming operator has lower precedence */
            if l_bp < min_bp {
                break;
            }

            let bin_op = if is_explicit {
                let (token, token_span) = self.advance_with_token();
                match token {
                    Token::Plus => BinaryOp::Add,
                    Token::Minus => BinaryOp::Sub,
                    Token::Multiply => BinaryOp::Mul,
                    Token::Divide => BinaryOp::Div,
                    Token::Power => BinaryOp::Pow,
                    Token::Percent => BinaryOp::Mod,
                    _ => {
                        return Err(EngineError::ParserError(format!(
                            "Unknown infix operator '{}' at {}",
                            describe(&token),
                            pos_str(&token_span)
                        )))
                    }
                }
            } else {
                BinaryOp::Mul
            };

            let rhs = self.parse_bp(r_bp)?;
            lhs = Expr::BinaryOp(bin_op, Box::new(lhs), Box::new(rhs));
        }

        Ok(lhs)
    }

    fn handle_identifier(&mut self, name: String) -> Result<Expr, EngineError> {
        match self.current() {
            Token::LParen => {
                /* Function call OR Function Definition: name(arg1, ...) = body */
                self.advance(); /* eat '(' */

                // We need to parse arguments. Accessing args logic.
                // If it's a definition, args must be identifiers.
                // But parse_arguments parses Exprs.
                // We can parse generic Exprs. If we hit '=', check if all args were Variables.
                let args = self.parse_arguments()?;

                if let Token::Equals = self.current() {
                    // Function Definition
                    self.advance(); // eat '='
                    let body = self.parse_bp(0)?; // Parse body

                    // Validate args are variables
                    let mut params = Vec::new();
                    for arg in args {
                        if let Expr::Variable(param_name) = arg {
                            params.push(param_name);
                        } else {
                            return Err(EngineError::ParserError("Function parameters must be identifiers".to_string()));
                        }
                    }
                    Ok(Expr::FunctionDef(name, params, Box::new(body)))
                } else {
                    // Function Call
                    Ok(Expr::FunctionCall(name, args))
                }
            }
            Token::Equals => {
                 // Assignment: name = expr
                 self.advance(); // eat '='
                 let expr = self.parse_bp(0)?;
                 Ok(Expr::Assignment(name, Box::new(expr)))
            }
            _ => {
                /* Variable access */
                Ok(Expr::Variable(name))
            }
        }
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expr>, EngineError> {
        let mut args = Vec::new();
        if let Token::RParen = self.current() {
            self.advance();
            return Ok(args);
        }

        loop {
            args.push(self.parse_bp(0)?);

            match self.current() {
                Token::Comma => {
                    self.advance();
                }
                Token::RParen => {
                    self.advance();
                    break;
                }
                _ => {
                    return Err(EngineError::ParserError(format!(
                        "Expected ',' or ')' in argument list at {}",
                        pos_str(&self.span)
                    )))
                }
            }
        }
        Ok(args)
    }
}

fn prefix_binding_power(op: &Token) -> Result<((), u8), EngineError> {
    match op {
        Token::Minus => Ok(((), 9)), // Unary minus
        _ => Err(EngineError::ParserError(format!("Bad prefix operator: {}", describe(op)))),
    }
}

fn infix_binding_power(op: &Token) -> Option<(u8, u8)> {
    match op {
        Token::Plus | Token::Minus => Some((1, 2)),
        Token::Multiply | Token::Divide | Token::Percent => Some((3, 4)),
        Token::Power => Some((6, 5)), // Right associative: 2^3^4 = 2^(3^4)
        _ => None,
    }
}

/// Format a span start as a human-readable position for error messages.
fn pos_str(span: &Range<usize>) -> String {
    format!("position {}", span.start)
}

/// A short, user-facing description of a token for error messages.
fn describe(token: &Token) -> String {
    match token {
        Token::Plus => "+".to_string(),
        Token::Minus => "-".to_string(),
        Token::Multiply => "*".to_string(),
        Token::Divide => "/".to_string(),
        Token::Power => "^".to_string(),
        Token::Factorial => "!".to_string(),
        Token::Percent => "%".to_string(),
        Token::LParen => "(".to_string(),
        Token::RParen => ")".to_string(),
        Token::Comma => ",".to_string(),
        Token::Equals => "=".to_string(),
        Token::Float(f) => f.to_string(),
        Token::Integer(i) => i.to_string(),
        Token::Identifier(s) => s.to_string(),
        Token::Eof => "end of input".to_string(),
        Token::Error => "invalid token".to_string(),
    }
}

// Helper function to fetch the next token from the lexer, returning it with its span.
// `prev_end` is used to give the EOF token a sensible position.
fn fetch_next_token<'a>(
    lexer: &mut logos::SpannedIter<'a, Token<'a>>,
    prev_end: usize,
) -> (Token<'a>, Range<usize>, usize) {
    match lexer.next() {
        Some((Ok(token), span)) => {
            let end = span.end;
            (token, span, end)
        }
        Some((Err(_), span)) => {
            let end = span.end;
            (Token::Error, span, end) // Lexer error becomes an Error token
        }
        None => (Token::Eof, prev_end..prev_end, prev_end),
    }
}
