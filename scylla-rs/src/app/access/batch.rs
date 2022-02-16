// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{
    delete::DeleteTable,
    insert::InsertTable,
    update::UpdateTable,
    *,
};
use crate::cql::{
    BatchBuilder,
    BatchType,
    Consistency,
};
use core::fmt::Debug;
use std::collections::HashMap;

/// A batch collector, used to collect statements and build a `BatchRequest`.
/// Access queries are defined by access traits ([`Insert`], [`Delete`], [`Update`])
/// and qualified for use in a Batch via batch traits ([`InsertBatch`], [`DeleteBatch`], [`UpdateBatch`])
/// ## Example
/// ```no_run
/// # #[derive(Default, Clone, Debug)]
/// # pub struct MyKeyspace {
/// #     pub name: String,
/// # }
/// #
/// # impl MyKeyspace {
/// #     pub fn new() -> Self {
/// #         Self {
/// #             name: "my_keyspace".into(),
/// #         }
/// #     }
/// # }
/// #
/// # impl ToString for MyKeyspace {
/// #     fn to_string(&self) -> String {
/// #         self.name.to_string()
/// #     }
/// # }
/// #
/// # impl Insert<u32, f32> for MyKeyspace {
/// #     type QueryOrPrepared = PreparedStatement;
/// #     fn statement(&self) -> InsertStatement {
/// #         parse_statement!("INSERT INTO my_table (key, val1, val2) VALUES (?,?,?)")
/// #     }
/// #
/// #     fn bind_values<B: Binder>(binder: B, key: &u32, values: &f32) -> B {
/// #         binder.value(key).value(values).value(values)
/// #     }
/// # }
/// #
/// # impl Update<u32, (), f32> for MyKeyspace {
/// #     type QueryOrPrepared = PreparedStatement;
/// #     fn statement(&self) -> UpdateStatement {
/// #         parse_statement!("UPDATE my_keyspace.my_table SET val1 = ?, val2 = ? WHERE key = ?")
/// #     }
/// #
/// #     fn bind_values<B: Binder>(binder: B, key: &u32, _variables: &(), values: &f32) -> B {
/// #         binder.value(values).value(values).value(key)
/// #     }
/// # }
/// #
/// # impl Delete<u32, (), f32> for MyKeyspace {
/// #     type QueryOrPrepared = PreparedStatement;
/// #     fn statement(&self) -> DeleteStatement {
/// #         parse_statement!("DELETE FROM my_keyspace.my_table WHERE key = ?")
/// #     }
/// #
/// #     fn bind_values<B: Binder>(binder: B, key: &u32, _variables: &()) -> B {
/// #         binder.value(key).value(key)
/// #     }
/// # }
/// #
/// # impl Delete<u32, (), i32> for MyKeyspace {
/// #     type QueryOrPrepared = PreparedStatement;
/// #     fn statement(&self) -> DeleteStatement {
/// #         parse_statement!("DELETE FROM my_table WHERE key = ?")
/// #     }
/// #
/// #     fn bind_values<B: Binder>(binder: B, key: &u32, _variables: &()) -> B {
/// #         binder.value(key)
/// #     }
/// # }
/// use scylla_rs::app::access::*;
///
/// # let keyspace = MyKeyspace::new();
/// # let (my_key, my_val, token_key) = (1, 1.0, 1);
/// let req = keyspace
///     // Creates the `BatchCollector`
///     .batch()
///     .logged()
///     // Add a few pre-defined access queries
///     .delete::<_, _, f32>(&my_key, &())
///     .insert_query(&my_key, &my_val)
///     .update_prepared(&my_key, &(), &my_val)
///     .consistency(Consistency::One)
///     .build()?
///     .compute_token(&token_key);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct BatchCollector<'a, S> {
    builder: BatchBuilder,
    map: HashMap<[u8; 16], ModificationStatement>,
    keyspace: &'a S,
}

impl<'a, S: Keyspace> BatchCollector<'a, S> {
    /// Construct a new batch collector with a keyspace definition
    /// which should implement access and batch traits that will be used
    /// to build this batch. The keyspace will be cloned here and held by
    /// the collector.
    pub fn new(keyspace: &S) -> BatchCollector<S> {
        BatchCollector {
            builder: BatchBuilder::default(),
            map: HashMap::new(),
            keyspace,
        }
    }

    /// Specify the batch type using an enum
    pub fn batch_type(&mut self, batch_type: BatchType) -> &mut Self {
        self.builder.batch_type(batch_type);
        self
    }

    /// Specify the batch type as Logged
    pub fn logged(&mut self) -> &mut Self {
        self.batch_type(BatchType::Logged)
    }

    /// Specify the batch type as Unlogged
    pub fn unlogged(&mut self) -> &mut Self {
        self.batch_type(BatchType::Unlogged)
    }

    /// Specify the batch type as Counter
    pub fn counter(&mut self) -> &mut Self {
        self.batch_type(BatchType::Counter)
    }

    /// Append an unprepared insert query using the statement defined in the `Insert` impl.
    pub fn insert<T, K>(&mut self, key: &K) -> Result<&mut Self, <BatchBuilder as Binder>::Error>
    where
        S: InsertTable<T, K>,
        T: Insert<S, K>,
        K: Bindable + TokenEncoder,
    {
        let statement = T::statement(self.keyspace);
        // this will advance the builder with QueryStatement
        self.builder.statement(&statement.to_string());
        // bind_values of Insert<K>
        T::bind_values(&mut self.builder, key)?;
        Ok(self)
    }

    /// Append a prepared insert query using the statement defined in the `Insert` impl.
    pub fn insert_prepared<T, K>(&mut self, key: &K) -> Result<&mut Self, <BatchBuilder as Binder>::Error>
    where
        S: 'static + InsertTable<T, K> + Debug,
        K: 'static + Bindable + TokenEncoder + Clone + Send + Debug,
        T: Insert<S, K>,
    {
        let statement = T::statement(self.keyspace);
        let id = T::id(self.keyspace);
        self.map.insert(id, statement.into());
        // this will advance the builder with QueryStatement
        self.builder.id(&id);
        // bind_values of Insert<K, V>
        T::bind_values(&mut self.builder, key)?;
        Ok(self)
    }

    /// Append an unprepared update query using the statement defined in the `Update` impl.
    pub fn update<T, K, V>(&mut self, key: &K, values: &V) -> Result<&mut Self, <BatchBuilder as Binder>::Error>
    where
        S: UpdateTable<T, K, V>,
        T: Update<S, K, V>,
        K: TokenEncoder,
    {
        let statement = T::statement(self.keyspace);
        // this will advance the builder with QueryStatement
        self.builder.statement(&statement.to_string());
        // bind_values of Update<K, V>
        T::bind_values(&mut self.builder, key, values)?;
        Ok(self)
    }

    /// Append a prepared update query using the statement defined in the `Update` impl.
    pub fn update_prepared<T, K, V>(
        &mut self,
        key: &K,
        values: &V,
    ) -> Result<&mut Self, <BatchBuilder as Binder>::Error>
    where
        S: 'static + UpdateTable<T, K, V> + Debug,
        T: Update<S, K, V>,
        K: 'static + TokenEncoder + Clone + Send + Debug,
        V: 'static + Clone + Send + Debug,
    {
        let statement = T::statement(self.keyspace);
        let id = T::id(self.keyspace);
        self.map.insert(id, statement.into());
        // this will advance the builder with QueryStatement
        self.builder.id(&id);
        // bind_values of Update<K, V>
        T::bind_values(&mut self.builder, key, values)?;
        Ok(self)
    }

    /// Append an unprepared delete query using the statement defined in the `Delete` impl.
    pub fn delete<T, K>(&mut self, key: &K) -> Result<&mut Self, <BatchBuilder as Binder>::Error>
    where
        S: DeleteTable<T, K>,
        T: Delete<S, K>,
        K: Bindable + TokenEncoder,
    {
        let statement = T::statement(self.keyspace);
        // this will advance the builder with QueryStatement
        self.builder.statement(&statement.to_string());
        // bind_values of Delete<K>
        T::bind_values(&mut self.builder, key)?;
        Ok(self)
    }

    /// Append a prepared delete query using the statement defined in the `Delete` impl.
    pub fn delete_prepared<T, K>(&mut self, key: &K) -> Result<&mut Self, <BatchBuilder as Binder>::Error>
    where
        S: 'static + DeleteTable<T, K> + Debug,
        T: Delete<S, K>,
        K: 'static + Bindable + TokenEncoder + Clone + Send + Debug,
    {
        let statement = T::statement(self.keyspace);
        let id = T::id(self.keyspace);
        self.map.insert(id, statement.into());
        // this will advance the builder with QueryStatement
        self.builder.id(&id);
        // bind_values of Delete<K>
        T::bind_values(&mut self.builder, key)?;
        Ok(self)
    }

    /// Set the consistency for this batch
    pub fn consistency(&mut self, consistency: Consistency) -> &mut Self {
        self.builder.consistency(consistency);
        self
    }

    /// Set the serial consistency for the batch
    pub fn serial_consistency(&mut self, consistency: Consistency) -> &mut Self {
        self.builder.serial_consistency(consistency);
        self
    }
    /// Set the timestamp for the batch
    pub fn timestamp(&mut self, timestamp: i64) -> &mut Self {
        self.builder.timestamp(timestamp);
        self
    }
    /// Build the batch request using the current collector
    pub fn build(&self) -> anyhow::Result<BatchRequest> {
        Ok(BatchRequest {
            token: rand::random(),
            map: self.map.clone(),
            payload: self.builder.build()?.0.into(),
            keyspace: self.keyspace.name().to_owned().into(),
        })
    }
}

/// A Batch request, which can be used to send queries to the Ring.
/// Stores a map of prepared statement IDs that were added to the
/// batch so that the associated statements can be re-prepared if necessary.
#[derive(Clone, Debug)]
pub struct BatchRequest {
    token: i64,
    payload: Vec<u8>,
    map: HashMap<[u8; 16], ModificationStatement>,
    keyspace: Option<String>,
}

impl Request for BatchRequest {
    fn token(&self) -> i64 {
        self.token
    }

    fn statement(&self) -> Statement {
        panic!("Must use `get_statement` on batch requests!")
    }

    fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }

    fn keyspace(&self) -> Option<String> {
        self.keyspace.clone()
    }
}

impl SendRequestExt for BatchRequest {
    type Marker = DecodeVoid;
    type Worker = BasicRetryWorker<Self>;
    const TYPE: RequestType = RequestType::Batch;

    fn worker(self) -> Box<Self::Worker> {
        BasicRetryWorker::new(self)
    }
}

impl BatchRequest {
    /// Compute the murmur3 token from the provided K
    pub fn compute_token<K>(mut self, key: &K) -> Result<Self, K::Error>
    where
        K: TokenEncoder,
    {
        self.token = key.token()?;
        Ok(self)
    }

    /// Clone the cql map
    pub fn clone_map(&self) -> HashMap<[u8; 16], ModificationStatement> {
        self.map.clone()
    }

    /// Take the cql map, leaving an empty map in the request
    pub fn take_map(&mut self) -> HashMap<[u8; 16], ModificationStatement> {
        std::mem::take(&mut self.map)
    }

    /// Get a statement given an id from the request's map
    pub fn get_statement(&self, id: &[u8; 16]) -> Option<&ModificationStatement> {
        self.map.get(id)
    }

    /// Get a basic worker for this request
    pub fn worker(self) -> Box<BasicRetryWorker<Self>> {
        BasicRetryWorker::new(self)
    }
}
