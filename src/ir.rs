use crate::error::Error;
use crate::lex::SourceToken;
use crate::parse::{Args, Specifier, Specifiers};
use displaydoc::Display;
use logos::{Lexer, Logos};
use std::fmt;
use std::ops::Range;

/// Intermediate representation for a parsed C file.
#[derive(Debug)]
pub struct IntermediateRepresentation<'src>(Interpolation<'src, Site<'src>>);

impl<'src> IntermediateRepresentation<'src> {
    /// Parse C source code into an [`IntermediateRepresentation`],
    /// otherwise return a list of [`Error`]s.
    pub fn parse(source: &'src str) -> Result<Self, Vec<Error>> {
        let mut lex = SourceToken::lexer(source);
        let mut span: Option<Range<usize>> = None;
        let mut pairs = Some(Vec::with_capacity(0));
        let mut errors = Vec::with_capacity(0);

        while let Some(token) = lex.next() {
            let (before, site) = match token {
                SourceToken::Identifier("printf") => {
                    let before = span
                        .as_ref()
                        .map(|span| &source[span.start..lex.span().start])
                        .unwrap_or("");

                    if lex.next() != Some(SourceToken::LParen) {
                        continue;
                    }

                    span = None;

                    let printf = parse_args(&mut lex, &mut errors)
                        .map(|([], format)| Site::Printf { format });

                    (before, printf)
                }
                SourceToken::Identifier("sprintf") => {
                    let before = span
                        .take()
                        .map(|span| &source[span.start..lex.span().start])
                        .unwrap_or("");

                    if lex.next() != Some(SourceToken::LParen) {
                        continue;
                    }

                    span = None;

                    let sprintf = parse_args(&mut lex, &mut errors)
                        .map(|([buffer], format)| Site::Sprintf { buffer, format });

                    (before, sprintf)
                }
                SourceToken::Identifier("snprintf") => {
                    let before = span
                        .take()
                        .map(|span| &source[span.start..lex.span().start])
                        .unwrap_or("");

                    if lex.next() != Some(SourceToken::LParen) {
                        continue;
                    }

                    span = None;

                    let snprintf =
                        parse_args(&mut lex, &mut errors).map(|([buffer, bufsz], format)| {
                            Site::Snprintf {
                                buffer,
                                bufsz,
                                format,
                            }
                        });

                    (before, snprintf)
                }
                // add other print kinds here
                _ => {
                    span = Some(match span {
                        Some(Range { start, .. }) => start..lex.span().end,
                        None => lex.span(),
                    });
                    continue;
                }
            };

            match (&mut pairs, site) {
                (Some(pairs), Some(site)) => {
                    pairs.push((before, site));
                }
                (_, None) => pairs = None,
                _ => { /* ignore */ }
            }
        }

        match pairs {
            Some(pairs) => Ok(Self(Interpolation::new(
                pairs,
                span.take().map(|span| &lex.source()[span]).unwrap_or(""),
            ))),
            None => Err(errors),
        }
    }

    /// Returns a displayable version of [`IntermediateRepresentation`] that
    /// replaces `printf` and family with optimized calls.
    pub fn display_optimize(&self) -> impl fmt::Display + '_ {
        DisplayIntermediateRepresentation {
            interpolation: &self.0,
            format_site: |site: &Site, f: &mut fmt::Formatter<'_>| -> fmt::Result {
                let format = match site {
                    Site::Printf { format } => {
                        f.write_str("safe_printf(")?;
                        format
                    }
                    Site::Sprintf { buffer, format } => {
                        write!(f, "safe_sprintf((char* restrict) ({buffer}), ")?;
                        format
                    }
                    Site::Snprintf {
                        buffer,
                        bufsz,
                        format,
                    } => {
                        write!(
                            f,
                            "safe_snprintf((char* restrict) ({buffer}), (size_t) ({bufsz}), "
                        )?;
                        format
                    }
                };

                write!(f, "{}", format.pairs.len() * 3 + 1)?;

                for (chunk, displayable) in format.pairs.iter() {
                    write!(
                        f,
                        ", \"{chunk}\", (void*) {}({}), {}",
                        if displayable.specifier.ctype != CType::String {
                            "&"
                        } else {
                            ""
                        },
                        displayable.arg,
                        displayable.specifier.ctype.format_fn()
                    )?;
                }

                write!(f, ", \"{}\")", format.last)
            },
        }
    }

    /// Returns a displayable version of [`IntermediateRepresentation`] that
    /// adds type casts to all function arguments..
    pub fn display_typecast(&self) -> impl fmt::Display + '_ {
        DisplayIntermediateRepresentation {
            interpolation: &self.0,
            format_site: |site: &Site, f: &mut fmt::Formatter<'_>| -> fmt::Result {
                let format = match site {
                    Site::Printf { format } => {
                        f.write_str("printf(\"")?;
                        format
                    }
                    Site::Sprintf { buffer, format } => {
                        write!(f, "sprintf((char* restrict) ({buffer}), \"")?;
                        format
                    }
                    Site::Snprintf {
                        buffer,
                        bufsz,
                        format,
                    } => {
                        write!(
                            f,
                            "snprintf((char* restrict) ({buffer}), (size_t) ({bufsz}), \""
                        )?;
                        format
                    }
                };

                // reconstruct the format string
                for (chunk, FormatValue { specifier, .. }) in format.pairs.iter() {
                    f.write_str(chunk)?;
                    write!(
                        f,
                        "%{}{}",
                        specifier.options,
                        specifier.ctype.specifier_char()
                    )?;
                }
                write!(f, "{}\"", format.last)?;

                // reconstruct the arguments, but with type casts now
                for (_, displayable) in format.pairs.iter() {
                    if displayable.type_checked {
                        write!(f, ", {}", displayable.arg)?;
                    } else {
                        write!(
                            f,
                            ", ({}) ({})",
                            displayable.specifier.ctype, displayable.arg
                        )?;
                    }
                }

                f.write_str(")")
            },
        }
    }
}

/// Displayable version of an [`IntermediateRepresentation`].
pub struct DisplayIntermediateRepresentation<'ir, 'src, F> {
    interpolation: &'ir Interpolation<'src, Site<'src>>,
    format_site: F,
}

impl<'ir, 'src, F> fmt::Display for DisplayIntermediateRepresentation<'ir, 'src, F>
where
    F: Fn(&'ir Site<'src>, &mut fmt::Formatter<'_>) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (chunk, site) in self.interpolation.pairs.iter() {
            chunk.fmt(f)?;
            (self.format_site)(site, f)?;
        }
        self.interpolation.last.fmt(f)
    }
}

/// Different callsites for string formatting in C.
#[derive(Debug)]
pub enum Site<'src> {
    /// printf
    Printf {
        format: Interpolation<'src, FormatValue<'src>>,
    },
    /// sprintf
    Sprintf {
        buffer: &'src str,
        format: Interpolation<'src, FormatValue<'src>>,
    },
    /// snprintf
    Snprintf {
        buffer: &'src str,
        bufsz: &'src str,
        format: Interpolation<'src, FormatValue<'src>>,
    },
}

/// Pair between an argument to be printed and the specifier that tells us
/// how it should be printed.
#[derive(Debug)]
pub struct FormatValue<'src> {
    /// The argument e.g. `name`.
    arg: &'src str,
    /// The argument was type casted the same type as the specifier expects.
    type_checked: bool,
    /// The specifier e.g. `%10s`.
    specifier: Specifier<'src>,
}

/// C types that can be formatted.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Display)]
pub enum CType {
    /// int
    Int,
    /// float
    Float,
    /// char*
    String,
}

impl CType {
    /// Character used that tells C how to format a value in a format string.
    pub fn specifier_char(&self) -> char {
        match self {
            CType::Int => 'd',
            CType::Float => 'f',
            CType::String => 's',
        }
    }

    /// Name of our function ptr that optimizes a print for a C type.
    pub fn format_fn(&self) -> &'static str {
        match self {
            CType::Int => "fmt_int",
            CType::Float => "fmt_float",
            CType::String => "fmt_string",
        }
    }
}

/// A set of string chunks and values that separate them.
#[derive(Debug)]
pub struct Interpolation<'src, T> {
    pairs: Vec<(&'src str, T)>,
    last: &'src str,
}

impl<'src, T> Interpolation<'src, T> {
    /// Returns a new [`Interpolation`].
    pub fn new(pairs: Vec<(&'src str, T)>, last: &'src str) -> Self {
        Self { pairs, last }
    }
}

/// Parses the arguments of any call to a string interpolating function,
/// otherwise pushes [`Error`]s to `errors` and returns `None`.
///
/// This function is also generic over `PRE_ARGS`, which is the number of arguments
/// to parse before the format string. For `printf`, this is 0, but for something
/// like `snprintf`, this is 2.
///
/// Note that even if errors occur and `None` is returned, the lexer will
/// still be moved to the end of the call.
///
/// # Example
///
/// ```c
/// snprintf(buffer, bufsz, "Total: $%d", (cost + fee) * tax);
/// //      ^                                               ^
/// //      assumes lexer starts here                       lexer ends up here
/// ```
pub fn parse_args<'src, const PRE_ARGS: usize>(
    lex: &mut Lexer<'src, SourceToken<'src>>,
    errors: &mut Vec<Error>,
) -> Option<(
    [&'src str; PRE_ARGS],
    Interpolation<'src, FormatValue<'src>>,
)> {
    let mut args = Args::new(lex);

    let mut pre_args = [""; PRE_ARGS];
    for pre_arg in pre_args.iter_mut() {
        let Some(arg) = args.next() else {
            errors.push(Error::MissingFunctionArgs(args.short_circuit().1));
            return None;
        };
        *pre_arg = args.source(arg.span);
    }

    let (format, format_span) = args
        .next_format_string()
        .map_err(|error| errors.push(error))
        .ok()?;

    let mut specifiers = Specifiers::new(format);
    let mut maybe_pairs = Some(Vec::with_capacity(4));

    loop {
        match (specifiers.next(), args.next()) {
            (Some(specifier), Some(arg)) => {
                match (&mut maybe_pairs, arg.cast) {
                    (Some(pairs), Some((cast_ctype, cast_span))) => {
                        if cast_ctype == specifier.ctype {
                            // passed typeck
                            pairs.push((
                                specifiers.before,
                                FormatValue {
                                    arg: args.source(arg.span),
                                    type_checked: true,
                                    specifier,
                                },
                            ));
                        } else {
                            // was okay, but just failed typeck
                            errors.push(Error::SpecifierCastMismatch {
                                specifier_span: specifiers.span(format_span.start + 1),
                                specifier_ctype: specifier.ctype,
                                cast_span,
                                cast_ctype,
                            });
                            maybe_pairs = None;
                        }
                    }
                    (Some(pairs), None) => {
                        // no type casting, skip typeck
                        pairs.push((
                            specifiers.before,
                            FormatValue {
                                arg: args.source(arg.span),
                                type_checked: false,
                                specifier,
                            },
                        ));
                    }
                    (None, Some((cast_ctype, cast_span))) => {
                        // already errored, maybe we can find a typeck mismatch
                        if cast_ctype != specifier.ctype {
                            // found one
                            errors.push(Error::SpecifierCastMismatch {
                                specifier_span: specifiers.span(format_span.start + 1),
                                specifier_ctype: specifier.ctype,
                                cast_span,
                                cast_ctype,
                            });
                        }
                    }
                    _ => { /* ignore  */ }
                }
            }
            (Some(_), None) => {
                // got a specifier but not an associated arg
                errors.push(Error::ExcessSpecifiers {
                    format_span,
                    args_span: args.short_circuit().1,
                    additional_specifiers: specifiers.count() + 1,
                });
                return None;
            }
            (None, Some(_)) => {
                // got an arg but not an associated specifier
                let (remaining, args_span) = args.short_circuit();
                errors.push(Error::ExcessArgs {
                    format_span,
                    args_span,
                    additional_args: remaining + 1,
                });
                return None;
            }
            (None, None) => {
                return Some((
                    pre_args,
                    Interpolation::new(maybe_pairs?, specifiers.remainder),
                ))
            }
        }
    }
}
