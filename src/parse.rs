use crate::error::Error;
use crate::ir::CType;
use crate::lex::{ArgToken, FormatToken, SourceToken};
use logos::{Lexer, Logos};
use std::ops::Range;

/// An argument in a function call.
///
/// This type is returned by [`Args`] on iteration.
#[derive(Debug)]
pub struct Arg<'src> {
    /// The token, if there's exactly one (skipping comments and whitespaces)
    pub single_token: Option<ArgToken<'src>>,
    /// Range in source code
    pub span: Range<usize>,
    /// Type cast of the argument, if present
    pub cast: Option<(CType, Range<usize>)>,
}

/// [`Iterator`] over [`Arg`]s in `printf` call e.g. `"input"` and `"4"` in `"printf("%s %d", input, 4)"`.
#[derive(Debug)]
pub struct Args<'lex, 'src> {
    // hold onto source_lex so we can bump it when done parsing
    source_lex: &'lex mut Lexer<'src, SourceToken>,
    lex: Lexer<'src, ArgToken<'src>>,
    has_remaining: Option<()>,
    start: usize,
    end: usize,
}

impl<'lex, 'src> Args<'lex, 'src> {
    /// Returns a new [`Args`].
    pub fn new(source_lex: &'lex mut Lexer<'src, SourceToken>) -> Self {
        let mut lex = ArgToken::lexer(source_lex.source());
        let start = source_lex.span().end;
        lex.bump(start);
        Args {
            source_lex,
            lex,
            has_remaining: Some(()),
            start,
            end: start,
        }
    }

    /// Returns the number of remaining arguments, as well as their combined spans.
    pub fn short_circuit(mut self) -> (usize, Range<usize>) {
        let remaining = self.by_ref().count();
        (remaining, self.start..self.end)
    }

    pub fn source(&self, span: Range<usize>) -> &'src str {
        &self.source_lex.source()[span]
    }

    /// Parses the next argument as a format string, or returns an error.
    pub fn next_format_string(&mut self) -> Result<(&'src str, Range<usize>), Error> {
        match self.next() {
            Some(Arg {
                single_token: Some(ArgToken::String(format)),
                span,
                ..
            }) => Ok((format, span)),
            Some(arg) => Err(Error::nonliteral(arg)),
            None => Err(Error::MissingFunctionArgs(self.start..self.end)),
        }
    }
}

impl<'lex, 'src> Iterator for Args<'lex, 'src> {
    type Item = Arg<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        self.has_remaining?;

        let mut cast = None;
        let mut span = None;
        let mut opened = 0u32;
        let mut single_token = None;
        let mut count = 0u32;

        loop {
            match self.lex.next()? {
                ArgToken::Comma if opened == 0 => {
                    // parsed an argument, now expecting another
                    return Some(Arg {
                        single_token,
                        span: span?,
                        cast,
                    });
                }
                ArgToken::LParen => opened = opened.checked_add(1).expect("overflow"),
                ArgToken::RParen => match opened.checked_sub(1) {
                    Some(n) => opened = n,
                    None => {
                        // parsed the last argument
                        self.has_remaining = None;
                        self.end = self.lex.span().start;
                        self.source_lex.bump(self.end - self.start + 1);
                        return Some(Arg {
                            single_token,
                            span: span?,
                            cast,
                        });
                    }
                },
                ArgToken::TypeCast(ctype) if cast.is_none() => {
                    cast = Some((ctype, self.lex.span()))
                }
                token => {
                    single_token = (count == 0).then_some(token);
                    count += 1;
                }
            }

            span = Some(union(span, self.lex.span()));

            self.end = self.lex.span().end;
        }
    }
}

/// A specifier in a `printf` call.
///
/// This type is returned by [`Specifiers`] on iteration.
#[derive(Debug)]
pub struct Specifier<'src> {
    /// The `-2.3` part of `printf("%-2.3f", 3.141)`.
    pub options: &'src str,
    /// The C type corresponding to the specifier e.g. `float` for `%f`.
    pub ctype: CType,
}

impl<'src> Specifier<'src> {
    /// Returns a new [`Specifier`].
    pub fn new(options: &'src str, ctype: CType) -> Self {
        Self { options, ctype }
    }
}

/// [`Iterator`] over [`Specifier`]s in a format string.
#[derive(Debug)]
pub struct Specifiers<'src> {
    lex: Lexer<'src, FormatToken<'src>>,
    /// text between specifiers
    pub before: &'src str,
    /// text after last specifier
    pub remainder: &'src str,
}

impl<'src> Specifiers<'src> {
    pub fn new(format: &'src str) -> Self {
        Specifiers {
            lex: FormatToken::lexer(format),
            before: "",
            remainder: format,
        }
    }

    pub fn span(&self, format_offset: usize) -> Range<usize> {
        let span = self.lex.span();
        format_offset + span.start..format_offset + span.end
    }
}

impl<'src> Iterator for Specifiers<'src> {
    type Item = Specifier<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut span: Option<Range<usize>> = None;
        loop {
            if let FormatToken::Specifier(specifier) = self.lex.next()? {
                self.before = span.map(|s| &self.lex.source()[s]).unwrap_or("");
                self.remainder = self.lex.remainder();
                return Some(specifier);
            }

            span = Some(union(span, self.lex.span()));
        }
    }
}

fn union(span: Option<Range<usize>>, other: Range<usize>) -> Range<usize> {
    match span {
        Some(span) => span.start..other.end,
        None => other,
    }
}
