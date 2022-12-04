use crate::ir::CType;
use crate::parse::Specifier;
use logos::Logos;

#[derive(Debug, Clone, Copy, Logos, PartialEq, Eq)]
// char prefix
#[logos(subpattern cp = r"[uUL]")]
// string prefix
#[logos(subpattern sp = r"u8|(?&cp)")]
// white space
#[logos(subpattern ws = r"[ \t\v\r\n\f]")]
// escape sequence
#[logos(subpattern es = r#"[\\](['"%?\\abefnrtv]|[0-7]+|[xu][a-fA-F0-9]+|[\r]?[\n])"#)]
pub enum SourceToken {
    #[regex("//[^\r\n]*")]
    #[token("/*", |lex| {
        lex.bump(lex.remainder().find("*/")? + 2);
        None
    })]
    Comment,

    #[regex(r#"((?&sp)?"([^"\\\n]|(?&es))*"(?&ws)*)+"#)]
    String,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("printf")]
    Printf,

    #[token("sprintf")]
    Sprintf,

    #[token("snprintf")]
    Snprintf,

    #[regex(r"(?&ws)+", logos::skip)]
    Whitespace,

    #[error]
    Other,
}

#[derive(Debug, Logos)]
// octal digit
#[logos(subpattern o = "[0-7]")]
// decimal digit
#[logos(subpattern d = "[0-9]")]
// non-zero decimal digit
#[logos(subpattern nz = "[1-9]")]
// hexadecimal digit
#[logos(subpattern h = "[a-fA-F0-9]")]
// hexadecimal prefix
#[logos(subpattern hp = "0[xX]")]
// hexadecimal digit
#[logos(subpattern b = "[01]")]
// hexadecimal prefix
#[logos(subpattern bp = "0[bB]")]
// exponent
#[logos(subpattern e = "[eE][+-]?(?&d)+")]
#[logos(subpattern p = "[pP][+-]?(?&d)+")]
// float suffix
#[logos(subpattern fs = "[fFlL]")]
// integer suffix
#[logos(subpattern is = "([uU]([lL]|ll|LL)?)|(([lL]|ll|LL)[uU]?)")]
#[logos(subpattern l = "[a-zA-Z_$]")]
#[logos(subpattern a = "[a-zA-Z_$0-9]")]
// char prefix
#[logos(subpattern cp = r"[uUL]")]
// string prefix
#[logos(subpattern sp = r"u8|(?&cp)")]
// white space
#[logos(subpattern ws = r"[ \t\v\r\n\f]")]
// escape sequence
#[logos(subpattern es = r#"[\\](['"%?\\abefnrtv]|[0-7]+|[xu][a-fA-F0-9]+|[\r]?[\n])"#)]
pub enum ArgToken<'src> {
    #[regex("//[^\r\n]*")]
    #[token("/*", |lex| {
        lex.bump(lex.remainder().find("*/")? + 2);
        None
    })]
    Comment,

    #[regex(r"\.\.\.")]
    #[regex(r">>=|<<=|[+]=|-=|[*]=|/=|%=|&=|[\^]=|\|=")]
    #[regex(r">>|<<|[+][+]|--|->|&&|[|][|]|<=|>=|==|!=|<%|%>|<:|:>")]
    #[regex(r"[;{}:=\[\].&!~\-+*/%<>^|?\\#]")]
    Symbol,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token(",")]
    Comma,

    #[regex(r"(?&cp)?'([^'\\\n]|(?&es))*'")]
    Char,

    #[regex(r#"((?&sp)?"([^"\\\n]|(?&es))*"(?&ws)*)+"#, |lex| trim(lex.slice()))]
    String(&'src str),

    #[regex("((?&hp)(?&h)+|(?&bp)(?&b)+|(?&nz)(?&d)*|0(?&o)*)(?&is)?")]
    Int,

    #[regex("((?&d)+(?&e)|(?&d)*[.](?&d)+(?&e)?|(?&d)+[.](?&e)?|(?&hp)((?&h)+(?&p)|(?&h)*[.](?&h)+(?&p)|(?&h)+[.](?&p)))(?&fs)?")]
    Float,

    #[token("(int)", |_| CType::Int)]
    #[token("(float)", |_| CType::Float)]
    #[token("(char*)", |_| CType::String)]
    TypeCast(CType),

    #[regex("(?&l)(?&a)*")]
    Identifier(&'src str),

    #[regex(r"(?&ws)+", logos::skip)]
    Whitespace,

    #[error]
    Unknown,
}

#[derive(Debug, Logos)]
#[logos(subpattern opts = r"[+-]?([0-9]+([.][0-9]*)?|[.][0-9]+)")]
pub enum FormatToken<'src> {
    #[regex(r"%(?&opts)?[di]", |lex| Specifier::new(trim(lex.slice()), CType::Int))]
    #[regex(r"%(?&opts)?s", |lex| Specifier::new(trim(lex.slice()), CType::String))]
    #[regex(r"%(?&opts)?f", |lex| Specifier::new(trim(lex.slice()), CType::Float))]
    Specifier(Specifier<'src>),

    #[error]
    #[regex("\\\\.")]
    Normal,
}

/// Trim first and last byte from a string
fn trim(s: &str) -> &str {
    &s[1..s.len() - 1]
}
