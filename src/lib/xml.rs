use crate::param::{ParamKind, ParamList, ParamStruct};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use std::io::{BufRead, Write, Error as ioError};
use std::str::{from_utf8, FromStr, Utf8Error};

#[derive(Debug)]
/// Types of errors encountered the XML param file
pub enum ReadError {
    /// `quick-xml` error, such as mismatched tags, non-utf8 text, broken syntax, etc
    QuickXml(quick_xml::Error),
    /// Value parsing error
    ParseError,
    /// Opening tag has an unknown name
    UnknownOpenTag(String),
    /// Close tag name doesn't match open tag name
    UnmatchedCloseTag(String),
    /// For child nodes of structs, the 'hash' attribute is not found
    MissingHash,
    /// For the first tag after XML declaration, tag must be 'struct'
    ExpectedStructTag,
    ExpectedOpenOrCloseTag(String),
    ExpectedCloseTag(String),
    ExpectedText,
}

// Bad practice to just copy event names?
// I need to have an "expected" event type as well so I can't just use Event<'a>
#[derive(Debug, Clone, Copy)]
pub enum QuickXmlEventType {
    CData,
    Comment,
    Decl,
    DocType,
    Empty,
    End,
    Eof,
    PI,
    Start,
    Text,
}

impl From<quick_xml::Error> for ReadError {
    fn from(f: quick_xml::Error) -> Self {
        Self::QuickXml(f)
    }
}

impl From<Utf8Error> for ReadError {
    fn from(f: Utf8Error) -> Self {
        Self::QuickXml(quick_xml::Error::from(f))
    }
}

impl From<ioError> for ReadError {
    fn from(f: ioError) -> Self {
        Self::QuickXml(quick_xml::Error::from(f))
    }
}

impl<'a> From<&'a Expect<'a>> for ReadError {
    fn from(f: &Expect) -> Self {
        match f {
            Expect::Struct => Self::ExpectedStructTag,
            Expect::OpenOrCloseTag(value) => {
                match from_utf8(value) {
                    Ok(s) => Self::ExpectedOpenOrCloseTag(String::from(s)),
                    Err(e) => Self::from(e)
                }
            },
            Expect::CloseTag(value) => {
                match from_utf8(value) {
                    Ok(s) => Self::ExpectedCloseTag(String::from(s)),
                    Err(e) => Self::from(e)
                }
            }
            Expect::Text => Self::ExpectedText,
        }
    }
}

/// Write a ParamStruct as XML
pub fn write_xml<W: Write>(param: &ParamStruct, writer: &mut W) -> Result<(), quick_xml::Error> {
    let mut xml_writer = Writer::new_with_indent(writer, b' ', 2);
    xml_writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"utf-8"), None)))?;
    struct_to_node(param, &mut xml_writer, None)?;
    Ok(())
}

#[derive(Debug)]
struct ParamStack<'a> {
    pub stack: Vec<ParamKind>,
    pub expect: Expect<'a>,
}

impl<'a> ParamStack<'a> {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            expect: Expect::Struct,
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            stack: Vec::with_capacity(capacity),
            expect: Expect::Struct,
        }
    }

    fn push(&mut self, node_name: &'a [u8]) -> Result<(), ReadError> {
        match self.expect {
            Expect::Struct => {
                if node_name == b"struct" {
                    self.stack.push(ParamKind::Struct(Default::default()));
                    Ok(())
                } else {
                    Err(ReadError::ExpectedStructTag)
                }
            }
            Expect::OpenOrCloseTag(_) => {
                self.stack.push(
                    match node_name {
                        b"bool" => ParamKind::Bool(Default::default()),
                        b"sbyte" => ParamKind::I8(Default::default()),
                        b"byte" => ParamKind::U8(Default::default()),
                        b"short" => ParamKind::I16(Default::default()),
                        b"ushort" => ParamKind::U16(Default::default()),
                        b"int" => ParamKind::I32(Default::default()),
                        b"uint" => ParamKind::U32(Default::default()),
                        b"float" => ParamKind::Float(Default::default()),
                        b"hash40" => ParamKind::Hash(Default::default()),
                        b"string" => ParamKind::Str(Default::default()),
                        b"list" => ParamKind::List(Default::default()),
                        b"struct" => ParamKind::Struct(Default::default()),
                        _ => return Err(ReadError::UnknownOpenTag(
                            String::from(from_utf8(node_name)?))
                        ),
                });

                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn pop(&mut self, node_name: &[u8]) -> Result<(), ReadError> {
        unimplemented!()
    }

    fn peek(&self) -> &ParamKind {
        &self.stack[self.stack.len() - 1]
    }

    fn last_mut(&mut self) -> &mut ParamKind {
        self.stack.last_mut().unwrap()
    }

    fn handle_text(&mut self, text: &[u8]) -> Result<(), ReadError> {
        if let Expect::Text = self.expect {
            let mut top = self.last_mut();
            macro_rules! convert {
                ($t:path) => {{
                    top = &mut FromStr::from_str(from_utf8(text)?)
                        .map($t)
                        .or(Err(ReadError::ParseError))?;
                        Ok(())
                };
            }}
            match top {
                ParamKind::Bool(_) => convert!(ParamKind::Bool),
                ParamKind::I8(_) => convert!(ParamKind::I8),
                ParamKind::U8(_) => convert!(ParamKind::U8),
                ParamKind::I16(_) => convert!(ParamKind::I16),
                ParamKind::U16(_) => convert!(ParamKind::U16),
                ParamKind::I32(_) => convert!(ParamKind::I32),
                ParamKind::U32(_) => convert!(ParamKind::U32),
                ParamKind::Float(_) => convert!(ParamKind::Float),
                ParamKind::Hash(_) => convert!(ParamKind::Hash),
                ParamKind::Str(_) => convert!(ParamKind::Str),
                // Note for readers
                // Expect is only set to Text after reading a value-type open tag
                // The two cases below are designed to be impossible
                ParamKind::List(_) => unreachable!(),
                ParamKind::Struct(_) => unreachable!(),
            }
        } else if text.len() == 0 {
            // empty text event being sent from quick-xml is meaningless
            Ok(())
        } else {
            Err(ReadError::from(&self.expect))
        }
    }
}

/// XML Reading state handling
#[derive(Debug, Clone)]
pub enum Expect<'a> {
    /// Should only be used at the start of the file
    Struct,
    /// After reading a list or struct, expects either the close tag
    /// Or any open tag for a new param
    OpenOrCloseTag(&'a [u8]),
    /// After parsing a text event out of a value-type param, expects this close tag.
    /// Instead of a stack of strings, this gets set when the stack is changed
    CloseTag(&'a [u8]),
    /// Used for the inside of value-type params
    Text,
}

/// Read a ParamStruct from XML
pub fn read_xml<R: BufRead>(buf_reader: &mut R) -> Result<ParamStruct, ReadError> {
    let mut reader = Reader::from_reader(buf_reader);
    reader.expand_empty_elements(true);
    let mut buf = Vec::with_capacity(0x100);
    let mut stack = Vec::<ParamKind>::with_capacity(0x100);

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(start) => {
                // match start.name() {

                // }
            }
            Event::Text(text) => {

            }
            Event::End(end) => {

            }
            Event::Eof => {

            }
            _ => unimplemented!(),
        }

        buf.clear();
    }
    //read_start(&mut xml_reader, &mut buf)
}

// METHODS FOR WRITING

fn param_to_node<W: Write>(
    param: &ParamKind,
    writer: &mut Writer<W>,
    attr: Option<(&str, &str)>,
) -> Result<(), quick_xml::Error> {
    macro_rules! write_constant {
        ($tag_name:literal, $value:expr) => {{
            let name = $tag_name;
            let mut start = BytesStart::borrowed_name(name);
            if let Some(a) = attr {
                start.push_attribute(a);
            }
            writer.write_event(Event::Start(start))?;
            // Is there any shorter way of expressing this?
            writer.write_event(Event::Text(BytesText::from_plain_str(&format!(
                "{}",
                $value
            ))))?;
            writer.write_event(Event::End(BytesEnd::borrowed(name)))?;
        }};
    };
    match param {
        ParamKind::Bool(val) => write_constant!(b"bool", val),
        ParamKind::I8(val) => write_constant!(b"sbyte", val),
        ParamKind::U8(val) => write_constant!(b"byte", val),
        ParamKind::I16(val) => write_constant!(b"short", val),
        ParamKind::U16(val) => write_constant!(b"ushort", val),
        ParamKind::I32(val) => write_constant!(b"int", val),
        ParamKind::U32(val) => write_constant!(b"uint", val),
        ParamKind::Float(val) => write_constant!(b"float", val),
        ParamKind::Hash(val) => write_constant!(b"hash40", val),
        ParamKind::Str(val) => write_constant!(b"string", val),
        ParamKind::List(val) => list_to_node(val, writer, attr)?,
        ParamKind::Struct(val) => struct_to_node(val, writer, attr)?,
    };

    Ok(())
}

fn list_to_node<W: Write>(
    param: &ParamList,
    writer: &mut Writer<W>,
    attr: Option<(&str, &str)>,
) -> Result<(), quick_xml::Error> {
    let name = b"list";
    let mut start = BytesStart::borrowed_name(name);
    if let Some(a) = attr {
        start.push_attribute(a);
    }

    if param.is_empty() {
        writer.write_event(Event::Empty(start))?;
    } else {
        writer.write_event(Event::Start(start))?;
        for (index, child) in param.iter().enumerate() {
            param_to_node(child, writer, Some(("index", &format!("{}", index))))?;
        }
        writer.write_event(Event::End(BytesEnd::borrowed(name)))?;
    }
    Ok(())
}

fn struct_to_node<W: Write>(
    param: &ParamStruct,
    writer: &mut Writer<W>,
    attr: Option<(&str, &str)>,
) -> Result<(), quick_xml::Error> {
    let name = b"struct";
    let mut start = BytesStart::borrowed_name(name);
    if let Some(a) = attr {
        start.push_attribute(a);
    }

    if param.is_empty() {
        writer.write_event(Event::Empty(start))?;
    } else {
        writer.write_event(Event::Start(start))?;
        for (hash, child) in param.iter() {
            param_to_node(child, writer, Some(("hash", &format!("{}", hash))))?;
        }
        writer.write_event(Event::End(BytesEnd::borrowed(name)))?;
    }
    Ok(())
}