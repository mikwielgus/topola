use serde::de::{self, DeserializeSeed, SeqAccess, Visitor};
use serde::Deserialize;
use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("unexpected EOF")]
    Eof,
    #[error("expected boolean value")]
    ExpectedBool,
    #[error("expected quoted string")]
    ExpectedQuoted,
    #[error("spaces in quoted strings weren't declared")]
    SpaceInQuoted,
    #[error("expected unquoted string")]
    ExpectedUnquoted,
    #[error("expected opening parenthesis")]
    ExpectedOpeningParen,
    #[error("expected closing parenthesis")]
    ExpectedClosingParen,
    #[error("wrong struct, expected {0}")]
    ExpectedStruct(&'static str),
}

impl de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

struct Deserializer<'de> {
    input: &'de str,
    line: usize,
    column: usize,

    string_quote: Option<char>,
    space_in_quoted_tokens: bool,
    reconfig_incoming: Option<ReconfigIncoming>,

    next_option_empty_hint: bool,
    last_deserialized_type: Option<&'static str>,
}

#[derive(PartialEq, Debug, Copy, Clone)]
enum ReconfigIncoming {
    StringQuote,
    SpaceAllowed,
}

impl<'de> Deserializer<'de> {
    fn from_str(input: &'de str) -> Self {
        Self {
            input,
            line: 0,
            column: 0,

            string_quote: None,
            space_in_quoted_tokens: false,
            reconfig_incoming: None,

            next_option_empty_hint: false,
            last_deserialized_type: None,
        }
    }

    fn next_name_lookahead(&self) -> Option<String> {
        let mut iter = self.input.chars();
        if iter.next() != Some('(') {
            None
        } else {
            Some(
                iter.take_while(|c| c != &' ' && c != &'\r' && c != &'\n')
                    .collect::<String>(),
            )
        }
    }

    fn peek(&mut self) -> Result<char> {
        self.input.chars().next().ok_or(Error::Eof)
    }

    fn next(&mut self) -> Result<char> {
        let chr = self.peek()?;
        self.input = &self.input[1..];
        if chr == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
        Ok(chr)
    }

    fn skip_ws(&mut self) {
        while let Ok(chr) = self.peek() {
            if chr != ' ' && chr != '\r' && chr != '\n' {
                return;
            } else {
                self.next().unwrap();
            }
        }
        return;
    }

    fn parse_bool(&mut self) -> Result<bool> {
        match &self.parse_identifier() {
            Ok(string) => match string.as_str() {
                "on" => Ok(true),
                "off" => Ok(false),
                _ => Err(Error::ExpectedBool),
            },
            Err(_) => Err(Error::ExpectedBool),
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        self.parse_unquoted()
    }

    fn parse_string(&mut self) -> Result<String> {
        let chr = self.peek()?;
        if self.string_quote == Some(chr) {
            self.parse_quoted()
        } else {
            self.parse_unquoted()
        }
    }

    fn parse_unquoted(&mut self) -> Result<String> {
        let mut string = String::new();
        loop {
            let chr = self.peek()?;
            if chr != ' ' && chr != '\r' && chr != '\n' && chr != '(' && chr != ')' {
                string.push(self.next()?); // can't fail because of earlier peek
            } else {
                if string.len() > 0 {
                    return Ok(string);
                } else {
                    dbg!(self.line, self.column);
                    return Err(Error::ExpectedUnquoted);
                }
            }
        }
    }

    // method only called if parse_string sees a valid string quote
    fn parse_quoted(&mut self) -> Result<String> {
        assert!(self.next().unwrap() == self.string_quote.unwrap());

        let mut string = String::new();
        loop {
            let chr = self.peek()?;

            // XXX this is silly
            // not declaring that spaces are allowed in qyoted strings downgrades the format
            // but there's no reason we shouldn't try to parse the file anyway, no ambiguity arises
            // maybe this should log a warning and proceed?
            if self.space_in_quoted_tokens != true && chr == ' ' {
                return Err(Error::SpaceInQuoted);
            }

            if Some(chr) == self.string_quote {
                self.next().unwrap();
                return Ok(string);
            } else {
                string.push(self.next()?); // can't fail because of earlier peek
            }
        }
    }
}

pub fn from_str<'a, T>(input: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(input);
    let t = T::deserialize(&mut deserializer)?;
    if !deserializer.input.is_empty() {
        println!("remaining input");
        dbg!(deserializer.input);
    }
    Ok(t)
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_bool()?;
        self.skip_ws();

        // if the struct deserializer set a variable saying the incoming value should reconfigure a specific variable in the parser
        // we do so and clear the flag
        if self.reconfig_incoming == Some(ReconfigIncoming::SpaceAllowed) {
            self.space_in_quoted_tokens = value;
            self.reconfig_incoming = None;
        }

        self.last_deserialized_type = Some("");
        visitor.visit_bool(value)
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_unquoted()?;
        self.skip_ws();

        self.last_deserialized_type = Some("");
        visitor.visit_i32(value.parse().unwrap())
    }
    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_unquoted()?;
        self.skip_ws();

        self.last_deserialized_type = Some("");
        visitor.visit_u32(value.parse().unwrap())
    }
    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_unquoted()?;
        self.skip_ws();

        self.last_deserialized_type = Some("");
        visitor.visit_f32(value.parse().unwrap())
    }
    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let chr = self.next()?;
        self.skip_ws();

        // if the struct deserializer set a variable saying the incoming value should reconfigure a specific variable in the parser
        // we do so and clear the flag
        if self.reconfig_incoming == Some(ReconfigIncoming::StringQuote) {
            self.string_quote = Some(chr);
            self.reconfig_incoming = None;
        }

        self.last_deserialized_type = Some("");
        visitor.visit_char(chr)
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let string = self.parse_string()?;
        self.skip_ws();

        self.last_deserialized_type = Some("");
        visitor.visit_string(string)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.next_option_empty_hint {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.next()? != '(' {
            return Err(Error::ExpectedOpeningParen);
        }
        self.skip_ws();

        if &self.parse_identifier()? != name {
            return Err(Error::ExpectedStruct(name));
        }
        self.skip_ws();

        if self.next()? != ')' {
            return Err(Error::ExpectedClosingParen);
        }
        self.skip_ws();

        self.last_deserialized_type = Some(name);

        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.next()? != '(' {
            return Err(Error::ExpectedOpeningParen);
        }
        self.skip_ws();

        if &self.parse_identifier()? != name {
            return Err(Error::ExpectedStruct(name));
        }
        self.skip_ws();

        // if what we're deserializing is a directive to update parser configuration
        // set a variable so the deserializer for the following value can update the relevant config
        // (the variable is reset to None by the bool/char deserializer when it updates the config)
        self.reconfig_incoming = match name {
            "string_quote" => Some(ReconfigIncoming::StringQuote),
            "space_in_quoted_tokens" => Some(ReconfigIncoming::SpaceAllowed),
            _ => None,
        };

        let value = visitor.visit_seq(NewtypeStructFields::new(self))?;

        if self.next()? != ')' {
            return Err(Error::ExpectedClosingParen);
        }
        self.skip_ws();

        self.last_deserialized_type = Some(name);

        Ok(value)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.last_deserialized_type = None;
        visitor.visit_seq(ArrayIndices::new(self))
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.next()? != '(' {
            return Err(Error::ExpectedOpeningParen);
        }
        self.skip_ws();

        if &self.parse_identifier()? != name {
            return Err(Error::ExpectedStruct(name));
        }
        self.skip_ws();

        let value = visitor.visit_seq(StructFields::new(self, fields))?;

        if self.next()? != ')' {
            return Err(Error::ExpectedClosingParen);
        }
        self.skip_ws();

        // a hint for the array deserializer
        self.last_deserialized_type = Some(name);

        Ok(value)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.parse_string()?)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
}

struct NewtypeStructFields<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> NewtypeStructFields<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'a> SeqAccess<'de> for NewtypeStructFields<'a, 'de> {
    type Error = Error;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: DeserializeSeed<'de>,
    {
        if self.de.peek()? == ')' {
            return Ok(None);
        }

        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct ArrayIndices<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> ArrayIndices<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'a> SeqAccess<'de> for ArrayIndices<'a, 'de> {
    type Error = Error;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: DeserializeSeed<'de>,
    {
        if self.de.peek()? == ')' {
            return Ok(None);
        }

        if let Some(prev) = self.de.last_deserialized_type {
            if let Some(lookahead) = self.de.next_name_lookahead() {
                if prev != lookahead {
                    // the next struct is of different type from the array contents
                    // that means the array implicitly ended
                    // and we're looking at a field following the array instead
                    return Ok(None);
                }
            }
        }

        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct StructFields<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    current_field: usize,
    fields: &'static [&'static str],
}

impl<'a, 'de> StructFields<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, fields: &'static [&'static str]) -> Self {
        Self {
            de,
            current_field: 0,
            fields,
        }
    }
}

impl<'de, 'a> SeqAccess<'de> for StructFields<'a, 'de> {
    type Error = Error;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: DeserializeSeed<'de>,
    {
        if self.de.peek()? == ')' {
            if self.current_field < self.fields.len() {
                // We're short a field (or multiple),
                // but the trailing field(s) might be optional and implicitly absent.
                // In that case we prepare a hint for deserialize_option to emit None:
                self.de.next_option_empty_hint = true;
                // and we tell serde to deserialize a field that may or may not be there:
                self.current_field += 1;
                return seed.deserialize(&mut *self.de).map(Some);
                // If it was a non-optional that was missing for real,
                // then even though our bet here was wrong (and we just lied to serde)
                // the deserializer we handed off to will see the same closing paren
                // (that we reacted to just now) and still return a sensible error.
            } else {
                return Ok(None);
            }
        }

        // TODO explain this part of empty option detection
        if let Some(lookahead) = self.de.next_name_lookahead() {
            if lookahead != self.fields[self.current_field] {
                if lookahead + "s" != self.fields[self.current_field] {
                    self.de.next_option_empty_hint = true;
                } else {
                    self.de.next_option_empty_hint = false;
                }
            } else {
                self.de.next_option_empty_hint = false;
            }
        } else {
            self.de.next_option_empty_hint = false;
        }

        self.current_field += 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
}
