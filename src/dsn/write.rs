use super::common::ListToken;
use std::io;

pub trait WriteDsn<W: io::Write> {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), io::Error>;
}

impl<W: io::Write> WriteDsn<W> for char {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), io::Error> {
        writer.write_leaf(self.to_string())
    }
}

impl<W: io::Write> WriteDsn<W> for String {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), io::Error> {
        let string = if self.len() == 0 {
            "\"\"".to_string()
        } else if self.contains(" ")
               || self.contains("(")
               || self.contains(")")
               || self.contains("\n")
        {
            format!("\"{}\"", self)
        } else {
            self.to_string()
        };
        writer.write_leaf(string)
    }
}

impl<W: io::Write> WriteDsn<W> for bool {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), io::Error> {
        writer.write_leaf(match self {
            true => "on".to_string(),
            false => "off".to_string(),
        })
    }
}

impl<W: io::Write> WriteDsn<W> for i32 {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), io::Error> {
        writer.write_leaf(self.to_string())
    }
}

impl<W: io::Write> WriteDsn<W> for u32 {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), io::Error> {
        writer.write_leaf(self.to_string())
    }
}

impl<W: io::Write> WriteDsn<W> for usize {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), io::Error> {
        writer.write_leaf(self.to_string())
    }
}

impl<W: io::Write> WriteDsn<W> for f32 {
    fn write_dsn(&self, writer: &mut ListWriter<W>) -> Result<(), io::Error> {
        writer.write_leaf(self.to_string())
    }
}

pub struct ListWriter<W: io::Write> {
    writable: W,
    indent_level: usize,
    multiline_level: usize,
    pub line_len: usize,
}

impl<W: io::Write> ListWriter<W> {
    pub fn new(writable: W) -> Self {
        Self {
            writable,
            indent_level: 0,
            multiline_level: 0,
            line_len: 0,
        }
    }

    pub fn write_token(&mut self, token: ListToken) -> Result<(), io::Error> {
        let len = token.len();

        match token {
            ListToken::Start { name } => {
                write!(self.writable,
                    "\n{}({}",
                    "  ".repeat(self.indent_level),
                    name
                )?;
                self.multiline_level = self.indent_level;
                self.line_len = 2 * self.indent_level + len;
                self.indent_level += 1;
                Ok(())
            }
            ListToken::Leaf { value } => {
                self.line_len += 1 + len;
                write!(self.writable, " {}", value)
            }
            ListToken::End => {
                if self.indent_level <= self.multiline_level {
                    self.indent_level -= 1;
                    self.line_len = 2 * self.indent_level + len;
                    write!(self.writable,
                        "\n{})",
                        "  ".repeat(self.indent_level)
                    )
                } else {
                    self.indent_level -= 1;
                    self.line_len += len;
                    write!(self.writable, ")")
                }
            }
        }
    }

    pub fn write_leaf(&mut self, value: String) -> Result<(), io::Error> {
        self.write_token(ListToken::Leaf { value })
    }

    pub fn write_value<T: WriteDsn<W>>(
        &mut self,
        value: &T,
    ) -> Result<(), io::Error> {
        value.write_dsn(self)
    }

    pub fn write_named<T: WriteDsn<W>>(
        &mut self,
        name: &'static str,
        value: &T,
    )
        -> Result<(), io::Error>
    {
        self.write_token(ListToken::Start { name: name.to_string() } )?;
        self.write_value(value)?;
        self.write_token(ListToken::End)?;

        Ok(())
    }

    pub fn write_optional<T: WriteDsn<W>>(
        &mut self,
        name: &'static str,
        optional: &Option<T>,
    )
        -> Result<(), io::Error>
    {
        if let Some(value) = optional {
            self.write_named(name, value)?;
        }

        Ok(())
    }

    pub fn write_array<T: WriteDsn<W>>(
        &mut self,
        array: &Vec<T>,
    )
        -> Result<(), io::Error>
    {
        for elem in array {
            self.write_value(elem)?;
        }

        Ok(())
    }

    pub fn write_named_array<T: WriteDsn<W>>(
        &mut self,
        name: &'static str,
        array: &Vec<T>,
    )
        -> Result<(), io::Error>
    {
        for elem in array {
            self.write_named(name, elem)?;
        }

        Ok(())
    }
}
