use std::fmt;

use serde::de::{self, DeserializeSeed, EnumAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserialize;
use thiserror::Error;

type Result<T> = std::result::Result<T, DeError>;

#[derive(Error, Debug)]
pub enum DeError {
    #[error("{0}")]
    Custom(String),
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
    #[error("expected opening parenthesis for {0}")]
    ExpectedOpeningParen(&'static str),
    #[error("expected closing parenthesis for {0}")]
    ExpectedClosingParen(&'static str),
    #[error("expected a keyword")]
    ExpectedKeyword,
    #[error("wrong keyword: expected {0}, got {1}")]
    WrongKeyword(&'static str, String),
}

impl de::Error for DeError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        DeError::Custom(msg.to_string())
    }
}

#[derive(Error, Debug)]
#[error("syntax error at {0}: {1}")]
pub struct SyntaxError(pub Context, pub DeError);

#[derive(Debug)]
pub struct Context {
    line: usize,
    column: usize,
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line {0}, column {1}", self.line, self.column)
    }
}

pub struct Deserializer<'de> {
    input: &'de str,
    context: Context,

    string_quote: Option<char>,
    space_in_quoted_tokens: bool,
    reconfig_incoming: Option<ReconfigIncoming>,

    vec_type: Option<&'static str>,
    next_option_empty_hint: bool,
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
            context: Context { line: 1, column: 0 },

            string_quote: None,
            space_in_quoted_tokens: false,
            reconfig_incoming: None,

            vec_type: None,
            next_option_empty_hint: false,
        }
    }

    fn keyword_lookahead(&self) -> Option<String> {
        let mut iter = self.input.chars();
        if let Some('(') = iter.next() {
            Some(
                iter.take_while(|c| c != &' ' && c != &'\r' && c != &'\n')
                    .collect::<String>(),
            )
        } else {
            None
        }
    }

    fn peek(&mut self) -> Result<char> {
        self.input.chars().next().ok_or(DeError::Eof)
    }

    fn next(&mut self) -> Result<char> {
        let chr = self.peek()?;
        self.input = &self.input[chr.len_utf8()..];
        if chr == '\n' {
            self.context.line += 1;
            self.context.column = 0;
        } else {
            self.context.column += 1;
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
    }

    fn parse_bool(&mut self) -> Result<bool> {
        self.skip_ws();
        match &self.parse_unquoted() {
            Ok(string) => match string.as_str() {
                "on" => Ok(true),
                "off" => Ok(false),
                _ => Err(DeError::ExpectedBool),
            },
            Err(_) => Err(DeError::ExpectedBool),
        }
    }

    fn parse_keyword(&mut self) -> Result<String> {
        self.skip_ws();
        self.parse_unquoted()
    }

    fn parse_string(&mut self) -> Result<String> {
        self.skip_ws();
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
                    return Err(DeError::ExpectedUnquoted);
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
                return Err(DeError::SpaceInQuoted);
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

pub fn from_str<'a, T>(input: &'a str) -> std::result::Result<T, SyntaxError>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(input);
    let value = T::deserialize(&mut deserializer);
    deserializer.skip_ws();
    value.map_err(|err| SyntaxError(deserializer.context, err))
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = DeError;

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
        self.skip_ws();
        let value = self.parse_bool()?;

        // If the struct deserializer set a variable saying the incoming value
        // should reconfigure a specific variable in the parser we do so and
        // clear the flag.
        if self.reconfig_incoming == Some(ReconfigIncoming::SpaceAllowed) {
            self.space_in_quoted_tokens = value;
            self.reconfig_incoming = None;
        }

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
        self.skip_ws();
        let value = self.parse_unquoted()?;

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
        self.skip_ws();
        let value = self.parse_unquoted()?;

        visitor.visit_u32(value.parse().unwrap())
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_ws();
        let value = self.parse_unquoted()?;

        visitor.visit_u64(value.parse().unwrap())
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_ws();
        let value = self.parse_unquoted()?;

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
        self.skip_ws();
        let chr = self.next()?;

        // If the struct deserializer set a variable saying the incoming value
        // should reconfigure a specific variable in the parser we do so and
        // clear the flag.
        if self.reconfig_incoming == Some(ReconfigIncoming::StringQuote) {
            self.string_quote = Some(chr);
            self.reconfig_incoming = None;
        }

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
        self.skip_ws();
        let value = self.parse_string()?;

        visitor.visit_string(value)
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

    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let elem_type = self
            .vec_type
            .expect("fields of type Vec<_> need to have names suffixed with _vec");

        visitor.visit_seq(ArrayIndices::new(self, elem_type))
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
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(StructFields::new(self, fields))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_ws();
        if self.next()? != '(' {
            return Err(DeError::ExpectedOpeningParen("an enum variant"));
        }

        let value = visitor.visit_enum(Enum::new(self))?;

        self.skip_ws();
        if self.next()? != ')' {
            return Err(DeError::ExpectedClosingParen(name));
        }

        Ok(value)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_ws();
        visitor.visit_string(
            self.parse_string()
                .map_err(|err| DeError::ExpectedKeyword)?,
        )
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = DeError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de).map(|value| (value, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = DeError;

    fn unit_variant(self) -> Result<()> {
        todo!();
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!();
    }
}

struct ArrayIndices<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    elem_type: &'static str,
}

impl<'a, 'de> ArrayIndices<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, elem_type: &'static str) -> Self {
        Self { de, elem_type }
    }
}

impl<'de, 'a> SeqAccess<'de> for ArrayIndices<'a, 'de> {
    type Error = DeError;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: DeserializeSeed<'de>,
    {
        self.de.skip_ws();
        if self.de.peek()? == ')' {
            return Ok(None);
        }

        if self.de.peek()? != '(' {
            // anonymous field
            seed.deserialize(&mut *self.de).map(Some)
        } else {
            let lookahead = self
                .de
                .keyword_lookahead()
                .ok_or(DeError::ExpectedOpeningParen(self.elem_type))?;
            if lookahead == self.elem_type {
                // cannot fail, consuming the lookahead
                self.de.next().unwrap();
                self.de.parse_keyword().unwrap();

                let value = seed.deserialize(&mut *self.de)?;

                self.de.skip_ws();
                if self.de.next()? != ')' {
                    Err(DeError::ExpectedClosingParen(self.elem_type))
                } else {
                    Ok(Some(value))
                }
            } else {
                Ok(None)
            }
        }
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
    type Error = DeError;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: DeserializeSeed<'de>,
    {
        let field_name = self.fields[self.current_field];

        self.de.skip_ws();

        let ret = if field_name.ends_with("_vec") {
            self.de.vec_type = field_name.strip_suffix("_vec");
            let value = seed.deserialize(&mut *self.de).map(Some);
            self.de.vec_type = None;

            value
        } else {
            if self.de.peek()? != '(' {
                // anonymous field, cannot be optional
                self.de.next_option_empty_hint = true;
                let value = seed.deserialize(&mut *self.de).map(Some);
                self.de.next_option_empty_hint = false;
                value
            } else {
                self.de.next()?; // consume the '('

                let parsed_keyword = self.de.parse_keyword()?;
                if parsed_keyword == field_name {
                    if field_name == "string_quote" {
                        self.de.reconfig_incoming = Some(ReconfigIncoming::StringQuote);
                    } else if field_name == "space_in_quoted_tokens" {
                        self.de.reconfig_incoming = Some(ReconfigIncoming::SpaceAllowed);
                    }

                    let value = seed.deserialize(&mut *self.de)?;

                    self.de.skip_ws();
                    if self.de.next()? != ')' {
                        Err(DeError::ExpectedClosingParen(field_name))
                    } else {
                        Ok(Some(value))
                    }
                } else {
                    Err(DeError::WrongKeyword(field_name, parsed_keyword))
                }
            }
        };

        self.current_field += 1;
        ret
    }
}
