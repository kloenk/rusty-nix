/*
 * This file was part of nix_cfg, a parser for the Nix configuration format.
 * now adapted to libutil a general nix util library
 * Copyright © 2020 Milan Pässler
 * Copyright © 2020 Finn Behrens
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fmt::Display;

use custom_error::custom_error;

custom_error! {
    pub Error
        ParseError{source: ParseError} = "Parsing Error: {source}",
        Io{source: std::io::Error} = "IoError: {source}",
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    Message(String),
    TrailingCharacters,
    ExpectedMapNewline,
    ExpectedMapEquals,
    ExpectedBool,
    ExpectedInteger,
    Eof,
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

impl std::error::Error for ParseError {}

impl serde::de::Error for ParseError {
    fn custom<T: Display>(msg: T) -> Self {
        ParseError::Message(msg.to_string())
    }
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "{:?}", self)
    }
}
