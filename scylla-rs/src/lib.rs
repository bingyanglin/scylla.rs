// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0
pub mod cql;
#[cfg(not(feature = "app"))]
pub use cql::*;
#[cfg(feature = "app")]
pub mod app;

#[cfg(feature = "app")]
pub mod prelude {
    pub use super::{
        app::{
            access::*,
            cluster::ClusterHandleExt,
            worker::*,
            ScyllaHandleExt,
            *,
        },
        cql::{
            compression::CompressionType,
            Batch,
            Binder,
            ColumnDecoder,
            ColumnEncoder,
            ColumnValue,
            Consistency,
            Decoder,
            Frame,
            Iter,
            Prepare,
            Query,
            Row,
            Rows,
            RowsDecoder,
            TokenEncodeChain,
            TokenEncoder,
            VoidDecoder,
        },
    };
    pub use backstage::core::*;
    pub use maplit::{
        self,
        *,
    };
    pub use scylla_parse;
    pub use scylla_rs_macros::*;
}
