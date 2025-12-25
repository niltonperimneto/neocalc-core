use super::types::Number;
use super::tokens::Token;
use super::errors::EngineError;
use super::ast::{Expr, BinaryOp, UnaryOp};
use logos::Logos;



/// Parses the expression into an Abstract Syntax Tree (AST).
/// Does NOT evaluate it.
pub fn parse(expression: &str) -> Result<Expr, EngineError> {
    /* Initialize the parser with the lexer directly */
    let mut parser = Parser::new(Token::lexer(expression).spanned());
    let result = parser.parse_bp(0)?;

    /* Ensure all tokens were consumed */
    if parser.current() != &Token::Eof {
        return Err(EngineError::ParserError(format!("Unexpected token at end: {:?}", parser.current())));
    }

    Ok(result)
}

struct Parser<'a> {
    lexer: logos::SpannedIter<'a, Token<'a>>,
    current: Token<'a>,
}

impl<'a> Parser<'a> {
    fn new(mut lexer: logos::SpannedIter<'a, Token<'a>>) -> Self {
        let current = fetch_next_token(&mut lexer);
        Parser { lexer, current }
    }

    fn current(&self) -> &Token<'a> {
        &self.current
    }

    fn advance(&mut self) {
        self.current = fetch_next_token(&mut self.lexer);
    }

    fn advance_with_token(&mut self) -> Token<'a> {
        let next = fetch_next_token(&mut self.lexer);
        std::mem::replace(&mut self.current, next)
    }

    /* Pratt parsing algorithm: Parse with a minimum binding power */
    fn parse_bp(&mut self, min_bp: u8) -> Result<Expr, EngineError> {
        let token = self.advance_with_token();

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
                    return Err(EngineError::ParserError("Expected ')'".to_string()));
                }
            }
            Token::Minus => {
                let ((), r_bp) = prefix_binding_power(&Token::Minus)?;
                let rhs = self.parse_bp(r_bp)?;
                Expr::UnaryOp(UnaryOp::Neg, Box::new(rhs))
            }
            Token::Eof => return Err(EngineError::ParserError("Unexpected EOF".to_string())),
            t => return Err(EngineError::ParserError(format!("Unexpected token: {:?}", t))),
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
                let token = self.advance_with_token();
                match token {
                    Token::Plus => BinaryOp::Add,
                    Token::Minus => BinaryOp::Sub,
                    Token::Multiply => BinaryOp::Mul,
                    Token::Divide => BinaryOp::Div,
                    Token::Power => BinaryOp::Pow,
                    Token::Percent => BinaryOp::Mod,
                    _ => return Err(EngineError::ParserError(format!("Unknown infix operator: {:?}", token))),
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
                _ => return Err(EngineError::ParserError("Expected ',' or ')' in argument list".to_string())),
            }
        }
        Ok(args)
    }
}

fn prefix_binding_power(op: &Token) -> Result<((), u8), EngineError> {
    match op {
        Token::Minus => Ok(((), 9)), // Unary minus
        _ => Err(EngineError::ParserError(format!("Bad prefix operator: {:?}", op))),
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

// Helper function to fetch the next token from the lexer
fn fetch_next_token<'a>(lexer: &mut logos::SpannedIter<'a, Token<'a>>) -> Token<'a> {
    match lexer.next() {
        Some((Ok(token), _)) => token,
        Some((Err(_), _)) => Token::Error, // Simple error token
        None => Token::Eof,
    }
}
