// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! This module defines the row/column decoder/encoder for the frame structure.

use super::{ColumnDecoder, Frame};
use std::{
    collections::HashMap,
    convert::TryInto,
    hash::Hash,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str,
};

/// The column count type.
pub type ColumnsCount = i32;
#[derive(Debug)]
/// The flags for row decoder.
pub struct Flags {
    global_table_spec: bool,
    has_more_pages: bool,
    no_metadata: bool,
}

impl Flags {
    /// Decode i32 to flags of row decoder.
    pub fn from_i32(flags: i32) -> Self {
        Flags {
            global_table_spec: (flags & 1) == 1,
            has_more_pages: (flags & 2) == 2,
            no_metadata: (flags & 4) == 4,
        }
    }
    /// Check if are there more pages to decode.
    pub fn has_more_pages(&self) -> bool {
        self.has_more_pages
    }
}
#[derive(Debug)]
/// The pageing state of the response.
pub struct PagingState {
    paging_state: Option<Vec<u8>>,
    end: usize,
}
impl PagingState {
    /// Create a new paing state.
    pub fn new(paging_state: Option<Vec<u8>>, end: usize) -> Self {
        PagingState { paging_state, end }
    }
}
#[derive(Debug)]
/// The meta structure of the row.
pub struct Metadata {
    flags: Flags,
    columns_count: ColumnsCount,
    paging_state: PagingState,
}

impl Metadata {
    /// Create a new meta data.
    pub fn new(flags: Flags, columns_count: ColumnsCount, paging_state: PagingState) -> Self {
        Metadata {
            flags,
            columns_count,
            paging_state,
        }
    }
    /// Get the starting rows.
    pub fn rows_start(&self) -> usize {
        self.paging_state.end
    }
    /// Take the paging state of the metadata.
    pub fn take_paging_state(&mut self) -> Option<Vec<u8>> {
        self.paging_state.paging_state.take()
    }
}

/// Rows trait to decode the final result from scylla
pub trait Rows: Iterator {
    /// create new rows decoder struct
    fn new(decoder: super::decoder::Decoder) -> Self;
    fn take_paging_state(&mut self) -> Option<Vec<u8>>;
}

// TODO impl columns specs for std types
pub trait Row: Sized {
    /// Define how to decode the row
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized;
}
pub trait ColumnValue {
    /// Decode the column value of C type;
    fn column_value<C: ColumnDecoder>(&mut self) -> C;
}

#[allow(unused)]
pub struct Iter<T: Row> {
    decoder: super::Decoder,
    rows_count: usize,
    column_start: usize,
    remaining_rows_count: usize,
    metadata: Metadata,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Row> Rows for Iter<T> {
    fn new(decoder: super::Decoder) -> Self {
        let metadata = decoder.metadata();
        let rows_start = metadata.rows_start();
        let column_start = rows_start + 4;
        let rows_count = i32::from_be_bytes(decoder.buffer_as_ref()[rows_start..column_start].try_into().unwrap());
        Self {
            decoder,
            metadata,
            rows_count: rows_count as usize,
            remaining_rows_count: rows_count as usize,
            column_start,
            _marker: std::marker::PhantomData,
        }
    }
    fn take_paging_state(&mut self) -> Option<Vec<u8>> {
        self.metadata.take_paging_state()
    }
}

impl<T: Row> Iterator for Iter<T> {
    type Item = T;
    /// Note the row decoder is implemented in this `next` method of HardCodedSpecs.
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.remaining_rows_count > 0 {
            self.remaining_rows_count -= 1;
            Some(T::decode_row(self).into())
        } else {
            None
        }
    }
}

impl<T: Row> ColumnValue for Iter<T> {
    fn column_value<C: ColumnDecoder>(&mut self) -> C {
        let length = i32::from_be_bytes(
            self.decoder.buffer_as_ref()[self.column_start..][..4]
                .try_into()
                .unwrap(),
        );
        self.column_start += 4; // now it become the column_value start, or next column_start if length < 0
        if length > 0 {
            let col_slice = self.decoder.buffer_as_ref()[self.column_start..][..(length as usize)].into();
            // update the next column_start to start from next column
            self.column_start += length as usize;
            C::decode(col_slice)
        } else {
            C::decode(&[])
        }
    }
}

macro_rules! row {
    (@tuple ($($t:tt),*)) => {
        impl<$($t: ColumnDecoder),*> Row for ($($t,)*) {
            fn decode_row<R: ColumnValue>(rows: &mut R) -> Self {
                (
                    $(
                        rows.column_value::<$t>(),
                    )*
                )

            }
        }
    };
}

// HardCoded Specs,
row!(@tuple (T));
row!(@tuple (T,TT));
row!(@tuple (T, TT, TTT));
row!(@tuple (T, TT, TTT, TTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT, TTTTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT, TTTTTTT, TTTTTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT, TTTTTTT, TTTTTTTT, TTTTTTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT, TTTTTTT, TTTTTTTT, TTTTTTTTT, TTTTTTTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT, TTTTTTT, TTTTTTTT, TTTTTTTTT, TTTTTTTTTT, TTTTTTTTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT, TTTTTTT, TTTTTTTT, TTTTTTTTT, TTTTTTTTTT, TTTTTTTTTTT, TTTTTTTTTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT, TTTTTTT, TTTTTTTT, TTTTTTTTT, TTTTTTTTTT, TTTTTTTTTTT, TTTTTTTTTTTT, TTTTTTTTTTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT, TTTTTTT, TTTTTTTT, TTTTTTTTT, TTTTTTTTTT, TTTTTTTTTTT, TTTTTTTTTTTT, TTTTTTTTTTTTT, TTTTTTTTTTTTTT));
row!(@tuple (T, TT, TTT, TTTT, TTTTT, TTTTTT, TTTTTTT, TTTTTTTT, TTTTTTTTT, TTTTTTTTTT, TTTTTTTTTTT, TTTTTTTTTTTT, TTTTTTTTTTTTT, TTTTTTTTTTTTTT, TTTTTTTTTTTTTTT));

impl<T: ColumnDecoder> Row for Option<T> {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for i64 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for u64 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for f64 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for i32 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for u32 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for f32 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for i16 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for u16 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for i8 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for u8 {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for String {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for std::net::IpAddr {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for Ipv4Addr {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl Row for Ipv6Addr {
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl<E> Row for Vec<E>
where
    E: ColumnDecoder,
{
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

impl<K, V, S> Row for HashMap<K, V, S>
where
    K: Eq + Hash + ColumnDecoder,
    V: ColumnDecoder,
    S: ::std::hash::BuildHasher + Default,
{
    fn decode_row<R: Rows + ColumnValue>(rows: &mut R) -> Self
    where
        Self: Sized,
    {
        rows.column_value()
    }
}

#[macro_export]
/// The rows macro implements the row decoder.
macro_rules! rows {
    (@common_rows $rows:ident$(<$($t:ident),+>)?) => {
        #[allow(dead_code)]
        #[allow(unused_parens)]
        /// The `rows` struct for processing each received row in ScyllaDB.
        pub struct $rows$(<$($t),+>)? {
            decoder: Decoder,
            rows_count: usize,
            remaining_rows_count: usize,
            metadata: Metadata,
            column_start: usize,
            $(_marker: PhantomData<($($t),+)>,)?
        }

        impl$(<$($t),+>)? $rows$(<$($t),+>)? {
            #[allow(dead_code)]
            pub fn rows_count(&self) -> usize {
                self.rows_count
            }

            #[allow(dead_code)]
            pub fn remaining_rows_count(&self) -> usize {
                self.remaining_rows_count
            }
        }

        #[allow(unused_parens)]
        impl$(<$($t),+>)? Rows for $rows$(<$($t),+>)? {
            /// Create a new rows structure.
            fn new(decoder: Decoder) -> Self {
                let metadata = decoder.metadata();
                let rows_start = metadata.rows_start();
                let column_start = rows_start + 4;
                let rows_count = i32::from_be_bytes(
                    decoder.buffer_as_ref()[rows_start..column_start]
                        .try_into()
                        .unwrap(),
                );
                Self {
                    decoder,
                    metadata,
                    rows_count: rows_count as usize,
                    remaining_rows_count: rows_count as usize,
                    column_start,
                    $(_marker: PhantomData::<($($t),+)>,)?
                }
            }
            fn take_paging_state(&mut self) -> Option<Vec<u8>> {
                self.metadata.take_paging_state()
            }
        }
    };
    (@common_row $row:ident {$( $col_field:ident: $col_type:ty),*}) => {
        /// It's the `row` struct
        pub struct $row {
            $(
                pub $col_field: $col_type,
            )*
        }
    };
    (@common_iter $rows:ident$(<$($t:ident),+>)?, $row:ident {$( $col_field:ident: $col_type:ty),*}, $row_into:ty) => {
        impl$(<$($t),+>)? Iterator for $rows$(<$($t),+>)? {
            type Item = $row_into;
            /// Note the row decoder is implemented in this `next` method.
            fn next(&mut self) -> Option<<Self as Iterator>::Item> {
                if self.remaining_rows_count > 0 {
                    self.remaining_rows_count -= 1;
                    let row_struct = $row {
                        $(
                            $col_field: {
                                let length = i32::from_be_bytes(
                                    self.decoder.buffer_as_ref()[self.column_start..][..4].try_into().unwrap()
                                );
                                self.column_start += 4; // now it become the column_value start, or next column_start if length < 0
                                if length > 0 {
                                    let col_slice = self.decoder.buffer_as_ref()[self.column_start..][..(length as usize)].into();
                                    // update the next column_start to start from next column
                                    self.column_start += (length as usize);
                                    <$col_type>::decode(col_slice)
                                } else {
                                    <$col_type>::decode(&[])
                                }
                            },
                        )*
                    };
                    Some(row_struct.into())
                } else {
                    None
                }
            }
        }
    };
    (single_row: $rows:ident$(<$($t:ident),+>)?, row: $row:ident {$( $col_field:ident: $col_type:ty),* $(,)?}, row_into: $row_into:ty $(,)? ) => {
        rows!(@common_rows $rows$(<$($t),+>)?);

        rows!(@common_row $row {$( $col_field: $col_type),*});

        rows!(@common_iter $rows$(<$($t),+>)?, $row {$( $col_field: $col_type),*}, $row_into);

        impl $rows {
            pub fn get(&mut self) -> Option<$row_into> {
                self.next()
            }
        }
    };
    (rows: $rows:ident$(<$($t:ident),+>)?, row: $row:ty, row_into: $row_into:ty $(,)? ) => {
        rows!(@common_rows $rows$(<$($t),+>)?);

        impl$(<$($t),+>)? Iterator for $rows$(<$($t),+>)? {
            type Item = $row_into;
            /// Note the row decoder is implemented in this `next` method.
            fn next(&mut self) -> Option<<Self as Iterator>::Item> {
                if self.remaining_rows_count > 0 {
                    self.remaining_rows_count -= 1;
                    let length = i32::from_be_bytes(
                        self.decoder.buffer_as_ref()[self.column_start..][..4]
                            .try_into()
                            .unwrap(),
                    );
                    self.column_start += 4; // now it become the column_value start, or next column_start if length < 0
                    if length > 0 {
                        let col_slice = self.decoder.buffer_as_ref()[self.column_start..][..(length as usize)].into();
                        // update the next column_start to start from next column
                        self.column_start += (length as usize);
                        Some(<$row>::decode(col_slice).into())
                    } else {
                        Some(<$row>::decode(&[]).into())
                    }
                } else {
                    None
                }
            }
        }
    };
    (rows: $rows:ident$(<$($t:ident),+>)?, row: $row:ident {$( $col_field:ident: $col_type:ty),* $(,)?}, row_into: $row_into:ty $(,)? ) => {
        rows!(@common_rows $rows$(<$($t),+>)?);

        rows!(@common_row $row {$( $col_field: $col_type),*});

        rows!(@common_iter $rows$(<$($t),+>)?, $row {$( $col_field: $col_type),*}, $row_into);
    };
}
