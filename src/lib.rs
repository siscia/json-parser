use std::cmp::PartialEq;

use arrayvec::ArrayVec;

trait OriginalDataRetriever<'s> {
    fn retrieve_original_data(&self) -> &'s str;
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum TT {
    OpenCurly,
    CloseCurly,
    OpenSquare,
    CloseSquare,
    Escape,
    Comma,
    Quote,
    Minus,
    Digit,
}

impl TT {
    fn from_char(input: char) -> Option<Self> {
        match input {
            '{' => Some(TT::OpenCurly),
            '}' => Some(TT::CloseCurly),
            '[' => Some(TT::OpenSquare),
            ']' => Some(TT::CloseSquare),
            '\\' => Some(TT::Escape),
            ',' => Some(TT::Comma),
            '"' => Some(TT::Quote),
            '-' => Some(TT::Minus),
            '0'...'9' => Some(TT::Digit),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Token<'s> {
    slice: &'s str,
    position: usize,
    tt: TT,
}

impl<'s> Token<'s> {
    fn new(slice: &'s str, position: usize, tt: TT) -> Self {
        Self {
            slice,
            position,
            tt,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Tokenizer<'s> {
    data: &'s str,
    tokenized_up_to: usize,
    tokens: ArrayVec<[Token<'s>; 256]>,
    tokens_index: usize,
}

enum TokenizerErrors {
    EndOfData,
}

impl<'a> Tokenizer<'a> {
    fn new(data: &'a str) -> Self {
        Tokenizer {
            data,
            tokenized_up_to: 0,
            tokens: ArrayVec::new(),
            tokens_index: 0,
        }
    }
}

impl<'t, 's> Tokenizer<'s> {
    fn tokenize_next_batch(&mut self) -> Result<usize, TokenizerErrors> {
        if self.data.len() - 1 <= self.tokenized_up_to {
            return Err(TokenizerErrors::EndOfData);
        }
        self.tokens.truncate(0);
        self.tokens_index = 0;
        let starting_index = self.tokenized_up_to;
        let mut tokenized_up_to = self.tokenized_up_to;
        let data = self.data;
        let new_tokens = self.data[starting_index..]
            .chars()
            .enumerate()
            .map(|(i, ss)| {
                tokenized_up_to = starting_index + i;
                (starting_index + i, TT::from_char(ss))
            })
            .filter(|(_i, tt)| tt.is_some())
            .map(|(i, tt)| Token::new(data, i, tt.unwrap()));

        self.tokens.extend(new_tokens);
        self.tokenized_up_to = tokenized_up_to + 1;
        Ok(self.tokens.len())
    }
}

impl<'s> Iterator for Tokenizer<'s> {
    type Item = Token<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.tokens_index >= self.tokens.len() {
            return match self.tokenize_next_batch() {
                Ok(_) => self.next(),
                Err(TokenizerErrors::EndOfData) => None,
            };
        }
        let next = Some(self.tokens[self.tokens_index]);
        self.tokens_index += 1;
        next
    }
}

impl<'t, 's> OriginalDataRetriever<'s> for Tokenizer<'s> {
    fn retrieve_original_data(&self) -> &'s str {
        self.data
    }
}

fn string_from_data<'s>(data: &'s str) -> &'s str {
    let t = Tokenizer::new(data);
    let mut begin = None;
    let mut end = None;
    for token in t {
        if token.tt == TT::Quote {
            if begin == None {
                begin = Some(token.position);
            } else {
                end = Some(token.position);
                break;
            }
        }
    }

    &data[begin.unwrap() + 1..=end.unwrap() - 1]
}

enum JT {
    OpenObject,
    CloseObject,
    OpenArray,
    CloseArray,
    Comma,
    JString,
    JNumber,
}

struct JValues<'s> {
    slice: &'s str,
    jt: JT,
}

enum ParserErrors {
    EndOfData,
    NeedMoreData,
    NeedTokenizer,
}

struct Parser {
    scratch: std::string::String,
    path: std::string::String,
    parsing_string: bool,
    escaping_string: bool,
    string_begin: Option<usize>,
    parsing_number: bool,
}

impl<'values, 'token: 'values> Parser {
    fn foo(&mut self, t: Tokenizer<'token>) -> Result<JValues<'values>, ParserErrors> {
        let data = t.data;
        for token in t {
            let res = if (self.parsing_string) {
                if (self.escaping_string) {
                    self.escaping_string = false;
                    continue;
                } else {
                    match token.tt {
                        TT::Quote => {
                            let begin_position = self.string_begin.unwrap();
                            self.string_begin = None;
                            self.parsing_string = false;
                            Ok(JValues {
                                slice: &data[self.string_begin.unwrap()..token.position],
                                jt: JT::JString,
                            })
                        }
                        TT::Escape => {
                            self.escaping_string = true;
                            continue;
                        }
                        _ => continue,
                    }
                }
            } else {
                match token.tt {
                    TT::OpenCurly => Ok(JValues {
                        slice: token.slice,
                        jt: JT::OpenObject,
                    }),
                    TT::CloseCurly => Ok(JValues {
                        slice: token.slice,
                        jt: JT::CloseObject,
                    }),
                    TT::OpenSquare => Ok(JValues {
                        slice: token.slice,
                        jt: JT::OpenArray,
                    }),
                    TT::CloseSquare => Ok(JValues {
                        slice: token.slice,
                        jt: JT::CloseArray,
                    }),
                    TT::Comma => Ok(JValues {
                        slice: token.slice,
                        jt: JT::Comma,
                    }),
                    TT::Quote => {
                        self.parsing_string = true;
                        self.string_begin = Some(token.position);
                        continue;
                    }
                    _ => Err(ParserErrors::NeedMoreData),
                }
            };
        }
        return Err(ParserErrors::NeedTokenizer);
    }

    /*
    fn next(&mut self) -> Result<JValues<'values>, ParserErrors> {
        for tokenizer in self.tokenizer.iter() {
            for token in **tokenizer {
                let res = match token.tt {
                    TT::OpenCurly => Ok(JValues {
                        slice: token.slice,
                        jt: JT::OpenObject,
                    }),
                    TT::CloseCurly => Ok(JValues {
                        slice: token.slice,
                        jt: JT::CloseObject,
                    }),
                    TT::OpenSquare => Ok(JValues {
                        slice: token.slice,
                        jt: JT::OpenArray,
                    }),
                    TT::CloseSquare => Ok(JValues {
                        slice: token.slice,
                        jt: JT::CloseArray,
                    }),
                    TT::Comma => Ok(JValues {
                        slice: token.slice,
                        jt: JT::Comma,
                    }),
                    _ => Err(ParserErrors::NeedMoreData),
                };
                return res;
            }
            return Err(ParserErrors::NeedTokenizer);
        }
        return Err(ParserErrors::NeedTokenizer);
    }
    */
}

#[cfg(test)]
mod tests {

    extern crate rand;
    use rand::seq::SliceRandom;

    use crate::{string_from_data, Parser, Tokenizer, TT};

    fn random_token() -> char {
        let v = vec![
            '{', '}', '[', ']', '"', '\\', ',', '"', '-', '0', '1', '2', '3', '4', '5', '6', '7',
            '8', '9',
        ];
        *v.choose(&mut rand::thread_rng()).unwrap()
    }

    #[test]
    fn open_close_curly() {
        let data = "{}";
        let mut t = Tokenizer::new(data);
        let open = t.next().unwrap();
        assert_eq!(open.position, 0);
        assert_eq!(open.tt, TT::OpenCurly);
        let close = t.next().unwrap();
        assert_eq!(close.position, 1);
        assert_eq!(close.tt, TT::CloseCurly);

        assert_eq!(t.next(), None);
    }

    #[test]
    fn very_long_symbols() {
        let mut s = String::with_capacity(1024 + 8);
        for _i in 0..(1024 + 8) {
            s.push(random_token())
        }
        let t = Tokenizer::new(&s);
        let mut i = 0;
        for token in t {
            i += 1;
            let c = token.slice.as_bytes()[token.position];
            let tt = TT::from_char(c as char);
            assert_eq!(token.tt, tt.unwrap());
        }
        assert_eq!(i, 1024 + 8);
    }

    #[test]
    fn getting_simple_string() {
        let data = "####\"simple string\"@@@@@";
        let s = string_from_data(data);
        assert_eq!("simple string", s);
    }

    /*
    #[test]
    fn testing_data_borrows() {
        let s = std::string::String::with_capacity(10);
        let p = std::string::String::with_capacity(10);
        let mut parser = Parser {
            scratch: s,
            path: p,
            i: 0,
        };
        let check = vec!["A", "AB", "ABC"];
        for i in 0..3 {
            let f = parser.next();
            assert_eq!(f, check[i]);
        }
    }
    */

}
