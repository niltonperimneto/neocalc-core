use logos::Logos;
use num_bigint::BigInt;

/* Define the tokens that can appear in an expression using the Logos lexer */
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token<'a> {
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Multiply,
    #[token("/")]
    Divide,
    #[token("^")]
    Power,
    #[token("!")]
    Factorial,
    #[token("%")]
    Percent, // Modulo
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(",")]
    Comma,
    #[token("=")]
    Equals,

    /* Match Floats: explicit dot or scientific notation */
    /* Needs to be checked BEFORE Integer to avoid greedy matching issues for things like 1.0 */
    /* Regex for float: digits dot digits (opt) exponent (opt) OR digits exponent */
    #[regex(r"[0-9]+\.[0-9]*([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    /* Match Integers: digits only */
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<BigInt>().ok())]
    #[regex(r"0x[0-9a-fA-F]+", |lex| BigInt::parse_bytes(&lex.slice()[2..].as_bytes(), 16))]
    #[regex(r"0b[01]+", |lex| BigInt::parse_bytes(&lex.slice()[2..].as_bytes(), 2))]
    Integer(BigInt),

    /* Match variable names or function identifiers */
    #[regex("[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice())]
    Identifier(&'a str),

    Eof,
    Error,
}
