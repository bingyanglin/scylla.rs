use derive_builder::Builder;
use derive_more::{
    From,
    TryInto,
};
use scylla_parse_macros::ParseFromStr;
use std::{
    collections::HashMap,
    convert::{
        TryFrom,
        TryInto,
    },
    fmt::{
        Display,
        Formatter,
    },
    marker::PhantomData,
    str::FromStr,
};
use uuid::Uuid;

mod statements;
pub use statements::*;

mod keywords;
pub use keywords::*;

mod data_types;
pub use data_types::*;

mod regex;
pub use self::regex::*;

#[derive(Clone)]
pub struct StatementStream<'a> {
    cursor: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> StatementStream<'a> {
    pub fn new(statement: &'a str) -> Self {
        Self {
            cursor: statement.chars().peekable(),
        }
    }

    pub fn remaining(&self) -> usize {
        self.cursor.clone().count()
    }

    pub fn nremaining(&self, n: usize) -> bool {
        let mut cursor = self.cursor.clone();
        for _ in 0..n {
            if cursor.next().is_none() {
                return false;
            }
        }
        true
    }

    pub fn peek(&mut self) -> Option<char> {
        self.cursor.peek().map(|c| *c)
    }

    pub fn peekn(&mut self, n: usize) -> Option<String> {
        let mut cursor = self.cursor.clone();
        let mut res = String::new();
        for _ in 0..n {
            if let Some(next) = cursor.next() {
                res.push(next);
            } else {
                return None;
            }
        }
        Some(res)
    }

    pub fn next(&mut self) -> Option<char> {
        self.cursor.next()
    }

    pub fn nextn(&mut self, n: usize) -> Option<String> {
        if self.nremaining(n) {
            let mut res = String::new();
            for _ in 0..n {
                res.push(self.next().unwrap());
            }
            Some(res)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.next();
                continue;
            } else {
                break;
            };
        }
    }

    pub fn check<P: Peek>(&self) -> bool {
        let mut this = self.clone();
        this.skip_whitespace();
        P::peek(this)
    }

    pub fn find<P: Parse<Output = P>>(&self) -> Option<P> {
        let mut this = self.clone();
        this.skip_whitespace();
        P::parse(&mut this).ok()
    }

    pub fn find_from<P: Parse>(&self) -> Option<P::Output> {
        let mut this = self.clone();
        this.skip_whitespace();
        P::parse(&mut this).ok()
    }

    pub fn parse<P: Parse<Output = P>>(&mut self) -> anyhow::Result<P> {
        self.skip_whitespace();
        P::parse(self)
    }

    pub fn parse_from<P: Parse>(&mut self) -> anyhow::Result<P::Output> {
        self.skip_whitespace();
        P::parse(self)
    }
}

pub trait Peek {
    fn peek(s: StatementStream<'_>) -> bool;
}

pub trait Parse {
    type Output;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output>;
}

macro_rules! peek_parse_tuple {
    ($($t:ident),+) => {
        impl<$($t: Peek + Parse),+> Peek for ($($t),+,) {
            fn peek(mut s: StatementStream<'_>) -> bool {
                $(
                    if s.parse_from::<Option<$t>>().transpose().is_none() {
                        return false;
                    }
                )+
                true
            }
        }

        impl<$($t: Parse),+> Parse for ($($t),+,) {
            type Output = ($($t::Output),+,);
            fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
                Ok(($(
                    s.parse_from::<$t>()?,
                )+))
            }
        }
    };
}

peek_parse_tuple!(T0);
peek_parse_tuple!(T0, T1);
peek_parse_tuple!(T0, T1, T2);
peek_parse_tuple!(T0, T1, T2, T3);
peek_parse_tuple!(T0, T1, T2, T3, T4);
peek_parse_tuple!(T0, T1, T2, T3, T4, T5);
peek_parse_tuple!(T0, T1, T2, T3, T4, T5, T6);
peek_parse_tuple!(T0, T1, T2, T3, T4, T5, T6, T7);
peek_parse_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
peek_parse_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);

impl Parse for char {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        match s.next() {
            Some(c) => Ok(c),
            None => Err(anyhow::anyhow!("End of statement!")),
        }
    }
}

impl Peek for char {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.next().is_some()
    }
}

impl Parse for bool {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if s.parse::<Option<TRUE>>()?.is_some() {
            true
        } else if s.parse::<Option<FALSE>>()?.is_some() {
            false
        } else {
            anyhow::bail!("Expected boolean!")
        })
    }
}

impl Peek for bool {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<TRUE>() || s.check::<FALSE>()
    }
}

macro_rules! peek_parse_number {
    ($n:ident, $t:ident) => {
        impl Parse for $n {
            type Output = Self;
            fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
                s.parse_from::<$t>()?
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid {}!", std::any::type_name::<$n>()))
            }
        }

        impl Peek for $n {
            fn peek(mut s: StatementStream<'_>) -> bool {
                s.parse::<Self>().is_ok()
            }
        }
    };
}

peek_parse_number!(i8, SignedNumber);
peek_parse_number!(i16, SignedNumber);
peek_parse_number!(i32, SignedNumber);
peek_parse_number!(i64, SignedNumber);
peek_parse_number!(u8, Number);
peek_parse_number!(u16, Number);
peek_parse_number!(u32, Number);
peek_parse_number!(u64, Number);
peek_parse_number!(f32, Float);
peek_parse_number!(f64, Float);

pub struct If<Cond, Res>(PhantomData<fn(Cond, Res) -> (Cond, Res)>);
impl<Cond: Peek + Parse, Res: Parse> Parse for If<Cond, Res> {
    type Output = Option<Res::Output>;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        match s.parse_from::<Option<Cond>>()? {
            Some(_) => Ok(Some(s.parse_from::<Res>()?)),
            None => Ok(None),
        }
    }
}

impl<T: Peek> Peek for Option<T> {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<T>()
    }
}

impl<T: Parse + Peek> Parse for Option<T> {
    type Output = Option<T::Output>;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if s.check::<T>() {
            Some(s.parse_from::<T>()?)
        } else {
            None
        })
    }
}

pub struct List<T, Delim>(PhantomData<fn(T, Delim) -> (T, Delim)>);
impl<T: Parse, Delim: Parse + Peek> Parse for List<T, Delim> {
    type Output = Vec<T::Output>;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let mut res = vec![s.parse_from::<T>()?];
        while s.parse_from::<Option<Delim>>()?.is_some() {
            res.push(s.parse_from::<T>()?);
        }
        Ok(res)
    }
}
impl<T: Parse, Delim: Parse + Peek> Peek for List<T, Delim> {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse_from::<Self>().is_ok()
    }
}

pub struct Nothing;
impl Parse for Nothing {
    type Output = Self;
    fn parse(_: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(Nothing)
    }
}
impl Peek for Nothing {
    fn peek(_: StatementStream<'_>) -> bool {
        true
    }
}

pub struct Whitespace;
impl Parse for Whitespace {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        while let Some(c) = s.peek() {
            if c.is_whitespace() {
                s.next();
            } else {
                break;
            }
        }
        Ok(Whitespace)
    }
}
impl Peek for Whitespace {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.peek().map(|c| c.is_whitespace()).unwrap_or(false)
    }
}

pub struct Token;
impl Parse for Token {
    type Output = String;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let mut res = String::new();
        while let Some(c) = s.next() {
            if c.is_whitespace() {
                break;
            } else {
                res.push(c);
            }
        }
        if res.is_empty() {
            anyhow::bail!("End of statement!")
        }
        Ok(res)
    }
}
impl Peek for Token {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.peek().is_some()
    }
}

pub struct Alpha;
impl Parse for Alpha {
    type Output = String;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let mut res = String::new();
        while let Some(c) = s.peek() {
            if c.is_alphabetic() {
                res.push(c);
                s.next();
            } else {
                break;
            }
        }
        if res.is_empty() {
            anyhow::bail!("End of statement!")
        }
        Ok(res)
    }
}
impl Peek for Alpha {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse_from::<Alpha>().is_ok()
    }
}

pub struct Hex;
impl Parse for Hex {
    type Output = Vec<u8>;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let mut res = String::new();
        while let Some(c) = s.peek() {
            if c.is_alphanumeric() {
                res.push(c);
                s.next();
            } else {
                break;
            }
        }
        if res.is_empty() {
            anyhow::bail!("End of statement!")
        }
        Ok(hex::decode(res)?)
    }
}
impl Peek for Hex {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse_from::<Hex>().is_ok()
    }
}

pub struct Alphanumeric;
impl Parse for Alphanumeric {
    type Output = String;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let mut res = String::new();
        while let Some(c) = s.peek() {
            if c.is_alphanumeric() {
                res.push(c);
                s.next();
            } else {
                break;
            }
        }
        if res.is_empty() {
            anyhow::bail!("End of statement!")
        }
        Ok(res)
    }
}
impl Peek for Alphanumeric {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse_from::<Alphanumeric>().is_ok()
    }
}

pub struct Number;
impl Parse for Number {
    type Output = String;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let mut res = String::new();
        while let Some(c) = s.peek() {
            if c.is_numeric() {
                res.push(c);
                s.next();
            } else {
                break;
            }
        }
        if res.is_empty() {
            anyhow::bail!("End of statement!")
        }
        Ok(res)
    }
}
impl Peek for Number {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse_from::<Number>().is_ok()
    }
}

pub struct SignedNumber;
impl Parse for SignedNumber {
    type Output = String;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let mut res = String::new();
        let mut has_negative = false;
        while let Some(c) = s.peek() {
            if c.is_numeric() {
                res.push(c);
                s.next();
            } else if c == '-' {
                if has_negative || !res.is_empty() {
                    anyhow::bail!("Invalid number: Improper negative sign")
                } else {
                    has_negative = true;
                    res.push(c);
                    s.next();
                }
            } else {
                break;
            }
        }
        if res.is_empty() {
            anyhow::bail!("End of statement!")
        }
        Ok(res)
    }
}
impl Peek for SignedNumber {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse_from::<SignedNumber>().is_ok()
    }
}

pub struct Float;
impl Parse for Float {
    type Output = String;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let mut res = String::new();
        let mut has_dot = false;
        let mut has_negative = false;
        let mut has_e = false;
        while let Some(c) = s.peek() {
            if c.is_numeric() {
                res.push(c);
                s.next();
            } else if c == '-' {
                if has_negative || !res.is_empty() {
                    anyhow::bail!("Invalid float: Improper negative sign")
                } else {
                    has_negative = true;
                    res.push(c);
                    s.next();
                }
            } else if c == '.' {
                if has_dot {
                    anyhow::bail!("Invalid float: Too many decimal points")
                } else {
                    has_dot = true;
                    res.push(c);
                    s.next();
                }
            } else if c == 'e' || c == 'E' {
                if has_e {
                    anyhow::bail!("Invalid float: Too many scientific notations")
                } else {
                    if res.is_empty() {
                        anyhow::bail!("Invalid float: Missing number before scientific notation")
                    }
                    res.push(c);
                    s.next();
                    has_e = true;
                    if let Some(next) = s.next() {
                        if next == '-' || next == '+' || next.is_numeric() {
                            res.push(next);
                        } else {
                            anyhow::bail!("Invalid float: Invalid scientific notation")
                        }
                    } else {
                        anyhow::bail!("Invalid float: Missing scientific notation value")
                    }
                }
            } else {
                break;
            }
        }
        if !has_dot {
            anyhow::bail!("Invalid float: Missing decimal point")
        }
        if res.is_empty() {
            anyhow::bail!("End of statement!")
        }
        Ok(res)
    }
}
impl Peek for Float {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse_from::<Float>().is_ok()
    }
}

macro_rules! parse_peek_group {
    ($g:ident, $l:ident, $r:ident) => {
        pub struct $g<T>(T);
        impl<T: Parse> Parse for $g<T> {
            type Output = T::Output;
            fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
                s.parse_from::<$l>()?;
                let res = s.parse_from::<T>()?;
                s.parse_from::<$r>()?;
                Ok(res)
            }
        }
        impl<T> Peek for $g<T> {
            fn peek(s: StatementStream<'_>) -> bool {
                s.check::<$l>()
            }
        }
    };
}

parse_peek_group!(Parens, LeftParen, RightParen);
parse_peek_group!(Brackets, LeftBracket, RightBracket);
parse_peek_group!(Braces, LeftBrace, RightBrace);
parse_peek_group!(Angles, LeftAngle, RightAngle);
parse_peek_group!(SingleQuoted, SingleQuote, SingleQuote);
parse_peek_group!(DoubleQuoted, DoubleQuote, DoubleQuote);

#[derive(ParseFromStr, Clone, Debug, TryInto, From)]
pub enum BindMarker {
    #[from(ignore)]
    #[try_into(ignore)]
    Anonymous,
    Named(Name),
}

impl Parse for BindMarker {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if s.parse::<Option<Question>>()?.is_some() {
            BindMarker::Anonymous
        } else {
            let (_, id) = s.parse::<(Colon, Name)>()?;
            BindMarker::Named(id)
        })
    }
}

impl Peek for BindMarker {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<Question>() || s.check::<(Colon, Name)>()
    }
}

impl Display for BindMarker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BindMarker::Anonymous => write!(f, "?"),
            BindMarker::Named(id) => write!(f, ":{}", id),
        }
    }
}

impl Parse for Uuid {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        if let Some(u) = s.nextn(36) {
            Ok(Uuid::parse_str(&u)?)
        } else {
            anyhow::bail!("Invalid UUID: {}", s.parse_from::<Token>()?)
        }
    }
}
impl Peek for Uuid {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse::<Self>().is_ok()
    }
}

#[derive(ParseFromStr, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Identifier {
    Name(Name),
    Keyword(ReservedKeyword),
}

impl Parse for Identifier {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        if let Some(keyword) = s.parse::<Option<ReservedKeyword>>()? {
            Ok(Identifier::Keyword(keyword))
        } else {
            Ok(Identifier::Name(s.parse::<Name>()?))
        }
    }
}

impl Peek for Identifier {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<ReservedKeyword>() || s.check::<Name>()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LitStrKind {
    Quoted,
    Escaped,
}

#[derive(Clone, Debug)]
pub struct LitStr {
    pub kind: LitStrKind,
    pub value: String,
}

impl Parse for LitStr {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let mut res = String::new();
        let mut kind = LitStrKind::Quoted;
        if s.peek() == Some('\'') {
            s.next();
        } else if s.peekn(2).map(|s| s.as_str() == "$$").unwrap_or(false) {
            kind = LitStrKind::Escaped;
            s.nextn(2);
        } else {
            return Err(anyhow::anyhow!("Expected opening quote!"));
        }
        while let Some(c) = s.next() {
            if kind == LitStrKind::Escaped && c == '$' && s.peek().map(|c| c == '$').unwrap_or(false) {
                s.next();
                return Ok(LitStr { kind, value: res });
            } else if kind == LitStrKind::Quoted && c == '\'' {
                return Ok(LitStr { kind, value: res });
            } else {
                res.push(c);
            }
        }
        anyhow::bail!("End of statement!")
    }
}
impl Peek for LitStr {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse::<Self>().is_ok()
    }
}

impl Display for LitStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            LitStrKind::Quoted => write!(f, "'{}'", self.value),
            LitStrKind::Escaped => write!(f, "$${}$$", self.value),
        }
    }
}

impl From<String> for LitStr {
    fn from(s: String) -> Self {
        if s.contains('\'') {
            LitStr {
                kind: LitStrKind::Escaped,
                value: s,
            }
        } else {
            LitStr {
                kind: LitStrKind::Quoted,
                value: s,
            }
        }
    }
}

impl From<&str> for LitStr {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

#[derive(ParseFromStr, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Name {
    Quoted(String),
    Unquoted(String),
}

impl Parse for Name {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        let mut res = String::new();
        if s.peek().map(|c| c == '"').unwrap_or(false) {
            while let Some(c) = s.next() {
                if c == '"' {
                    return Ok(Self::Quoted(res));
                } else {
                    res.push(c);
                }
            }
            anyhow::bail!("End of statement!")
        } else {
            while let Some(c) = s.peek() {
                if c.is_alphanumeric() || c == '_' {
                    s.next();
                    res.push(c);
                } else {
                    break;
                }
            }
            if res.is_empty() {
                anyhow::bail!("End of statement!")
            } else if ReservedKeyword::from_str(&res).is_ok() {
                anyhow::bail!("Invalid name: {} is a reserved keyword", res)
            }
            return Ok(Self::Unquoted(res));
        }
    }
}

impl Peek for Name {
    fn peek(mut s: StatementStream<'_>) -> bool {
        s.parse::<Self>().is_ok()
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quoted(s) => write!(f, "\"{}\"", s),
            Self::Unquoted(s) => s.fmt(f),
        }
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Self::Quoted(s)
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self::Quoted(s.to_string())
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub struct KeyspaceQualifiedName {
    pub keyspace: Option<Name>,
    pub name: Name,
}

impl Parse for KeyspaceQualifiedName {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        let (keyspace, name) = s.parse::<(Option<(Name, Dot)>, Name)>()?;
        Ok(KeyspaceQualifiedName {
            keyspace: keyspace.map(|(i, _)| i),
            name,
        })
    }
}

impl Peek for KeyspaceQualifiedName {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<(Option<(Name, Dot)>, Name)>()
    }
}

impl Display for KeyspaceQualifiedName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(keyspace) = &self.keyspace {
            write!(f, "{}.{}", keyspace, self.name)?;
        } else {
            self.name.fmt(f)?;
        }
        Ok(())
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub struct StatementOpt {
    pub name: Name,
    pub value: StatementOptValue,
}

impl Parse for StatementOpt {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        let (name, _, value) = s.parse::<(Name, Equals, StatementOptValue)>()?;
        Ok(StatementOpt { name, value })
    }
}

impl Display for StatementOpt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.name, self.value)
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum StatementOptValue {
    Identifier(Name),
    Constant(Constant),
    Map(MapLiteral),
}

impl Parse for StatementOptValue {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        if let Some(map) = s.parse::<Option<MapLiteral>>()? {
            Ok(StatementOptValue::Map(map))
        } else if let Some(constant) = s.parse::<Option<Constant>>()? {
            Ok(StatementOptValue::Constant(constant))
        } else if let Some(identifier) = s.parse::<Option<Name>>()? {
            Ok(StatementOptValue::Identifier(identifier))
        } else {
            anyhow::bail!("Invalid statement option value: {}", s.parse_from::<Token>()?)
        }
    }
}

impl Display for StatementOptValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identifier(identifier) => identifier.fmt(f),
            Self::Constant(constant) => constant.fmt(f),
            Self::Map(map) => map.fmt(f),
        }
    }
}

#[derive(Builder, Clone, Debug)]
pub struct ColumnDefinition {
    #[builder(setter(into))]
    pub name: Name,
    pub data_type: CqlType,
    #[builder(default)]
    pub static_column: bool,
    #[builder(default)]
    pub primary_key: bool,
}

impl ColumnDefinition {
    pub fn build() -> ColumnDefinitionBuilder {
        ColumnDefinitionBuilder::default()
    }
}

impl Parse for ColumnDefinition {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        Ok(Self {
            name: s.parse()?,
            data_type: s.parse()?,
            static_column: s.parse::<Option<STATIC>>()?.is_some(),
            primary_key: s.parse::<Option<(PRIMARY, KEY)>>()?.is_some(),
        })
    }
}

impl Peek for ColumnDefinition {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<(Name, CqlType)>()
    }
}

impl Display for ColumnDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.data_type)?;
        if self.static_column {
            write!(f, " STATIC")?;
        }
        if self.primary_key {
            write!(f, " PRIMARY KEY")?;
        }
        Ok(())
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub struct PrimaryKey {
    pub partition_key: PartitionKey,
    pub clustering_columns: Option<Vec<Name>>,
}

impl Parse for PrimaryKey {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        let (partition_key, clustering_columns) =
            s.parse_from::<(PartitionKey, Option<(Comma, List<Name, Comma>)>)>()?;
        Ok(PrimaryKey {
            partition_key,
            clustering_columns: clustering_columns.map(|i| i.1),
        })
    }
}

impl Display for PrimaryKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.partition_key.fmt(f)?;
        if let Some(clustering_columns) = &self.clustering_columns {
            write!(
                f,
                ", {}",
                clustering_columns
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }
        Ok(())
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub struct PartitionKey {
    pub columns: Vec<Name>,
}

impl Parse for PartitionKey {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        Ok(
            if let Some(columns) = s.parse_from::<Option<Parens<List<Name, Comma>>>>()? {
                Self { columns }
            } else {
                Self {
                    columns: vec![s.parse::<Name>()?],
                }
            },
        )
    }
}

impl Display for PartitionKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.columns.len() {
            0 => {
                panic!("No partition key columns specified!");
            }
            1 => self.columns[0].fmt(f),
            _ => write!(
                f,
                "({})",
                self.columns
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

// TODO: Scylla encryption opts and caching?
#[derive(Builder, Clone, Debug, Default)]
#[builder(setter(strip_option), default)]
pub struct TableOpts {
    pub compact_storage: bool,
    pub clustering_order: Option<Vec<ColumnOrder>>,
    #[builder(setter(into))]
    pub comment: Option<LitStr>,
    pub speculative_retry: Option<SpeculativeRetry>,
    pub change_data_capture: Option<bool>,
    pub gc_grace_seconds: Option<i32>,
    pub bloom_filter_fp_chance: Option<f32>,
    pub default_time_to_live: Option<i32>,
    pub compaction: Option<Compaction>,
    pub compression: Option<Compression>,
    pub caching: Option<Caching>,
    pub memtable_flush_period_in_ms: Option<i32>,
    pub read_repair: Option<bool>,
}

impl Parse for TableOpts {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        let mut res = TableOptsBuilder::default();
        loop {
            if s.parse::<Option<(COMPACT, STORAGE)>>()?.is_some() {
                if res.compact_storage.is_some() {
                    anyhow::bail!("Duplicate compact storage option");
                }
                res.compact_storage(true);
                if s.parse::<Option<AND>>()?.is_none() {
                    break;
                }
            } else if s.parse::<Option<(CLUSTERING, ORDER, BY)>>()?.is_some() {
                if res.clustering_order.is_some() {
                    anyhow::bail!("Duplicate clustering order option");
                }
                res.clustering_order(s.parse_from::<Parens<List<ColumnOrder, Comma>>>()?);
                if s.parse::<Option<AND>>()?.is_none() {
                    break;
                }
            } else {
                if let Some(v) = s.parse_from::<Option<List<StatementOpt, AND>>>()? {
                    for StatementOpt { name, value } in v {
                        let (Name::Quoted(n) | Name::Unquoted(n)) = &name;
                        match n.as_str() {
                            "comment" => {
                                if res.comment.is_some() {
                                    anyhow::bail!("Duplicate comment option");
                                } else if let StatementOptValue::Constant(Constant::String(s)) = value {
                                    res.comment(s);
                                } else {
                                    anyhow::bail!("Invalid comment value: {}", value);
                                }
                            }
                            "speculative_retry" => {
                                if res.speculative_retry.is_some() {
                                    anyhow::bail!("Duplicate speculative retry option");
                                } else if let StatementOptValue::Constant(Constant::String(s)) = value {
                                    res.speculative_retry(s.to_string().parse()?);
                                } else {
                                    anyhow::bail!("Invalid speculative retry value: {}", value);
                                }
                            }
                            "cdc" => {
                                if res.change_data_capture.is_some() {
                                    anyhow::bail!("Duplicate change data capture option");
                                } else if let StatementOptValue::Constant(Constant::Boolean(b)) = value {
                                    res.change_data_capture(b);
                                } else {
                                    anyhow::bail!("Invalid change data capture value: {}", value);
                                }
                            }
                            "gc_grace_seconds" => {
                                if res.gc_grace_seconds.is_some() {
                                    anyhow::bail!("Duplicate gc_grace_seconds option");
                                } else if let StatementOptValue::Constant(Constant::Integer(i)) = value {
                                    res.gc_grace_seconds(i.parse()?);
                                } else {
                                    anyhow::bail!("Invalid gc_grace_seconds value: {}", value);
                                }
                            }
                            "bloom_filter_fp_chance" => {
                                if res.bloom_filter_fp_chance.is_some() {
                                    anyhow::bail!("Duplicate bloom_filter_fp_chance option");
                                } else if let StatementOptValue::Constant(Constant::Float(f)) = value {
                                    res.bloom_filter_fp_chance(f.parse()?);
                                } else {
                                    anyhow::bail!("Invalid bloom_filter_fp_chance value: {}", value);
                                }
                            }
                            "default_time_to_live" => {
                                if res.default_time_to_live.is_some() {
                                    anyhow::bail!("Duplicate default_time_to_live option");
                                } else if let StatementOptValue::Constant(Constant::Integer(i)) = value {
                                    res.default_time_to_live(i.parse()?);
                                } else {
                                    anyhow::bail!("Invalid default_time_to_live value: {}", value);
                                }
                            }
                            "compaction" => {
                                if res.compaction.is_some() {
                                    anyhow::bail!("Duplicate compaction option");
                                } else if let StatementOptValue::Map(m) = value {
                                    res.compaction(m.try_into()?);
                                } else {
                                    anyhow::bail!("Invalid compaction value: {}", value);
                                }
                            }
                            "compression" => {
                                if res.compression.is_some() {
                                    anyhow::bail!("Duplicate compression option");
                                } else if let StatementOptValue::Map(m) = value {
                                    res.compression(m.try_into()?);
                                } else {
                                    anyhow::bail!("Invalid compression value: {}", value);
                                }
                            }
                            "caching" => {
                                if res.caching.is_some() {
                                    anyhow::bail!("Duplicate caching option");
                                } else if let StatementOptValue::Map(m) = value {
                                    res.caching(m.try_into()?);
                                } else {
                                    anyhow::bail!("Invalid caching value: {}", value);
                                }
                            }
                            "memtable_flush_period_in_ms" => {
                                if res.memtable_flush_period_in_ms.is_some() {
                                    anyhow::bail!("Duplicate memtable_flush_period_in_ms option");
                                } else if let StatementOptValue::Constant(Constant::Integer(i)) = value {
                                    res.memtable_flush_period_in_ms(i.parse()?);
                                } else {
                                    anyhow::bail!("Invalid memtable_flush_period_in_ms value: {}", value);
                                }
                            }
                            "read_repair" => {
                                if res.read_repair.is_some() {
                                    anyhow::bail!("Duplicate read_repair option");
                                } else if let StatementOptValue::Constant(Constant::String(s)) = value {
                                    res.read_repair(match s.value.to_uppercase().as_str() {
                                        "BLOCKING" => true,
                                        "NONE" => false,
                                        _ => anyhow::bail!("Invalid read_repair value: {}", s),
                                    });
                                } else {
                                    anyhow::bail!("Invalid read_repair value: {}", value);
                                }
                            }
                            _ => anyhow::bail!("Invalid table option: {}", name),
                        }
                    }
                }
                break;
            }
        }
        Ok(res
            .build()
            .map_err(|e| anyhow::anyhow!("Invalid Table Options: {}", e))?)
    }
}

impl Display for TableOpts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut res = Vec::new();
        if self.compact_storage {
            res.push("COMPACT STORAGE".to_string());
        }
        if let Some(ref c) = self.clustering_order {
            res.push(format!(
                "COMPACT STORAGE AND {}",
                c.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
            ));
        }
        if let Some(ref c) = self.comment {
            res.push(format!("comment = {}", c));
        }
        if let Some(ref c) = self.speculative_retry {
            res.push(format!("speculative_retry = {}", c));
        }
        if let Some(ref c) = self.change_data_capture {
            res.push(format!("cdc = {}", c));
        }
        if let Some(ref c) = self.gc_grace_seconds {
            res.push(format!("gc_grace_seconds = {}", c));
        }
        if let Some(ref c) = self.bloom_filter_fp_chance {
            res.push(format!("bloom_filter_fp_chance = {}", c));
        }
        if let Some(ref c) = self.default_time_to_live {
            res.push(format!("default_time_to_live = {}", c));
        }
        if let Some(ref c) = self.compaction {
            res.push(format!("compaction = {}", c));
        }
        if let Some(ref c) = self.compression {
            res.push(format!("compression = {}", c));
        }
        if let Some(ref c) = self.caching {
            res.push(format!("caching = {}", c));
        }
        if let Some(ref c) = self.memtable_flush_period_in_ms {
            res.push(format!("memtable_flush_period_in_ms = {}", c));
        }
        if let Some(ref c) = self.read_repair {
            res.push(format!("read_repair = {}", c));
        }
        write!(f, "{}", res.join(" AND "))
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub struct ColumnOrder {
    pub column: Name,
    pub order: Order,
}

impl Parse for ColumnOrder {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let (column, order) = s.parse::<(Name, Order)>()?;
        Ok(ColumnOrder { column, order })
    }
}

impl Display for ColumnOrder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.column, self.order)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Order {
    Ascending,
    Descending,
}

impl Parse for Order {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        if s.parse::<Option<ASC>>()?.is_some() {
            Ok(Order::Ascending)
        } else if s.parse::<Option<DESC>>()?.is_some() {
            Ok(Order::Descending)
        } else {
            anyhow::bail!("Invalid sort order: {}", s.parse_from::<Token>()?)
        }
    }
}

impl Default for Order {
    fn default() -> Self {
        Self::Ascending
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ascending => write!(f, "ASC"),
            Self::Descending => write!(f, "DESC"),
        }
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum Relation {
    Normal {
        column: Name,
        operator: Operator,
        term: Term,
    },
    Tuple {
        columns: Vec<Name>,
        operator: Operator,
        tuple_literal: TupleLiteral,
    },
    Token {
        columns: Vec<Name>,
        operator: Operator,
        term: Term,
    },
}

impl Parse for Relation {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        Ok(if s.parse::<Option<TOKEN>>()?.is_some() {
            let (columns, operator, term) = s.parse_from::<(Parens<List<Name, Comma>>, Operator, Term)>()?;
            Relation::Token {
                columns,
                operator,
                term,
            }
        } else if s.check::<LeftParen>() {
            let (columns, operator, tuple_literal) =
                s.parse_from::<(Parens<List<Name, Comma>>, Operator, TupleLiteral)>()?;
            Relation::Tuple {
                columns,
                operator,
                tuple_literal,
            }
        } else {
            let (column, operator, term) = s.parse()?;
            Relation::Normal { column, operator, term }
        })
    }
}

impl Display for Relation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Relation::Normal { column, operator, term } => write!(f, "{} {} {}", column, operator, term),
            Relation::Tuple {
                columns,
                operator,
                tuple_literal,
            } => write!(
                f,
                "({}) {} {}",
                columns.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "),
                operator,
                tuple_literal
            ),
            Relation::Token {
                columns,
                operator,
                term,
            } => write!(
                f,
                "TOKEN ({}) {} {}",
                columns.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "),
                operator,
                term
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Replication {
    SimpleStrategy(i32),
    NetworkTopologyStrategy(HashMap<String, i32>),
}

impl Replication {
    pub fn simple(replication_factor: i32) -> Self {
        Replication::SimpleStrategy(replication_factor)
    }

    pub fn network_topology(replication_map: HashMap<String, i32>) -> Self {
        Replication::NetworkTopologyStrategy(replication_map)
    }
}

impl Display for Replication {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Replication::SimpleStrategy(i) => write!(f, "{{'class': 'SimpleStrategy', 'replication_factor': {}}}", i),
            Replication::NetworkTopologyStrategy(i) => write!(
                f,
                "{{'class': 'NetworkTopologyStrategy', {}}}",
                i.iter()
                    .map(|(k, v)| format!("'{}': {}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl TryFrom<MapLiteral> for Replication {
    type Error = anyhow::Error;

    fn try_from(value: MapLiteral) -> Result<Self, Self::Error> {
        let mut class = None;
        let v = value
            .elements
            .into_iter()
            .filter(|(k, v)| {
                if let Term::Constant(Constant::String(s)) = k {
                    if s.value.to_lowercase().as_str() == "class" {
                        class = Some(v.clone());
                        return false;
                    }
                }
                true
            })
            .collect::<Vec<_>>();
        let class = class.ok_or_else(|| anyhow::anyhow!("No class in replication map literal!"))?;
        match class {
            Term::Constant(Constant::String(s)) => {
                if s.value.ends_with("SimpleStrategy") {
                    if v.len() > 1 {
                        anyhow::bail!(
                            "SimpleStrategy map literal should only contain a single 'replication_factor' key!"
                        )
                    } else if v.is_empty() {
                        anyhow::bail!("SimpleStrategy map literal should contain a 'replication_factor' key!")
                    }
                    let (k, v) = &v[0];
                    if let Term::Constant(Constant::String(s)) = k {
                        if s.value.to_lowercase().as_str() == "replication_factor" {
                            if let Term::Constant(Constant::Integer(i)) = v {
                                return Ok(Replication::SimpleStrategy(i.parse()?));
                            } else {
                                anyhow::bail!("Invalid replication factor value: {}", v)
                            }
                        } else {
                            anyhow::bail!("SimpleStrategy map literal should only contain a 'class' and 'replication_factor' key!")
                        }
                    } else {
                        anyhow::bail!("Invalid key: {}", k)
                    }
                } else if s.value.ends_with("NetworkTopologyStrategy") {
                    let mut map = HashMap::new();
                    for (k, v) in v {
                        if let Term::Constant(Constant::String(s)) = k {
                            if let Term::Constant(Constant::Integer(i)) = v {
                                map.insert(s.value, i.parse()?);
                            } else {
                                anyhow::bail!("Invalid replication factor value: {}", v)
                            }
                        } else {
                            anyhow::bail!("Invalid key in replication map literal!");
                        }
                    }
                    return Ok(Replication::NetworkTopologyStrategy(map));
                } else {
                    return Err(anyhow::anyhow!("Unknown replication class: {}", s));
                }
            }
            _ => anyhow::bail!("Invalid class: {}", class),
        }
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum SpeculativeRetry {
    None,
    Always,
    Percentile(f32),
    Custom(LitStr),
}

impl Default for SpeculativeRetry {
    fn default() -> Self {
        SpeculativeRetry::Percentile(99.0)
    }
}

impl Parse for SpeculativeRetry {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let token = s.parse::<LitStr>()?;
        Ok(
            if let Ok(res) = StatementStream::new(&token.value).parse_from::<(Float, PERCENTILE)>() {
                SpeculativeRetry::Percentile(res.0.parse()?)
            } else if let Ok(res) = StatementStream::new(&token.value).parse_from::<(Number, PERCENTILE)>() {
                SpeculativeRetry::Percentile(res.0.parse()?)
            } else {
                match token.value.to_uppercase().as_str() {
                    "NONE" => SpeculativeRetry::None,
                    "ALWAYS" => SpeculativeRetry::Always,
                    _ => SpeculativeRetry::Custom(token),
                }
            },
        )
    }
}

impl Display for SpeculativeRetry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeculativeRetry::None => write!(f, "'NONE'"),
            SpeculativeRetry::Always => write!(f, "'ALWAYS'"),
            SpeculativeRetry::Percentile(p) => write!(f, "'{:.1}PERCENTILE'", p),
            SpeculativeRetry::Custom(s) => s.fmt(f),
        }
    }
}

#[derive(Builder, Copy, Clone, Debug, Default)]
#[builder(setter(strip_option), default)]
pub struct SizeTieredCompactionStrategy {
    enabled: Option<bool>,
    tombstone_threshhold: Option<f32>,
    tombsone_compaction_interval: Option<i32>,
    log_all: Option<bool>,
    unchecked_tombstone_compaction: Option<bool>,
    only_purge_repaired_tombstone: Option<bool>,
    min_threshold: Option<i32>,
    max_threshold: Option<i32>,
    min_sstable_size: Option<i32>,
    bucket_low: Option<f32>,
    bucket_high: Option<f32>,
}

impl CompactionType for SizeTieredCompactionStrategy {}

impl Display for SizeTieredCompactionStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut res = vec![format!("'class': 'SizeTieredCompactionStrategy'")];
        if let Some(enabled) = self.enabled {
            res.push(format!("'enabled': {}", enabled));
        }
        if let Some(tombstone_threshhold) = self.tombstone_threshhold {
            res.push(format!("'tombstone_threshhold': {:.1}", tombstone_threshhold));
        }
        if let Some(tombsone_compaction_interval) = self.tombsone_compaction_interval {
            res.push(format!(
                "'tombsone_compaction_interval': {}",
                tombsone_compaction_interval
            ));
        }
        if let Some(log_all) = self.log_all {
            res.push(format!("'log_all': {}", log_all));
        }
        if let Some(unchecked_tombstone_compaction) = self.unchecked_tombstone_compaction {
            res.push(format!(
                "'unchecked_tombstone_compaction': {}",
                unchecked_tombstone_compaction
            ));
        }
        if let Some(only_purge_repaired_tombstone) = self.only_purge_repaired_tombstone {
            res.push(format!(
                "'only_purge_repaired_tombstone': {}",
                only_purge_repaired_tombstone
            ));
        }
        if let Some(min_threshold) = self.min_threshold {
            res.push(format!("'min_threshold': {}", min_threshold));
        }
        if let Some(max_threshold) = self.max_threshold {
            res.push(format!("'max_threshold': {}", max_threshold));
        }
        if let Some(min_sstable_size) = self.min_sstable_size {
            res.push(format!("'min_sstable_size': {}", min_sstable_size));
        }
        if let Some(bucket_low) = self.bucket_low {
            res.push(format!("'bucket_low': {:.1}", bucket_low));
        }
        if let Some(bucket_high) = self.bucket_high {
            res.push(format!("'bucket_high': {:.1}", bucket_high));
        }
        write!(f, "{{{}}}", res.join(", "))
    }
}

#[derive(Builder, Copy, Clone, Debug, Default)]
#[builder(setter(strip_option), default)]
pub struct LeveledCompactionStrategy {
    enabled: Option<bool>,
    tombstone_threshhold: Option<f32>,
    tombsone_compaction_interval: Option<i32>,
    log_all: Option<bool>,
    unchecked_tombstone_compaction: Option<bool>,
    only_purge_repaired_tombstone: Option<bool>,
    min_threshold: Option<i32>,
    max_threshold: Option<i32>,
    sstable_size_in_mb: Option<i32>,
    fanout_size: Option<i32>,
}

impl CompactionType for LeveledCompactionStrategy {}

impl Display for LeveledCompactionStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut res = vec![format!("'class': 'SizeTieredCompactionStrategy'")];
        if let Some(enabled) = self.enabled {
            res.push(format!("'enabled': {}", enabled));
        }
        if let Some(tombstone_threshhold) = self.tombstone_threshhold {
            res.push(format!("'tombstone_threshhold': {:.1}", tombstone_threshhold));
        }
        if let Some(tombsone_compaction_interval) = self.tombsone_compaction_interval {
            res.push(format!(
                "'tombsone_compaction_interval': {}",
                tombsone_compaction_interval
            ));
        }
        if let Some(log_all) = self.log_all {
            res.push(format!("'log_all': {}", log_all));
        }
        if let Some(unchecked_tombstone_compaction) = self.unchecked_tombstone_compaction {
            res.push(format!(
                "'unchecked_tombstone_compaction': {}",
                unchecked_tombstone_compaction
            ));
        }
        if let Some(only_purge_repaired_tombstone) = self.only_purge_repaired_tombstone {
            res.push(format!(
                "'only_purge_repaired_tombstone': {}",
                only_purge_repaired_tombstone
            ));
        }
        if let Some(min_threshold) = self.min_threshold {
            res.push(format!("'min_threshold': {}", min_threshold));
        }
        if let Some(max_threshold) = self.max_threshold {
            res.push(format!("'max_threshold': {}", max_threshold));
        }
        if let Some(sstable_size_in_mb) = self.sstable_size_in_mb {
            res.push(format!("'sstable_size_in_mb': {}", sstable_size_in_mb));
        }
        if let Some(fanout_size) = self.fanout_size {
            res.push(format!("'fanout_size': {}", fanout_size));
        }
        write!(f, "{{{}}}", res.join(", "))
    }
}

#[derive(Builder, Copy, Clone, Debug, Default)]
#[builder(setter(strip_option), default)]
pub struct TimeWindowCompactionStrategy {
    enabled: Option<bool>,
    tombstone_threshhold: Option<f32>,
    tombsone_compaction_interval: Option<i32>,
    log_all: Option<bool>,
    unchecked_tombstone_compaction: Option<bool>,
    only_purge_repaired_tombstone: Option<bool>,
    min_threshold: Option<i32>,
    max_threshold: Option<i32>,
    compaction_window_unit: Option<JavaTimeUnit>,
    compaction_window_size: Option<i32>,
    unsafe_aggressive_sstable_expiration: Option<bool>,
}

impl CompactionType for TimeWindowCompactionStrategy {}

impl Display for TimeWindowCompactionStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut res = vec![format!("'class': 'SizeTieredCompactionStrategy'")];
        if let Some(enabled) = self.enabled {
            res.push(format!("'enabled': {}", enabled));
        }
        if let Some(tombstone_threshhold) = self.tombstone_threshhold {
            res.push(format!("'tombstone_threshhold': {:.1}", tombstone_threshhold));
        }
        if let Some(tombsone_compaction_interval) = self.tombsone_compaction_interval {
            res.push(format!(
                "'tombsone_compaction_interval': {}",
                tombsone_compaction_interval
            ));
        }
        if let Some(log_all) = self.log_all {
            res.push(format!("'log_all': {}", log_all));
        }
        if let Some(unchecked_tombstone_compaction) = self.unchecked_tombstone_compaction {
            res.push(format!(
                "'unchecked_tombstone_compaction': {}",
                unchecked_tombstone_compaction
            ));
        }
        if let Some(only_purge_repaired_tombstone) = self.only_purge_repaired_tombstone {
            res.push(format!(
                "'only_purge_repaired_tombstone': {}",
                only_purge_repaired_tombstone
            ));
        }
        if let Some(min_threshold) = self.min_threshold {
            res.push(format!("'min_threshold': {}", min_threshold));
        }
        if let Some(max_threshold) = self.max_threshold {
            res.push(format!("'max_threshold': {}", max_threshold));
        }
        if let Some(compaction_window_unit) = self.compaction_window_unit {
            res.push(format!("'compaction_window_unit': {}", compaction_window_unit));
        }
        if let Some(compaction_window_size) = self.compaction_window_size {
            res.push(format!("'compaction_window_size': {}", compaction_window_size));
        }
        if let Some(unsafe_aggressive_sstable_expiration) = self.unsafe_aggressive_sstable_expiration {
            res.push(format!(
                "'unsafe_aggressive_sstable_expiration': {}",
                unsafe_aggressive_sstable_expiration
            ));
        }
        write!(f, "{{{}}}", res.join(", "))
    }
}

pub trait CompactionType: Display + Into<Compaction> {}

#[derive(Clone, Debug, From, TryInto)]
pub enum Compaction {
    SizeTiered(SizeTieredCompactionStrategy),
    Leveled(LeveledCompactionStrategy),
    TimeWindow(TimeWindowCompactionStrategy),
}

impl Compaction {
    pub fn size_tiered() -> SizeTieredCompactionStrategyBuilder
    where
        Self: Sized,
    {
        SizeTieredCompactionStrategyBuilder::default()
    }

    pub fn leveled() -> LeveledCompactionStrategyBuilder
    where
        Self: Sized,
    {
        LeveledCompactionStrategyBuilder::default()
    }

    pub fn time_window() -> TimeWindowCompactionStrategyBuilder
    where
        Self: Sized,
    {
        TimeWindowCompactionStrategyBuilder::default()
    }
}

impl TryFrom<MapLiteral> for Compaction {
    type Error = anyhow::Error;

    fn try_from(value: MapLiteral) -> Result<Self, Self::Error> {
        let mut class = None;
        let v = value
            .elements
            .into_iter()
            .filter(|(k, v)| {
                if let Term::Constant(Constant::String(s)) = k {
                    if s.value.to_lowercase().as_str() == "class" {
                        class = Some(v.clone());
                        return false;
                    }
                }
                true
            })
            .collect::<Vec<_>>();
        let class = class.ok_or_else(|| anyhow::anyhow!("No class in compaction map literal!"))?;
        Ok(match class {
            Term::Constant(Constant::String(s)) => {
                let mut map = HashMap::new();
                for (k, v) in v {
                    if let Term::Constant(Constant::String(s)) = k {
                        map.insert(s.value.to_lowercase(), v);
                    } else {
                        anyhow::bail!("Invalid key in compaction map literal!");
                    }
                }
                if s.value.ends_with("SizeTieredCompactionStrategy") {
                    let mut builder = Self::size_tiered();
                    if let Some(t) = map.remove("enabled") {
                        builder.enabled(t.try_into()?);
                    }
                    if let Some(t) = map.remove("tombstone_threshold") {
                        builder.tombstone_threshhold(t.try_into()?);
                    }
                    if let Some(t) = map.remove("tombstone_compaction_interval") {
                        builder.tombsone_compaction_interval(t.try_into()?);
                    }
                    if let Some(t) = map.remove("log_all") {
                        builder.log_all(t.try_into()?);
                    }
                    if let Some(t) = map.remove("unchecked_tombstone_compaction") {
                        builder.unchecked_tombstone_compaction(t.try_into()?);
                    }
                    if let Some(t) = map.remove("only_purge_repaired_tombstone") {
                        builder.only_purge_repaired_tombstone(t.try_into()?);
                    }
                    if let Some(t) = map.remove("min_threshold") {
                        builder.min_threshold(t.try_into()?);
                    }
                    if let Some(t) = map.remove("max_threshold") {
                        builder.max_threshold(t.try_into()?);
                    }
                    if let Some(t) = map.remove("min_sstable_size") {
                        builder.min_sstable_size(t.try_into()?);
                    }
                    if let Some(t) = map.remove("bucket_low") {
                        builder.bucket_low(t.try_into()?);
                    }
                    if let Some(t) = map.remove("bucket_high") {
                        builder.bucket_high(t.try_into()?);
                    }
                    Compaction::SizeTiered(builder.build()?)
                } else if s.value.ends_with("LeveledCompactionStrategy") {
                    let mut builder = Self::leveled();
                    if let Some(t) = map.remove("enabled") {
                        builder.enabled(t.try_into()?);
                    }
                    if let Some(t) = map.remove("tombstone_threshold") {
                        builder.tombstone_threshhold(t.try_into()?);
                    }
                    if let Some(t) = map.remove("tombstone_compaction_interval") {
                        builder.tombsone_compaction_interval(t.try_into()?);
                    }
                    if let Some(t) = map.remove("log_all") {
                        builder.log_all(t.try_into()?);
                    }
                    if let Some(t) = map.remove("unchecked_tombstone_compaction") {
                        builder.unchecked_tombstone_compaction(t.try_into()?);
                    }
                    if let Some(t) = map.remove("only_purge_repaired_tombstone") {
                        builder.only_purge_repaired_tombstone(t.try_into()?);
                    }
                    if let Some(t) = map.remove("min_threshold") {
                        builder.min_threshold(t.try_into()?);
                    }
                    if let Some(t) = map.remove("max_threshold") {
                        builder.max_threshold(t.try_into()?);
                    }
                    if let Some(t) = map.remove("sstable_size_in_mb") {
                        builder.sstable_size_in_mb(t.try_into()?);
                    }
                    if let Some(t) = map.remove("fanout_size") {
                        builder.fanout_size(t.try_into()?);
                    }
                    Compaction::Leveled(builder.build()?)
                } else if s.value.ends_with("TimeWindowCompactionStrategy") {
                    let mut builder = Self::time_window();
                    if let Some(t) = map.remove("enabled") {
                        builder.enabled(t.try_into()?);
                    }
                    if let Some(t) = map.remove("tombstone_threshold") {
                        builder.tombstone_threshhold(t.try_into()?);
                    }
                    if let Some(t) = map.remove("tombstone_compaction_interval") {
                        builder.tombsone_compaction_interval(t.try_into()?);
                    }
                    if let Some(t) = map.remove("log_all") {
                        builder.log_all(t.try_into()?);
                    }
                    if let Some(t) = map.remove("unchecked_tombstone_compaction") {
                        builder.unchecked_tombstone_compaction(t.try_into()?);
                    }
                    if let Some(t) = map.remove("only_purge_repaired_tombstone") {
                        builder.only_purge_repaired_tombstone(t.try_into()?);
                    }
                    if let Some(t) = map.remove("min_threshold") {
                        builder.min_threshold(t.try_into()?);
                    }
                    if let Some(t) = map.remove("max_threshold") {
                        builder.max_threshold(t.try_into()?);
                    }
                    if let Some(t) = map.remove("compaction_window_unit") {
                        builder.compaction_window_unit(TryInto::<LitStr>::try_into(t)?.value.parse()?);
                    }
                    if let Some(t) = map.remove("compaction_window_size") {
                        builder.compaction_window_size(t.try_into()?);
                    }
                    if let Some(t) = map.remove("unsafe_aggressive_sstable_expiration") {
                        builder.unsafe_aggressive_sstable_expiration(t.try_into()?);
                    }
                    Compaction::TimeWindow(builder.build()?)
                } else {
                    return Err(anyhow::anyhow!("Unknown compaction class: {}", s));
                }
            }
            _ => anyhow::bail!("Invalid class: {}", class),
        })
    }
}

impl Display for Compaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Compaction::SizeTiered(s) => s.fmt(f),
            Compaction::Leveled(s) => s.fmt(f),
            Compaction::TimeWindow(s) => s.fmt(f),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum JavaTimeUnit {
    Minutes,
    Hours,
    Days,
}

impl FromStr for JavaTimeUnit {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MINUTES" => Ok(JavaTimeUnit::Minutes),
            "HOURS" => Ok(JavaTimeUnit::Hours),
            "DAYS" => Ok(JavaTimeUnit::Days),
            _ => Err(anyhow::anyhow!("Invalid time unit: {}", s)),
        }
    }
}

impl Parse for JavaTimeUnit {
    type Output = Self;

    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        s.parse::<LitStr>()?.value.parse()
    }
}

impl Display for JavaTimeUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaTimeUnit::Minutes => write!(f, "MINUTES"),
            JavaTimeUnit::Hours => write!(f, "HOURS"),
            JavaTimeUnit::Days => write!(f, "DAYS"),
        }
    }
}

#[derive(Builder, Clone, Debug, Default)]
#[builder(setter(strip_option), default)]
pub struct Compression {
    #[builder(setter(into))]
    class: Option<LitStr>,
    enabled: Option<bool>,
    chunk_length_in_kb: Option<i32>,
    crc_check_chance: Option<f32>,
    compression_level: Option<i32>,
}

impl Compression {
    pub fn build() -> CompressionBuilder {
        CompressionBuilder::default()
    }
}

impl Display for Compression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut res = Vec::new();
        if let Some(class) = &self.class {
            res.push(format!("'class': {}", class));
        }
        if let Some(enabled) = &self.enabled {
            res.push(format!("'enabled': {}", enabled));
        }
        if let Some(chunk_length_in_kb) = &self.chunk_length_in_kb {
            res.push(format!("'chunk_length_in_kb': {}", chunk_length_in_kb));
        }
        if let Some(crc_check_chance) = &self.crc_check_chance {
            res.push(format!("'crc_check_chance': {:.1}", crc_check_chance));
        }
        if let Some(compression_level) = &self.compression_level {
            res.push(format!("'compression_level': {}", compression_level));
        }
        write!(f, "{{{}}}", res.join(", "))
    }
}

impl TryFrom<MapLiteral> for Compression {
    type Error = anyhow::Error;

    fn try_from(value: MapLiteral) -> Result<Self, Self::Error> {
        let mut map = HashMap::new();
        for (k, v) in value.elements {
            if let Term::Constant(Constant::String(s)) = k {
                map.insert(s.value.to_lowercase(), v);
            } else {
                anyhow::bail!("Invalid key in compaction map literal!");
            }
        }
        let mut builder = Self::build();
        if let Some(t) = map.remove("class") {
            builder.class(TryInto::<LitStr>::try_into(t)?);
        }
        if let Some(t) = map.remove("enabled") {
            builder.enabled(t.try_into()?);
        }
        if let Some(t) = map.remove("chunk_length_in_kb") {
            builder.chunk_length_in_kb(t.try_into()?);
        }
        if let Some(t) = map.remove("crc_check_chance") {
            builder.crc_check_chance(t.try_into()?);
        }
        if let Some(t) = map.remove("compression_level") {
            builder.compression_level(t.try_into()?);
        }
        Ok(builder.build()?)
    }
}

#[derive(Builder, Clone, Debug, Default)]
#[builder(setter(strip_option), default)]
pub struct Caching {
    keys: Option<Keys>,
    rows_per_partition: Option<RowsPerPartition>,
}

impl Caching {
    pub fn build() -> CachingBuilder {
        CachingBuilder::default()
    }
}

impl Display for Caching {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut res = Vec::new();
        if let Some(keys) = &self.keys {
            res.push(format!("'keys': {}", keys));
        }
        if let Some(rows_per_partition) = &self.rows_per_partition {
            res.push(format!("'rows_per_partition': {}", rows_per_partition));
        }
        write!(f, "{{{}}}", res.join(", "))
    }
}

impl TryFrom<MapLiteral> for Caching {
    type Error = anyhow::Error;

    fn try_from(value: MapLiteral) -> Result<Self, Self::Error> {
        let mut map = HashMap::new();
        for (k, v) in value.elements {
            if let Term::Constant(Constant::String(s)) = k {
                map.insert(s.value.to_lowercase(), v);
            } else {
                anyhow::bail!("Invalid key in compaction map literal!");
            }
        }
        let mut builder = Self::build();
        if let Some(t) = map.remove("keys") {
            builder.keys(TryInto::<LitStr>::try_into(t)?.value.parse()?);
        }
        if let Some(t) = map.remove("rows_per_partition") {
            builder.rows_per_partition(t.to_string().parse()?);
        }
        Ok(builder.build()?)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Keys {
    All,
    None,
}

impl FromStr for Keys {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ALL" => Ok(Keys::All),
            "NONE" => Ok(Keys::None),
            _ => Err(anyhow::anyhow!("Invalid keys: {}", s)),
        }
    }
}

impl Parse for Keys {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        s.parse::<LitStr>()?.value.parse()
    }
}

impl Display for Keys {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Keys::All => write!(f, "'ALL'"),
            Keys::None => write!(f, "'NONE'"),
        }
    }
}

#[derive(ParseFromStr, Copy, Clone, Debug)]
pub enum RowsPerPartition {
    All,
    None,
    Count(i32),
}

impl Parse for RowsPerPartition {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if let Some(ss) = s.parse::<Option<LitStr>>()? {
            match ss.value.to_uppercase().as_str() {
                "ALL" => RowsPerPartition::All,
                "NONE" => RowsPerPartition::None,
                _ => anyhow::bail!("Invalid rows_per_partition: {}", ss),
            }
        } else {
            Self::Count(s.parse()?)
        })
    }
}

impl Display for RowsPerPartition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RowsPerPartition::All => write!(f, "'ALL'"),
            RowsPerPartition::None => write!(f, "'NONE'"),
            RowsPerPartition::Count(count) => count.fmt(f),
        }
    }
}
