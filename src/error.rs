use crate::ir::CType;
use crate::lex::ArgToken;
use crate::parse::Arg;
use displaydoc::Display;
use miette::{Diagnostic, NamedSource};
use std::ops::Range;
use std::path::PathBuf;

/// A collections of things that went wrong while validating a file.
#[derive(Debug, Display, Diagnostic)]
#[displaydoc("Source code contains errors.")]
pub struct SourceErrors {
    /// Name and source code of the file.
    #[source_code]
    source: NamedSource,

    #[related]
    errors: Vec<Error>,
}

impl SourceErrors {
    /// Returns a new [`SourceErrors`]
    pub fn new(filename: PathBuf, source: String, errors: Vec<Error>) -> Self {
        Self {
            source: NamedSource::new(filename.to_string_lossy(), source),
            errors,
        }
    }
}

impl std::error::Error for SourceErrors {}

/// Error that may occur during validation.
#[derive(Debug, Display, Diagnostic)]
pub enum Error {
    /// Missing function arguments.
    #[diagnostic(help("Supply enough arguments for the function call."))]
    MissingFunctionArgs(#[label("not enough arguments in function call")] Range<usize>),

    /// Format string isn't a string literal, this is potentially an overflow vulnerability!
    NonliteralFormat {
        #[label("not a string literal")]
        span: Range<usize>,
        #[help]
        help: String,
    },

    /// Incorrect specifier for type casted argument.
    #[diagnostic(help("Change the specifier to `%{}`, or change the cast to `({specifier_ctype})`.", cast_ctype.specifier_char()))]
    SpecifierCastMismatch {
        #[label("format string expects `{specifier_ctype}` value")]
        specifier_span: Range<usize>,
        specifier_ctype: CType,

        #[label("argument is casted as `{cast_ctype}`")]
        cast_span: Range<usize>,
        cast_ctype: CType,
    },

    /// Excess specifiers, this will read arbitrary data off the stack!
    #[diagnostic(help("{}", help_excess_specifiers(*additional_specifiers)))]
    ExcessSpecifiers {
        #[label("{additional_specifiers} too many specifiers")]
        format_span: Range<usize>,

        #[label("not enough arguments")]
        args_span: Range<usize>,
        additional_specifiers: usize,
    },

    /// Excess arguments.
    #[diagnostic(help("{}", help_excess_args(*additional_args)))]
    ExcessArgs {
        #[label("not enough specifiers")]
        format_span: Range<usize>,

        #[label("{additional_args} too many arguments")]
        args_span: Range<usize>,
        additional_args: usize,
    },
}

impl Error {
    pub fn nonliteral(arg: Arg<'_>) -> Self {
        Self::NonliteralFormat {
            span: arg.span,
            help: match arg.single_token {
                Some(ArgToken::Identifier(ident)) => {
                    format!(r#"To safely print a string, use `printf("%s", {ident})` instead."#)
                }
                _ => r#"Use a string literal as the first argument, like `printf("hello")`."#
                    .to_string(),
            },
        }
    }
}

impl std::error::Error for Error {}

fn help_excess_args(count: usize) -> String {
    if count == 1 {
        "Add a specifier or remove an argument.".to_string()
    } else {
        format!("Add {count} specifiers or remove {count} arguments.")
    }
}

fn help_excess_specifiers(count: usize) -> String {
    if count == 1 {
        "Add an argument or remove a specifier.".to_string()
    } else {
        format!("Add {count} arguments or remove {count} specifiers.")
    }
}
