extern crate arrayvec;

use std::cmp::PartialEq;

use arrayvec::{ArrayString, ArrayVec};

#[derive(Clone, Copy, PartialEq)]
enum JTypes {
    JObject,
    JArray,
    JString,
    JNumber,
    JBool,
    JNull,
    Begin,
    JKey,
}

#[derive(Clone, Copy, Debug)]
enum JError {
    UnknowError(&'static str),
    Unimplemented(),
    ObjectNotOpen(&'static str),
    ArrayNotOpen(),
}

struct JSONObject<'a> {
    jtype: JTypes,
    parser: &'a Parser,
    path: &'a str,
    value: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum JAction {
    Start,
    End,
    BeginObject,
    EndObject,
    BeginArray,
    EndArray,
    ObjectKey,
    ObjectValue,
    JString(&'static str),
}

struct AAA<'p> {
    action: JAction,
    path: &'p str,
}

struct JStep {
    Start,
    Stop,
    BeginObject,
    EndObject,
    Key,
    String,
}

struct Parser {
    text: &'static [u8],
    index: usize,
    next_text: Option<&'static str>,
    stack: ArrayVec<[JTypes; 512]>,
    path: ArrayString<[u8; 2048]>,
}

impl Parser {
    fn empty() -> Self {
        let mut path = ArrayString::<[u8; 2048]>::new();
        path.push('$');
        return Parser {
            text: &[],
            index: 0,
            next_text: None,
            stack: ArrayVec::new(),
            path,
        };
    }
    fn new(data: &'static str) -> Self {
        let mut p = Parser::empty();
        p.text = data.as_bytes();
        p
    }
    fn begin<'a>(&'a self) -> AAA<'a> {
        AAA {
            action: JAction::Start,
            path: self.path.as_str(),
        }
    }
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
    fn foo(&mut self) {
        self.path.try_push_str("[1]");
    }
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
