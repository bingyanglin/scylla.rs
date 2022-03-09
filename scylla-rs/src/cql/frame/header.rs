// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! This module defines the header trait.

use super::{
    FromPayload,
    OpCode,
    ToPayload,
};
use std::convert::{
    TryFrom,
    TryInto,
};

/// Direction of a request.
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum Direction {
    Request = 0,
    Response = 1,
}

/**
    The version is a single byte that indicates both the direction of the message
    (request or response) and the version of the protocol in use. The most
    significant bit of version is used to define the direction of the message:
    0 indicates a request, 1 indicates a response. This can be useful for protocol
    analyzers to distinguish the nature of the packet from the direction in which
    it is moving. The rest of that byte is the protocol version (4 for the protocol
    defined in this document). In other words, for this version of the protocol,
    version will be one of:
    - `0x0V`: Request frame for this protocol version
    - `0x8V`: Response frame for this protocol version

    Please note that while every message ships with the version, only one version
    of messages is accepted on a given connection. In other words, the first message
    exchanged (STARTUP) sets the version for the connection for the lifetime of this
    connection.
*/
#[derive(Copy, Clone, Debug)]
pub struct Version(u8);

impl Default for Version {
    fn default() -> Self {
        Self(0x04)
    }
}

impl Version {
    /// The direction of the frame, either request or response.
    pub fn direction(&self) -> Direction {
        match self.0 & 0x80 {
            0 => Direction::Request,
            _ => Direction::Response,
        }
    }

    /// The version of the protocol.
    pub fn version(&self) -> u8 {
        self.0 & 0x7f
    }
}

/**
    Flags applying to this frame. The flags have the following meaning (described
    by the mask that allows selecting them):

    - `0x01`: Compression flag. If set, the frame body is compressed. The actual
            compression to use should have been set up beforehand through the
            Startup message.
    - `0x02`: Tracing flag. For a request frame, this indicates the client requires
            tracing of the request. Note that only QUERY, PREPARE and EXECUTE queries
            support tracing. Other requests will simply ignore the tracing flag if
            set. If a request supports tracing and the tracing flag is set, the response
            to this request will have the tracing flag set and contain tracing
            information.
            If a response frame has the tracing flag set, its body contains
            a tracing ID. The tracing ID is a `[uuid]` and is the first thing in
            the frame body.
    - `0x04`: Custom payload flag. For a request or response frame, this indicates
            that a generic key-value custom payload for a custom QueryHandler
            implementation is present in the frame. Such a custom payload is simply
            ignored by the default QueryHandler implementation.
            Currently, only QUERY, PREPARE, EXECUTE and BATCH requests support
            payload.
            Type of custom payload is `[bytes map]`. If either or both
            of the tracing and warning flags are set, the custom payload will follow
            those indicated elements in the frame body. If neither are set, the custom
            payload will be the first value in the frame body.
    - `0x08`: Warning flag. The response contains warnings which were generated by the
            server to go along with this response.
            If a response frame has the warning flag set, its body will contain the
            text of the warnings. The warnings are a `[string list]` and will be the
            first value in the frame body if the tracing flag is not set, or directly
            after the tracing ID if it is.

    The rest of flags is currently unused and ignored.
*/
#[derive(Copy, Clone, Debug, Default)]
#[repr(transparent)]
pub struct HeaderFlags(u8);

impl HeaderFlags {
    /// The compression flag.
    pub const COMPRESSION: u8 = 0x01;
    /// The tracing flag.
    pub const TRACING: u8 = 0x02;
    /// The custoem payload flag.
    pub const CUSTOM_PAYLOAD: u8 = 0x04;
    /// The warning flag.
    pub const WARNING: u8 = 0x08;

    /// Compression flag. If set, the frame body is compressed. The actual
    /// compression to use should have been set up beforehand through the
    /// Startup message.
    pub fn compression(&self) -> bool {
        self.0 & Self::COMPRESSION != 0
    }

    /// Set the compression flag.
    pub fn set_compression(&mut self, value: bool) {
        if value {
            self.0 |= Self::COMPRESSION;
        } else {
            self.0 &= !Self::COMPRESSION;
        }
    }

    /// Tracing flag. For a request frame, this indicates the client requires
    /// tracing of the request. Note that only QUERY, PREPARE and EXECUTE queries
    /// support tracing. Other requests will simply ignore the tracing flag if
    /// set. If a request supports tracing and the tracing flag is set, the response
    /// to this request will have the tracing flag set and contain tracing
    /// information.
    /// If a response frame has the tracing flag set, its body contains
    /// a tracing ID. The tracing ID is a `[uuid]` and is the first thing in
    /// the frame body.
    pub fn tracing(&self) -> bool {
        self.0 & Self::TRACING != 0
    }

    /// Set the tracing flag.
    pub fn set_tracing(&mut self, value: bool) {
        if value {
            self.0 |= Self::TRACING;
        } else {
            self.0 &= !Self::TRACING;
        }
    }

    /// Custom payload flag. For a request or response frame, this indicates
    /// that a generic key-value custom payload for a custom QueryHandler
    /// implementation is present in the frame. Such a custom payload is simply
    /// ignored by the default QueryHandler implementation.
    /// Currently, only QUERY, PREPARE, EXECUTE and BATCH requests support
    /// payload.
    /// Type of custom payload is `[bytes map]`. If either or both
    /// of the tracing and warning flags are set, the custom payload will follow
    /// those indicated elements in the frame body. If neither are set, the custom
    /// payload will be the first value in the frame body.
    pub fn custom_payload(&self) -> bool {
        self.0 & Self::CUSTOM_PAYLOAD != 0
    }

    /// Set the custom payload flag.
    pub fn set_custom_payload(&mut self, value: bool) {
        if value {
            self.0 |= Self::CUSTOM_PAYLOAD;
        } else {
            self.0 &= !Self::CUSTOM_PAYLOAD;
        }
    }

    /// Warning flag. The response contains warnings which were generated by the
    /// server to go along with this response.
    /// If a response frame has the warning flag set, its body will contain the
    /// text of the warnings. The warnings are a `[string list]` and will be the
    /// first value in the frame body if the tracing flag is not set, or directly
    /// after the tracing ID if it is.
    pub fn warning(&self) -> bool {
        self.0 & Self::WARNING != 0
    }

    /// Set the warning flag.
    pub fn set_warning(&mut self, value: bool) {
        if value {
            self.0 |= Self::WARNING;
        } else {
            self.0 &= !Self::WARNING;
        }
    }
}

/// The full header of a scylla frame, which contains the protocol version, frame flags, stream id, opcode, and body
/// length.
#[derive(Copy, Clone, Debug)]
pub struct Header {
    version: Version,
    flags: HeaderFlags,
    stream: u16,
    opcode: OpCode,
    body_len: u32,
}

impl Header {
    /// The direction of the frame. See [`Version`] for more information.
    pub fn direction(&self) -> Direction {
        self.version.direction()
    }

    /// The protocol version of the frame. See [`Version`] for more information.
    pub fn version(&self) -> u8 {
        self.version.version()
    }

    /// The mutable protocol version of the frame. See [`Version`] for more information.
    pub fn version_mut(&mut self) -> &mut Version {
        &mut self.version
    }

    /// The flags of the frame.
    pub fn flags(&self) -> &HeaderFlags {
        &self.flags
    }

    /// The mutable flags of the frame.
    pub fn flags_mut(&mut self) -> &mut HeaderFlags {
        &mut self.flags
    }

    /// The compression flag of the frame. See [`HeaderFlags`] for more information.
    pub fn compression(&self) -> bool {
        self.flags.compression()
    }

    /// The tracing flag of the frame. See [`HeaderFlags`] for more information.
    pub fn tracing(&self) -> bool {
        self.flags.tracing()
    }

    /// The custom payload flag of the frame. See [`HeaderFlags`] for more information.
    pub fn custom_payload(&self) -> bool {
        self.flags.custom_payload()
    }

    /// The warning flag of the frame. See [`HeaderFlags`] for more information.
    pub fn warning(&self) -> bool {
        self.flags.warning()
    }

    /// The stream id of the frame. When sending request messages, this
    /// stream id must be set by the client to a non-negative value (negative stream id
    /// are reserved for streams initiated by the server; currently all EVENT messages
    /// have a streamId of -1). If a client sends a request message
    /// with the stream id X, it is guaranteed that the stream id of the response to
    /// that message will be X.
    ///
    /// This helps to enable the asynchronous nature of the protocol. If a client
    /// sends multiple messages simultaneously (without waiting for responses), there
    /// is no guarantee on the order of the responses. For instance, if the client
    /// writes REQ_1, REQ_2, REQ_3 on the wire (in that order), the server might
    /// respond to REQ_3 (or REQ_2) first. Assigning different stream ids to these 3
    /// requests allows the client to distinguish to which request a received answer
    /// responds to. As there can only be 32768 different simultaneous streams, it is up
    /// to the client to reuse stream id.
    ///
    /// Note that clients are free to use the protocol synchronously (i.e. wait for
    /// the response to REQ_N before sending REQ_N+1). In that case, the stream id
    /// can be safely set to 0. Clients should also feel free to use only a subset of
    /// the 32768 maximum possible stream ids if it is simpler for its implementation.
    pub fn stream(&self) -> u16 {
        self.stream
    }

    /// Set the stream id of the frame.
    pub fn set_stream(&mut self, stream: u16) {
        self.stream = stream;
    }

    /// An integer byte that distinguishes the actual message:
    /// - `0x00`: ERROR
    /// - `0x01`: STARTUP
    /// - `0x02`: READY
    /// - `0x03`: AUTHENTICATE
    /// - `0x05`: OPTIONS
    /// - `0x06`: SUPPORTED
    /// - `0x07`: QUERY
    /// - `0x08`: RESULT
    /// - `0x09`: PREPARE
    /// - `0x0A`: EXECUTE
    /// - `0x0B`: REGISTER
    /// - `0x0C`: EVENT
    /// - `0x0D`: BATCH
    /// - `0x0E`: AUTH_CHALLENGE
    /// - `0x0F`: AUTH_RESPONSE
    /// - `0x10`: AUTH_SUCCESS
    pub fn opcode(&self) -> OpCode {
        self.opcode
    }

    /// Set the opcode of the frame.
    pub fn set_opcode(&mut self, opcode: OpCode) {
        self.opcode = opcode;
    }

    /// The length of the body of the frame.
    pub fn body_len(&self) -> u32 {
        self.body_len
    }

    /// Set the length of the body of the frame.
    pub fn set_body_len(&mut self, body_len: u32) {
        self.body_len = body_len;
    }

    /// Create a default header for a frame given an opcode.
    pub fn from_opcode(opcode: OpCode) -> Self {
        Self {
            version: Version::default(),
            flags: HeaderFlags::default(),
            stream: 0,
            opcode,
            body_len: 0,
        }
    }
}

impl TryFrom<&[u8]> for Header {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        anyhow::ensure!(bytes.len() == 9, "Invalid header length");
        Ok(Header {
            version: Version(bytes[0]),
            flags: HeaderFlags(bytes[1]),
            stream: u16::from_be_bytes([bytes[2], bytes[3]]),
            opcode: bytes[4].try_into()?,
            body_len: u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]),
        })
    }
}

impl FromPayload for Header {
    fn from_payload(start: &mut usize, payload: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(payload.len() >= *start + 9, "Payload is too small");
        let header = payload[*start..][..9].try_into()?;
        *start += 9;
        Ok(header)
    }
}

impl ToPayload for Header {
    fn to_payload(self, payload: &mut Vec<u8>) {
        if self.body_len() > 0 {
            payload.reserve(9 + self.body_len() as usize);
        }
        payload.extend(Into::<[u8; 9]>::into(self));
    }
}

impl Into<[u8; 9]> for Header {
    fn into(self) -> [u8; 9] {
        [
            self.version.0,
            self.flags.0,
            (self.stream >> 8) as u8,
            self.stream as u8,
            self.opcode as u8,
            (self.body_len >> 24) as u8,
            (self.body_len >> 16) as u8,
            (self.body_len >> 8) as u8,
            self.body_len as u8,
        ]
    }
}
