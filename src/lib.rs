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
enum TokenizerErrors {
    EndOfData,
    NeedMoreData,
    WrongEscapeSequence(usize),
    WrongFormat(usize),
}

struct Tokenizer {
    scratch: std::string::String,
    state: TokenizerState,
    index: usize,
}

enum TokenizerState {
    Base,
    ZeroCopyString,
    StartEscaping,
    CopyingString,
    ReadingHex(u64, i8),
}

impl<'s, 'scratch: 's> Tokenizer {
    fn tokenize(&'scratch mut self, data: &'s str) -> Result<JValues<'s>, TokenizerErrors> {
        match self.state {
            TokenizerState::Base => self.tokenize_base(data),
            TokenizerState::ZeroCopyString => self.tokenize_zero_copy_string(data),
            TokenizerState::StartEscaping { .. } => self.tokenize_start_escaping(data),
            TokenizerState::CopyingString { .. } => self.tokenize_copying_string(data),
            TokenizerState::ReadingHex { .. } => self.tokenize_reading_hex(data),
        }
    }
    fn tokenize_base(&'scratch mut self, data: &'s str) -> Result<JValues<'s>, TokenizerErrors> {
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
                    self.state = TokenizerState::ZeroCopyString;
                    return self.tokenize_zero_copy_string(data);
                }
                c if c.is_whitespace() => JT::WhiteSpace,
                _ => return Err(TokenizerErrors::WrongFormat(self.index + i)),
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
        Err(TokenizerErrors::NeedMoreData)
    }
    fn tokenize_zero_copy_string(
        &'scratch mut self,
        data: &'s str,
    ) -> Result<JValues<'s>, TokenizerErrors> {
        let begin = self.index;
        for (i, c) in data[self.index..].chars().enumerate() {
            match c {
                '"' => {
                    self.index = self.index + i + 1;
                    self.state = TokenizerState::Base;
                    return Ok(JValues {
                        slice: &data[begin..self.index - 1],
                        jt: JT::JString,
                    });
                }
                '\\' => {
                    self.scratch.truncate(0);
                    self.index = self.index + i + 1;
                    // here we remove the escape byte
                    self.scratch.push_str(&data[begin..self.index - 1]);
                    self.state = TokenizerState::StartEscaping;
                    return self.tokenize_start_escaping(data);
                }
                _ => {}
            }
        }
        self.scratch.truncate(0);
        self.scratch.push_str(&data[begin..]);
        self.state = TokenizerState::CopyingString;
        self.index = data.len();
        Err(TokenizerErrors::NeedMoreData)
    }
    fn tokenize_start_escaping(
        &'scratch mut self,
        data: &'s str,
    ) -> Result<JValues<'scratch>, TokenizerErrors> {
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
                    self.state = TokenizerState::ReadingHex(0, 4);
                    return self.tokenize_reading_hex(data);
                }
                _ => return Err(TokenizerErrors::WrongEscapeSequence(self.index)),
            };
            self.scratch.push(to_add);
            self.index += 1;
            self.state = TokenizerState::CopyingString;
            return self.tokenize_copying_string(data);
        } else {
            Err(TokenizerErrors::NeedMoreData)
        }
    }
    fn tokenize_copying_string(
        &'scratch mut self,
        data: &'s str,
    ) -> Result<JValues<'scratch>, TokenizerErrors> {
        for (i, c) in data[self.index..].chars().enumerate() {
            match c {
                '"' => {
                    self.index = self.index + i + 1;
                    self.state = TokenizerState::Base;
                    return Ok(JValues {
                        slice: &self.scratch,
                        jt: JT::JString,
                    });
                }
                '\\' => {
                    self.index += i + 1;
                    self.state = TokenizerState::StartEscaping;
                    return self.tokenize_start_escaping(data);
                }
                c => self.scratch.push(c),
            }
        }
        self.state = TokenizerState::CopyingString;
        self.index = data.len();
        Err(TokenizerErrors::NeedMoreData)
    }
    fn tokenize_reading_hex(
        &'scratch mut self,
        data: &'s str,
    ) -> Result<JValues<'scratch>, TokenizerErrors> {
        unimplemented!()
    }
}

#[derive(Debug, PartialEq)]
enum F {
    InObject,
    InObjectAfterKey,
    InArray,
    Key,
    JString,
    JNumber,
}

enum PE {
    EndOfData,
    NeedMoreData,
    WrongEscapeSequence(usize),
    WrongFormat(usize),
}

impl From<TokenizerErrors> for PE {
    fn from(error: TokenizerErrors) -> Self {
        match error {
            TokenizerErrors::EndOfData => PE::EndOfData,
            TokenizerErrors::NeedMoreData => PE::NeedMoreData,
            TokenizerErrors::WrongEscapeSequence(u) => PE::WrongEscapeSequence(u),
            TokenizerErrors::WrongFormat(u) => PE::WrongFormat(u),
        }
    }
}

struct Parser {
    stack: std::vec::Vec<F>,
    tokenizer: Tokenizer,
}
type PR<'s> = (F, Option<&'s str>);

enum PR2<'s> {
    InObject,
    InArray,
    Key(&'s str),
    JString(&'s str),
}

impl<'s, 'ss: 's> Parser {
    fn parse(&'ss mut self, data: &'s str) -> Result<PR<'s>, PE> {
        let index = self.tokenizer.index;
        let token = self.tokenizer.tokenize(data)?;
        let state = self.stack.last();
        let result = match (state, token.jt) {
            (None, JT::OpenObject) => {
                self.stack.push(F::InObject);
                (F::InObject, None)
            }
            (None, JT::OpenArray) => {
                self.stack.push(F::InArray);
                (F::InArray, None)
            }
            (None, JT::JString) => (F::JString, Some(token.slice)),
            (None, JT::JNumber) => (F::JNumber, None),
            (None, _) => return Err(PE::WrongFormat(index)),

            (Some(F::InObject), JT::JString) => {
                self.stack.push(F::Key);
                (F::JString, Some(token.slice))
            }
            /*
            (Some(F::Key), JT::Comma) => {
                self.stack.push(F::InObjectAfterKey);
                return self.parse(data);
            }
            */
            (Some(F::InObjectAfterKey), JT::JString) => {
                self.stack.pop();
                (F::JString, Some(token.slice))
            }

            (_, _) => unimplemented!(),
        };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {

    use crate::{Tokenizer, TokenizerErrors, TokenizerState, JT};

    #[test]
    fn tokenizer2_open_close_curly() {
        let mut tokenizer = Tokenizer {
            scratch: std::string::String::new(),
            state: TokenizerState::Base,
            index: 0,
        };
        let data = "{}";
        let open = tokenizer.tokenize(data).unwrap();
        assert_eq!(open.jt, JT::OpenObject);
        assert_eq!(open.slice, "{");
        let close = tokenizer.tokenize(data).unwrap();
        assert_eq!(close.jt, JT::CloseObject);
        assert_eq!(close.slice, "}");
        let error = tokenizer.tokenize(data);
        assert!(error.is_err());
    }

    #[test]
    fn tokenize_simple_string() {
        let mut tokenizer = Tokenizer {
            scratch: std::string::String::new(),
            state: TokenizerState::Base,
            index: 0,
        };
        let data = "    \"foo_ _bar\"  ";
        let string = tokenizer.tokenize(data).unwrap();
        assert_eq!(string.jt, JT::JString);
        assert_eq!(string.slice, "foo_ _bar");
    }

    #[test]
    fn tokenize_string_multiple_buffers() {
        let mut tokenizer = Tokenizer {
            scratch: std::string::String::new(),
            state: TokenizerState::Base,
            index: 0,
        };
        let data = "    \"foo";
        let string = tokenizer.tokenize(data);
        assert!(string.is_err());

        tokenizer.index = 0;
        let data = " bar\" \"ok\"";

        let string = tokenizer.tokenize(data).unwrap();
        assert_eq!(string.jt, JT::JString);
        assert_eq!(string.slice, "foo bar");

        let ok = tokenizer.tokenize(data).unwrap();
        assert_eq!(ok.jt, JT::JString);
        assert_eq!(ok.slice, "ok");

        let err = tokenizer.tokenize(data).is_err();
        assert!(err);

        tokenizer.index = 0;
        let data = "\"again\"";

        let again = tokenizer.tokenize(data).unwrap();
        assert_eq!(again.jt, JT::JString);
        assert_eq!(again.slice, "again");

        let err = tokenizer.tokenize(data).is_err();
        assert!(err);

        tokenizer.index = 0;
        let data = "\"with\\nnewlines\\n\"";
        let new_line = tokenizer.tokenize(data).unwrap();
        assert_eq!(new_line.jt, JT::JString);
        assert_eq!(new_line.slice, "with\nnewlines\n");

        let err = tokenizer.tokenize(data).is_err();
        assert!(err);

        tokenizer.index = 0;
        let data = "\"foo\\";

        let err = tokenizer.tokenize(data);
        assert_eq!(TokenizerErrors::NeedMoreData, err.err().unwrap());

        tokenizer.index = 0;
        let data = "nbar\"";
        let different_string_escape = tokenizer.tokenize(data).unwrap();
        assert_eq!(different_string_escape.jt, JT::JString);
        assert_eq!(different_string_escape.slice, "foo\nbar");
    }

}
