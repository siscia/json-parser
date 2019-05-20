extern crate arrayvec;

use std::cmp::PartialEq;
use str;

use arrayvec::{ArrayString, ArrayVec};

#[derive(Clone, Copy, Debug)]
enum JError {
    UnknowError(&'static str),
    Unimplemented(),
    ObjectNotOpen(&'static str),
    ArrayNotOpen(),
    ExpectedObject,
    ExpectedKey,
    StringNotClosed,
    FinishedString,
    NotFoundValue,
}

struct JValues {
    step: JStep,
    path: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum JStep {
    JStart,
    JStop,
    JBeginObject,
    JEndObject,
    JKey(&'static str),
    JString(&'static str),
}

struct Parser {
    text: &'static [u8],
    index: usize,
    next_text: Option<&'static str>,
    stack: ArrayVec<[JStep; 512]>,
    path: ArrayString<[u8; 2048]>,
}

impl Parser {
    fn empty() -> Self {
        let mut path = ArrayString::<[u8; 2048]>::new();
        path.push('$');
        let mut stack = ArrayVec::<[_; 512]>::new();
        stack.push(JStep::JStart);
        return Parser {
            text: &[],
            index: 0,
            next_text: None,
            stack: stack,
            path,
        };
    }
    fn new(data: &'static str) -> Self {
        let mut p = Parser::empty();
        p.text = data.as_bytes();
        p
    }
    fn peek_stack(&self) -> JStep {
        self.stack[self.stack.len() - 1]
    }
    fn next(&mut self) -> Result<JValues, JError> {
        match self.peek_stack() {
            JStep::JStart => self.match_start(),
            JStep::JBeginObject => self.match_key(),
            JStep::JKey(_) => self.match_value(),
            JStep::JValue() => 
            _ => Err(JError::Unimplemented()),
        }
    }
    fn match_start(&mut self) -> Result<JValues, JError> {
        for (i, s) in self.text.iter().enumerate() {
            let s = *s as char;
            if s.is_whitespace() {
                continue;
            }
            match s {
                '{' => {
                    let step = JStep::JBeginObject;
                    self.stack.push(step);
                    self.text = &self.text[i..self.text.len()];
                    return Ok(JValues {
                        step,
                        path: self.path.as_str().to_string(),
                    });
                }
                _ => return Err(JError::ExpectedObject),
            }
        }
        Ok(JValues {
            step: JStep::JStop,
            path: self.path.as_str().to_string(),
        })
    }
    fn match_key(&mut self) -> Result<JValues, JError> {
        for (i, j) in self.text.iter().enumerate() {
            let s = *j as char;
            if s.is_whitespace() {
                continue;
            }
            match s {
                '"' => {
                    let opening_index = i;
                    let text = &self.text[opening_index..self.text.len()];
                    match Parser::closing_string(text) {
                        Err(()) => return Err(JError::StringNotClosed),
                        Ok(closing_index) => {
                            let key = &self.text[opening_index + 1..closing_index - 1];
                            let step =
                                JStep::JKey(unsafe { std::str::from_utf8_unchecked(key.clone()) });
                            let value = Ok(JValues {
                                step,
                                path: self.path.as_str().to_string(),
                            });

                            self.path.push('.');
                            for k in key {
                                self.path.push(*k as char);
                            }
                            self.stack.push(step);
                            self.text = &self.text[closing_index..self.text.len()];
                            return value;
                        }
                    }
                }
                _ => return Err(JError::ExpectedKey),
            }
        }
        Err(JError::ExpectedKey)
    }
    fn match_value(&mut self) -> Result<JValues, JError> {
        for (i, ss) in self.text.iter().enumerate() {
            let s = *ss as char;
            if s.is_whitespace() {
                continue;
            }
            match s {
                ':' => {
                    self.text = &self.text[i..self.text.len()];
                    match Parser::closing_string(self.text) {
                        Err(()) => return Err(JError::StringNotClosed),
                        Ok(closing_index) => {
                            let opening_index = i;
                            let string = &self.text[opening_index + 1..closing_index - 1];
                            let string = unsafe { std::str::from_utf8_unchecked(string) };
                            let step = JStep::JString(string);
                            self.text = &self.text[closing_index..self.text.len()];
                            self.stack.pop();
                            return Ok(JValues {
                                step,
                                path: self.path.as_str().to_string(),
                            });
                        }
                    }
                }
                _ => return Err(JError::NotFoundValue),
            }
        }
        return Err(JError::FinishedString);
    }
    fn closing_string(text: &[u8]) -> Result<usize, ()> {
        let mut escaped = false;
        for (i, s) in text.iter().enumerate() {
            if escaped {
                escaped = false;
                continue;
            }
            match *s as char {
                '\\' => {
                    escaped = true;
                    continue;
                }
                '"' => {
                    return Ok(i);
                }
                _ => continue,
            }
        }
        Err(())
    }
    /*
        fn next<'p>(&'p mut self) -> Result<AAA<'p>, JError> {
            self.text = &self.text[self.index..self.text.len()];
            self.index = 0;
            for s in self.text {
                self.index += 1;
                match *s as char {
                    '{' => {
                        self.stack.push(JTypes::JObject);
                        return Ok(AAA {
                            action: JAction::BeginObject,
                            path: self.path.as_str(),
                        });
                    }
                    '}' => {
                        let jobject = self.stack.pop().unwrap();
                        if jobject != JTypes::JObject {
                            return Err(JError::ObjectNotOpen("Found"));
                        }
                        return Ok(AAA {
                            action: JAction::EndObject,
                            path: self.path.as_str(),
                        });
                    }
                    '[' => {
                        self.stack.push(JTypes::JArray);
                        return Ok(AAA {
                            action: JAction::BeginArray,
                            path: self.path.as_str(),
                        });
                    }
                    ']' => {
                        let jobject = self.stack.pop().unwrap();
                        if jobject != JTypes::JArray {
                            return Err(JError::ArrayNotOpen());
                        }
                        return Ok(AAA {
                            action: JAction::EndArray,
                            path: self.path.as_str(),
                        });
                    }
                    '"' => {
                        let mut escaped = false;
                        let mut end = 0;
                        for (i, ss) in self.text.iter().enumerate() {
                            if i == 0 {
                                continue;
                            }
                            if escaped {
                                escaped = false;
                                continue;
                            }
                            match *ss as char {
                                '"' => {
                                    end = i;
                                    break;
                                }
                                '\\' => {
                                    escaped = true;
                                    continue;
                                }
                                _ => {}
                            }
                        }
                        let begin = self.index;
                        if self.stack.len() == 0 {
                            let simple_str = std::str::from_utf8(&self.text[begin..end]).unwrap();
                            return Ok(AAA {
                                action: JAction::JString(simple_str),
                                path: self.path.as_str(),
                            });
                        }
                        match self.stack[self.stack.len()] {
                            JTypes::JObject => {
                                self.stack.push(JTypes::JKey);
                                self.path.push('.');
                                for adding in begin..end {
                                    self.path.push(self.text[adding] as char)
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => return Err(JError::Unimplemented()),
                }
            }
            return Err(JError::UnknowError(""));
        }
    */
}

/*
 *
 * {}
 *
 * BeginObject, path(?)
 * EndObject, path(?)
 *
 * "foo"
 * String("foo"), path(?)
 *
 * {'a' : [1,2]}
 *
 * BeginObject, path(?)
 * Key(a), path(?)
 * BeginArray, path(?.a)
 * Integer(1), path(?.a[0])
 * Integer(2), path(?.a[1])
 * EndArray, path(?.a)
 * EndObject, path(?)
 *
 *
 *
 */

#[cfg(test)]
mod tests {
    #[test]
    fn empty_object() {
        let mut p = crate::Parser::new("{}");
        let begin = p.next().unwrap();
        assert_eq!(begin.action, crate::JAction::BeginObject);
        assert_eq!(begin.path, "$");
        let end = p.next().unwrap();
        assert_eq!(end.action, crate::JAction::EndObject);
        assert_eq!(end.path, "$");
    }

    #[test]
    fn simple_string() {
        let mut p = crate::Parser::new("\"foo0\"");
        let foo = p.next().unwrap();
        assert_eq!(foo.action, crate::JAction::JString("foo0"));
        assert_eq!(foo.path, "$");
    }
}
