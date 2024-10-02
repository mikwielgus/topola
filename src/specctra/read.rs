use super::common::ListToken;
use super::structure::Parser;
use thiserror::Error;
use utf8_chars::BufReadCharsExt;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("unexpected end of file")]
    Eof,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("expected {0}")]
    Expected(&'static str),
    #[error("expected ({0}")]
    ExpectedStartOfList(&'static str),
}

impl ParseError {
    pub fn add_context(self, context: (usize, usize)) -> ParseErrorContext {
        ParseErrorContext {
            error: self,
            context,
        }
    }
}

#[derive(Error, Debug)]
#[error("line {}, column {}: {error}", .context.0, .context.1)]
pub struct ParseErrorContext {
    error: ParseError,
    context: (usize, usize),
}

pub struct InputToken {
    pub token: ListToken,
    context: (usize, usize),
}

impl InputToken {
    pub fn new(token: ListToken, context: (usize, usize)) -> Self {
        Self { token, context }
    }

    pub fn expect_start(self, name: &'static str) -> Result<(), ParseErrorContext> {
        self.token
            .expect_start(name)
            .map_err(|err| err.add_context(self.context))
    }

    pub fn expect_any_start(self) -> Result<String, ParseErrorContext> {
        self.token
            .expect_any_start()
            .map_err(|err| err.add_context(self.context))
    }

    pub fn expect_leaf(self) -> Result<String, ParseErrorContext> {
        self.token
            .expect_leaf()
            .map_err(|err| err.add_context(self.context))
    }

    pub fn expect_end(self) -> Result<(), ParseErrorContext> {
        self.token
            .expect_end()
            .map_err(|err| err.add_context(self.context))
    }
}

pub trait ReadDsn<R: std::io::BufRead>: Sized {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext>;
}
// custom impl feeding the read values back into the tokenizer
impl<R: std::io::BufRead> ReadDsn<R> for Parser {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext> {
        Ok(Self {
            string_quote: tokenizer
                .read_optional("string_quote")?
                .inspect(|v| tokenizer.quote_char = Some(*v)),
            space_in_quoted_tokens: tokenizer
                .read_optional("space_in_quoted_tokens")?
                .inspect(|v| tokenizer.space_in_quoted = *v),
            host_cad: tokenizer.read_optional("host_cad")?,
            host_version: tokenizer.read_optional("host_version")?,
        })
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for String {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext> {
        tokenizer.consume_token()?.expect_leaf()
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for char {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext> {
        let string = tokenizer.consume_token()?.expect_leaf()?;
        if string.chars().count() == 1 {
            Ok(string.chars().next().unwrap())
        } else {
            Err(tokenizer.add_context(ParseError::Expected("a single character")))
        }
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for bool {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext> {
        match tokenizer.consume_token()?.expect_leaf()?.as_str() {
            "on" => Ok(true),
            "off" => Ok(false),
            _ => Err(tokenizer.add_context(ParseError::Expected("boolean"))),
        }
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for i32 {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext> {
        tokenizer
            .consume_token()?
            .expect_leaf()?
            .parse()
            .map_err(|_| tokenizer.add_context(ParseError::Expected("i32")))
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for u32 {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext> {
        tokenizer
            .consume_token()?
            .expect_leaf()?
            .parse()
            .map_err(|_| tokenizer.add_context(ParseError::Expected("u32")))
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for usize {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext> {
        tokenizer
            .consume_token()?
            .expect_leaf()?
            .parse()
            .map_err(|_| tokenizer.add_context(ParseError::Expected("usize")))
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for f32 {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext> {
        tokenizer
            .consume_token()?
            .expect_leaf()?
            .parse()
            .map_err(|_| tokenizer.add_context(ParseError::Expected("f32")))
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for f64 {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseErrorContext> {
        tokenizer
            .consume_token()?
            .expect_leaf()?
            .parse()
            .map_err(|_| tokenizer.add_context(ParseError::Expected("f64")))
    }
}

pub struct ListTokenizer<R: std::io::BufRead> {
    reader: R,
    peeked_char: Option<char>,
    cached_token: Option<InputToken>,
    space_in_quoted: bool,
    quote_char: Option<char>,
    line: usize,
    column: usize,
}

impl<R: std::io::BufRead> ListTokenizer<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            peeked_char: None,
            cached_token: None,
            space_in_quoted: false,
            quote_char: None,
            line: 1,
            column: 0,
        }
    }

    pub fn context(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    fn add_context(&self, error: ParseError) -> ParseErrorContext {
        ParseErrorContext {
            error,
            context: (self.line, self.column),
        }
    }

    fn map_context<T>(&self, result: Result<T, ParseError>) -> Result<T, ParseErrorContext> {
        result.map_err(|err| self.add_context(err))
    }

    fn next_char(&mut self) -> Result<char, ParseErrorContext> {
        let return_chr = self.peek_char()?;
        self.peeked_char = None;

        if return_chr == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }

        Ok(return_chr)
    }

    fn peek_char(&mut self) -> Result<char, ParseErrorContext> {
        Ok(if let Some(chr) = self.peeked_char {
            chr
        } else {
            let chr = self
                .reader
                .read_char()
                .transpose()
                .ok_or(self.add_context(ParseError::Eof))?
                .map_err(|err| self.add_context(err.into()))?;
            self.peeked_char = Some(chr);
            chr
        })
    }

    fn skip_whitespace(&mut self) -> Result<(), ParseErrorContext> {
        loop {
            let chr = self.peek_char()?;
            if chr == ' ' || chr == '\r' || chr == '\n' {
                self.next_char().unwrap();
            } else {
                return Ok(());
            }
        }
    }

    fn read_string(&mut self) -> Result<String, ParseErrorContext> {
        if let Some(chr) = self.quote_char {
            if chr == self.peek_char()? {
                return self.read_quoted();
            }
        }
        self.read_unquoted()
    }

    fn read_unquoted(&mut self) -> Result<String, ParseErrorContext> {
        let mut string = String::new();

        loop {
            let chr = self.peek_char()?;
            if chr == ' ' || chr == '(' || chr == ')' || chr == '\r' || chr == '\n' {
                break;
            }
            string.push(self.next_char().unwrap());
        }

        if string.is_empty() {
            Err(self.add_context(ParseError::Expected("string (unquoted)")))
        } else {
            Ok(string)
        }
    }

    fn read_quoted(&mut self) -> Result<String, ParseErrorContext> {
        let mut string = String::new();

        if self.next_char().unwrap() != self.quote_char.unwrap() {
            panic!();
        }

        loop {
            let chr = self.peek_char()?;
            if !self.space_in_quoted && chr == ' ' {
                panic!("found a space inside a quoted string, but file didn't declare this possibility");
            }
            if chr == self.quote_char.unwrap() {
                self.next_char().unwrap();
                break;
            }
            string.push(self.next_char().unwrap());
        }

        Ok(string)
    }

    // the following two methods effectively allow 1 token of lookahead

    // returns next token, either a cached one returned earlier or a newly read one
    pub fn consume_token(&mut self) -> Result<InputToken, ParseErrorContext> {
        // move out of cache if not empty, otherwise consume input
        // always leaves cache empty
        if let Some(token) = self.cached_token.take() {
            Ok(token)
        } else {
            let token = self.read_token()?;
            Ok(token)
        }
    }

    // puts a token back into cache, to be consumed by something else
    pub fn return_token(&mut self, token: InputToken) {
        self.cached_token = Some(token);
    }

    fn read_token(&mut self) -> Result<InputToken, ParseErrorContext> {
        self.skip_whitespace()?;
        let context = self.context();

        let chr = self.peek_char()?;
        Ok(InputToken::new(
            if chr == '(' {
                self.next_char().unwrap();
                self.skip_whitespace()?;
                ListToken::Start {
                    name: self.read_string()?,
                }
            } else if chr == ')' {
                self.next_char().unwrap();
                ListToken::End
            } else {
                ListToken::Leaf {
                    value: self.read_string()?,
                }
            },
            context,
        ))
    }

    pub fn read_value<T: ReadDsn<R>>(&mut self) -> Result<T, ParseErrorContext> {
        T::read_dsn(self)
    }

    pub fn read_named<T: ReadDsn<R>>(
        &mut self,
        name: &'static str,
    ) -> Result<T, ParseErrorContext> {
        self.consume_token()?.expect_start(name)?;
        let value = self.read_value::<T>()?;
        self.consume_token()?.expect_end()?;
        Ok(value)
    }

    pub fn read_optional<T: ReadDsn<R>>(
        &mut self,
        name: &'static str,
    ) -> Result<Option<T>, ParseErrorContext> {
        let input = self.consume_token()?;
        if let ListToken::Start {
            name: ref actual_name,
        } = input.token
        {
            if actual_name == name {
                let value = self.read_value::<T>()?;
                self.consume_token()?.expect_end()?;
                Ok(Some(value))
            } else {
                self.return_token(input);
                Ok(None)
            }
        } else {
            self.return_token(input);
            Ok(None)
        }
    }

    pub fn read_array<T: ReadDsn<R>>(&mut self) -> Result<Vec<T>, ParseErrorContext> {
        let mut array = Vec::<T>::new();
        loop {
            let input = self.consume_token()?;
            if let ListToken::Leaf { .. } = input.token {
                self.return_token(input);
                array.push(self.read_value::<T>()?);
            } else {
                self.return_token(input);
                break;
            }
        }
        Ok(array)
    }

    pub fn read_named_array<T: ReadDsn<R>>(
        &mut self,
        name: &'static str,
    ) -> Result<Vec<T>, ParseErrorContext> {
        self.read_array_with_alias(&[name])
    }

    pub fn read_array_with_alias<T: ReadDsn<R>>(
        &mut self,
        valid_names: &[&'static str],
    ) -> Result<Vec<T>, ParseErrorContext> {
        let mut array = Vec::<T>::new();
        loop {
            let input = self.consume_token()?;
            if let ListToken::Start {
                name: ref actual_name,
            } = input.token
            {
                if valid_names.contains(&actual_name.to_ascii_lowercase().as_ref()) {
                    let value = self.read_value::<T>()?;
                    self.consume_token()?.expect_end()?;
                    array.push(value);
                } else {
                    self.return_token(input);
                    break;
                }
            } else {
                self.return_token(input);
                break;
            }
        }
        Ok(array)
    }
}
