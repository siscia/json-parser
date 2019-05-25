use std::cmp::PartialEq;

use arrayvec::ArrayVec;

#[derive(Debug, PartialEq, Copy, Clone)]
enum JT {
    OpenObject,
    CloseObject,
    OpenArray,
    CloseArray,
    Colon,
    Comma,
    WhiteSpace,
    JString,
    JNumber,
}

#[derive(Debug, PartialEq)]
struct JValues<'s> {
    slice: &'s str,
    jt: JT,
}

#[derive(Debug, PartialEq)]
enum ParserErrors {
    EndOfData,
    NeedMoreData,
    NeedTokenizer,
    WrongEscapeSequence,
}

struct Parser {
    scratch: std::string::String,
    state: ParserState,
    index: usize,
}

enum ParserState {
    Base,
    ZeroCopyString,
    StartEscaping,
    CopyingString,
    ReadingHex(u64, i8),
}

impl<'s, 'scratch: 's> Parser {
    fn parse(&'scratch mut self, data: &'s str) -> Result<JValues<'s>, ParserErrors> {
        match self.state {
            ParserState::Base => self.parse_base(data),
            ParserState::ZeroCopyString => self.parse_zero_copy_string(data),
            ParserState::StartEscaping { .. } => self.parse_start_escaping(data),
            ParserState::CopyingString { .. } => self.parse_copying_string(data),
            ParserState::ReadingHex { .. } => self.parse_reading_hex(data),
        }
    }
    fn parse_base(&'scratch mut self, data: &'s str) -> Result<JValues<'s>, ParserErrors> {
        for (i, c) in data[self.index..].chars().enumerate() {
            let jt = match c {
                '{' => JT::OpenObject,
                '}' => JT::CloseObject,
                '[' => JT::OpenArray,
                ']' => JT::CloseArray,
                ':' => JT::Colon,
                ',' => JT::Comma,
                '"' => {
                    self.index = self.index + i + 1;
                    self.state = ParserState::ZeroCopyString;
                    return self.parse_zero_copy_string(data);
                }
                c if c.is_whitespace() => JT::WhiteSpace,
                _ => unimplemented!(),
            };
            match jt {
                JT::WhiteSpace => {}
                _ => {
                    let begin = self.index + i;
                    self.index += i + 1;
                    return Ok(JValues {
                        slice: &data[begin..(self.index)],
                        jt,
                    });
                }
            }
        }
        Err(ParserErrors::NeedMoreData)
    }
    fn parse_zero_copy_string(
        &'scratch mut self,
        data: &'s str,
    ) -> Result<JValues<'s>, ParserErrors> {
        let begin = self.index;
        for (i, c) in data[self.index..].chars().enumerate() {
            match c {
                '"' => {
                    self.index = self.index + i + 1;
                    self.state = ParserState::Base;
                    return Ok(JValues {
                        slice: &data[begin..self.index - 1],
                        jt: JT::JString,
                    });
                }
                '\\' => {
                    dbg!(i);
                    self.scratch.truncate(0);
                    self.index = self.index + i + 1;
                    // here we remove the escape byte
                    self.scratch.push_str(&data[begin..self.index - 1]);
                    self.state = ParserState::StartEscaping;
                    return self.parse_start_escaping(data);
                }
                _ => {}
            }
        }
        self.scratch.truncate(0);
        self.scratch.push_str(&data[begin..]);
        self.state = ParserState::CopyingString;
        self.index = data.len();
        Err(ParserErrors::NeedMoreData)
    }
    fn parse_start_escaping(
        &'scratch mut self,
        data: &'s str,
    ) -> Result<JValues<'scratch>, ParserErrors> {
        if let Some(c) = data[self.index..].chars().nth(0) {
            let to_add = match c {
                '"' => '"',
                '\\' => '\\',
                '/' => '/',
                'b' => 0x08 as char,
                'f' => 0x0c as char,
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'u' => {
                    self.index += 1;
                    self.state = ParserState::ReadingHex(0, 4);
                    return self.parse_reading_hex(data);
                }
                _ => return Err(ParserErrors::WrongEscapeSequence),
            };
            self.scratch.push(to_add);
            self.index += 1;
            self.state = ParserState::CopyingString;
            return self.parse_copying_string(data);
        } else {
            Err(ParserErrors::NeedMoreData)
        }
    }
    fn parse_copying_string(
        &'scratch mut self,
        data: &'s str,
    ) -> Result<JValues<'scratch>, ParserErrors> {
        for (i, c) in data[self.index..].chars().enumerate() {
            match c {
                '"' => {
                    self.index = self.index + i + 1;
                    self.state = ParserState::Base;
                    return Ok(JValues {
                        slice: &self.scratch,
                        jt: JT::JString,
                    });
                }
                '\\' => {
                    self.index += i + 1;
                    self.state = ParserState::StartEscaping;
                    return self.parse_start_escaping(data);
                }
                c => self.scratch.push(c),
            }
        }
        self.state = ParserState::CopyingString;
        self.index = data.len();
        Err(ParserErrors::NeedMoreData)
    }
    fn parse_reading_hex(
        &'scratch mut self,
        data: &'s str,
    ) -> Result<JValues<'scratch>, ParserErrors> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {

    use crate::{Parser, ParserErrors, ParserState, JT};

    #[test]
    fn parser2_open_close_curly() {
        let mut parser = Parser {
            scratch: std::string::String::new(),
            state: ParserState::Base,
            index: 0,
        };
        let data = "{}";
        let open = parser.parse(data).unwrap();
        assert_eq!(open.jt, JT::OpenObject);
        assert_eq!(open.slice, "{");
        let close = parser.parse(data).unwrap();
        assert_eq!(close.jt, JT::CloseObject);
        assert_eq!(close.slice, "}");
        let error = parser.parse(data);
        assert!(error.is_err());
    }

    #[test]
    fn parse_simple_string() {
        let mut parser = Parser {
            scratch: std::string::String::new(),
            state: ParserState::Base,
            index: 0,
        };
        let data = "    \"foo_ _bar\"  ";
        let string = parser.parse(data).unwrap();
        assert_eq!(string.jt, JT::JString);
        assert_eq!(string.slice, "foo_ _bar");
    }

    #[test]
    fn parse_string_multiple_buffers() {
        let mut parser = Parser {
            scratch: std::string::String::new(),
            state: ParserState::Base,
            index: 0,
        };
        let data = "    \"foo";
        let string = parser.parse(data);
        assert!(string.is_err());

        parser.index = 0;
        let data = " bar\" \"ok\"";

        let string = parser.parse(data).unwrap();
        assert_eq!(string.jt, JT::JString);
        assert_eq!(string.slice, "foo bar");

        let ok = parser.parse(data).unwrap();
        assert_eq!(ok.jt, JT::JString);
        assert_eq!(ok.slice, "ok");

        let err = parser.parse(data).is_err();
        assert!(err);

        parser.index = 0;
        let data = "\"again\"";

        let again = parser.parse(data).unwrap();
        assert_eq!(again.jt, JT::JString);
        assert_eq!(again.slice, "again");

        let err = parser.parse(data).is_err();
        assert!(err);

        parser.index = 0;
        let data = "\"with\\nnewlines\\n\"";
        let new_line = parser.parse(data).unwrap();
        assert_eq!(new_line.jt, JT::JString);
        assert_eq!(new_line.slice, "with\nnewlines\n");

        let err = parser.parse(data).is_err();
        assert!(err);

        parser.index = 0;
        let data = "\"foo\\";

        let err = parser.parse(data);
        assert_eq!(ParserErrors::NeedMoreData, err.err().unwrap());

        parser.index = 0;
        let data = "nbar\"";
        let different_string_escape = parser.parse(data).unwrap();
        assert_eq!(different_string_escape.jt, JT::JString);
        assert_eq!(different_string_escape.slice, "foo\nbar");
    }

}
