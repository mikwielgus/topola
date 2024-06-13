use super::common::ListToken;
use super::structure2::Parser;
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

pub trait ReadDsn<R: std::io::BufRead>: Sized {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError>;
}
// custom impl feeding the read values back into the tokenizer
impl<R: std::io::BufRead> ReadDsn<R> for Parser {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
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
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        Ok(tokenizer.consume_token()?.expect_leaf()?)
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for char {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        let string = tokenizer.consume_token()?.expect_leaf()?;
        if string.chars().count() == 1 {
            Ok(string.chars().next().unwrap())
        } else {
            Err(ParseError::Expected("a single character"))
        }
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for bool {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        match tokenizer.consume_token()?.expect_leaf()?.as_str() {
            "on" => Ok(true),
            "off" => Ok(false),
            _ => Err(ParseError::Expected("boolean")),
        }
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for i32 {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        Ok(tokenizer
            .consume_token()?
            .expect_leaf()?
            .parse()
            .map_err(|_| ParseError::Expected("i32"))?)
    }
}

impl<R: std::io::BufRead> ReadDsn<R> for u32 {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        Ok(tokenizer
            .consume_token()?
            .expect_leaf()?
            .parse()
            .map_err(|_| ParseError::Expected("u32"))?)
    }
}
impl<R: std::io::BufRead> ReadDsn<R> for usize {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        Ok(tokenizer
            .consume_token()?
            .expect_leaf()?
            .parse()
            .map_err(|_| ParseError::Expected("usize"))?)
    }
}
impl<R: std::io::BufRead> ReadDsn<R> for f32 {
    fn read_dsn(tokenizer: &mut ListTokenizer<R>) -> Result<Self, ParseError> {
        Ok(tokenizer
            .consume_token()?
            .expect_leaf()?
            .parse()
            .map_err(|_| ParseError::Expected("f32"))?)
    }
}

pub struct ListTokenizer<R: std::io::BufRead> {
    reader: R,
    peeked_char: Option<char>,
    cached_token: Option<ListToken>,
    space_in_quoted: bool,
    quote_char: Option<char>,
    line: usize,
    column: usize,
}

impl<R: std::io::BufRead> ListTokenizer<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: reader,
            peeked_char: None,
            cached_token: None,
            space_in_quoted: false,
            quote_char: None,
            line: 1,
            column: 0,
        }
    }

    fn next_char(&mut self) -> Result<char, ParseError> {
        let return_chr = if let Some(chr) = self.peeked_char {
            self.peeked_char = None;
            chr
        } else {
            self.reader.chars().next().ok_or(ParseError::Eof)??
        };

        if return_chr == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }

        Ok(return_chr)
    }

    fn peek_char(&mut self) -> Result<char, ParseError> {
        if let Some(chr) = self.peeked_char {
            Ok(chr)
        } else {
            let chr = self.reader.chars().next().ok_or(ParseError::Eof)??;
            self.peeked_char = Some(chr);
            Ok(chr)
        }
    }

    fn skip_whitespace(&mut self) -> Result<(), ParseError> {
        loop {
            let chr = self.peek_char()?;
            if chr == ' ' || chr == '\r' || chr == '\n' {
                self.next_char().unwrap();
            } else {
                return Ok(());
            }
        }
    }

    fn read_string(&mut self) -> Result<String, ParseError> {
        if let Some(chr) = self.quote_char {
            if chr == self.peek_char()? {
                return self.read_quoted();
            }
        }
        self.read_unquoted()
    }

    fn read_unquoted(&mut self) -> Result<String, ParseError> {
        let mut string = String::new();

        loop {
            let chr = self.peek_char()?;
            if chr == ' ' || chr == '(' || chr == ')' || chr == '\r' || chr == '\n' {
                break;
            }
            string.push(self.next_char().unwrap());
        }

        if string.is_empty() {
            Err(ParseError::Expected("string (unquoted)"))
        } else {
            Ok(string)
        }
    }

    fn read_quoted(&mut self) -> Result<String, ParseError> {
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
    pub fn consume_token(&mut self) -> Result<ListToken, ParseError> {
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
    pub fn return_token(&mut self, token: ListToken) {
        self.cached_token = Some(token);
    }

    fn read_token(&mut self) -> Result<ListToken, ParseError> {
        self.skip_whitespace()?;
        let chr = self.peek_char()?;
        Ok(if chr == '(' {
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
        })
    }

    pub fn read_value<T: ReadDsn<R>>(&mut self) -> Result<T, ParseError> {
        T::read_dsn(self)
    }

    pub fn read_named<T: ReadDsn<R>>(&mut self, name: &'static str) -> Result<T, ParseError> {
        self.consume_token()?.expect_start(name)?;
        let value = self.read_value::<T>()?;
        self.consume_token()?.expect_end()?;
        Ok(value)
    }

    pub fn read_optional<T: ReadDsn<R>>(
        &mut self,
        name: &'static str,
    ) -> Result<Option<T>, ParseError> {
        let token = self.consume_token()?;
        if let ListToken::Start {
            name: ref actual_name,
        } = token
        {
            if actual_name == name {
                let value = self.read_value::<T>()?;
                self.consume_token()?.expect_end()?;
                Ok(Some(value))
            } else {
                self.return_token(token);
                Ok(None)
            }
        } else {
            self.return_token(token);
            Ok(None)
        }
    }

    pub fn read_array<T: ReadDsn<R>>(&mut self) -> Result<Vec<T>, ParseError> {
        let mut array = Vec::<T>::new();
        loop {
            let token = self.consume_token()?;
            if let ListToken::Leaf { .. } = token {
                self.return_token(token);
                array.push(self.read_value::<T>()?);
            } else {
                self.return_token(token);
                break;
            }
        }
        Ok(array)
    }

    pub fn read_named_array<T: ReadDsn<R>>(
        &mut self,
        name: &'static str,
    ) -> Result<Vec<T>, ParseError> {
        let mut array = Vec::<T>::new();
        loop {
            let token = self.consume_token()?;
            if let ListToken::Start {
                name: ref actual_name,
            } = token
            {
                if actual_name == name {
                    let value = self.read_value::<T>()?;
                    self.consume_token()?.expect_end()?;
                    array.push(value);
                } else {
                    self.return_token(token);
                    break;
                }
            } else {
                self.return_token(token);
                break;
            }
        }
        Ok(array)
    }
}
