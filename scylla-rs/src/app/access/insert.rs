// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

/// Insert query trait which creates an `InsertRequest`
/// that can be sent to the `Ring`.
///
/// ## Example
/// ```
/// use scylla_rs::app::access::*;
/// #[derive(Clone, Debug)]
/// struct MyKeyspace {
///     pub name: String,
/// }
/// # impl MyKeyspace {
/// #     pub fn new(name: &str) -> Self {
/// #         Self {
/// #             name: name.to_string().into(),
/// #         }
/// #     }
/// # }
/// impl Keyspace for MyKeyspace {
///     fn name(&self) -> String {
///         self.name.clone()
///     }
///
///     fn opts(&self) -> KeyspaceOpts {
///         KeyspaceOptsBuilder::default()
///             .replication(Replication::network_topology(maplit::btreemap! {
///                 "datacenter1" => 1,
///             }))
///             .durable_writes(true)
///             .build()
///             .unwrap()
///     }
/// }
/// # type MyKeyType = i32;
/// # #[derive(Default)]
/// struct MyValueType {
///     value1: f32,
///     value2: f32,
/// }
/// impl<B: Binder> Bindable<B> for MyValueType {
///     fn bind(&self, binder: B) -> B {
///         binder.bind(&self.value1).bind(&self.value2)
///     }
/// }
/// impl Insert<MyKeyType, MyValueType> for MyKeyspace {
///     type QueryOrPrepared = PreparedStatement;
///     fn statement(&self) -> InsertStatement {
///         parse_statement!("INSERT INTO my_table (key, val1, val2) VALUES (?,?,?)")
///     }
///
///     fn bind_values<B: Binder>(builder: B, key: &MyKeyType, values: &MyValueType) -> B {
///         builder.value(key).bind(values)
///     }
/// }
///
/// # let (my_key, my_val) = (1, MyValueType::default());
/// let request = MyKeyspace::new("my_keyspace")
///     .insert_prepared(&my_key, &my_val)
///     .consistency(Consistency::One)
///     .build()?;
/// let worker = request.worker();
/// # Ok::<(), anyhow::Error>(())
/// ```
pub trait Insert<K: Bindable>: Table {
    /// Create your insert statement here.
    fn statement(keyspace: &dyn Keyspace) -> InsertStatement;

    /// Bind the cql values to the builder
    fn bind_values<B: Binder>(binder: B, key: &K) -> Result<B, B::Error> {
        binder.bind(key)
    }
}

impl<T: Table + Bindable> Insert<T> for T {
    fn statement(keyspace: &dyn Keyspace) -> InsertStatement {
        let names = T::COLS.iter().map(|&(c, _)| Name::from(c)).collect::<Vec<_>>();
        let values = T::COLS
            .iter()
            .map(|_| Term::from(BindMarker::Anonymous))
            .collect::<Vec<_>>();
        parse_statement!(
            "INSERT INTO #.# (#) VALUES (#)",
            keyspace.name(),
            T::NAME,
            names,
            values
        )
    }
}

/// Specifies helper functions for creating static insert requests from a keyspace with a `Delete<K, V>` definition
pub trait GetStaticInsertRequest<K: Bindable>: Table {
    /// Create a static insert request from a keyspace with a `Insert<K, V>` definition. Will use the default `type
    /// QueryOrPrepared` from the trait definition.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// #[derive(Clone, Debug)]
    /// struct MyKeyspace {
    ///     pub name: String,
    /// }
    /// # impl MyKeyspace {
    /// #     pub fn new(name: &str) -> Self {
    /// #         Self {
    /// #             name: name.to_string().into(),
    /// #         }
    /// #     }
    /// # }
    /// impl Keyspace for MyKeyspace {
    ///     fn name(&self) -> String {
    ///         self.name.clone()
    ///     }
    ///
    ///     fn opts(&self) -> KeyspaceOpts {
    ///         KeyspaceOptsBuilder::default()
    ///             .replication(Replication::network_topology(maplit::btreemap! {
    ///                 "datacenter1" => 1,
    ///             }))
    ///             .durable_writes(true)
    ///             .build()
    ///             .unwrap()
    ///     }
    /// }
    /// # type MyKeyType = i32;
    /// # type MyVarType = String;
    /// # #[derive(Default)]
    /// struct MyValueType {
    ///     value1: f32,
    ///     value2: f32,
    /// }
    /// impl Insert<MyKeyType, MyVarType, MyValueType> for MyKeyspace {
    ///     type QueryOrPrepared = PreparedStatement;
    ///     fn statement(&self) -> InsertStatement {
    ///         parse_statement!("UPDATE my_table SET val1 = ?, val2 = ? WHERE key = ? AND var = ?")
    ///     }
    ///
    ///     fn bind_values<B: Binder>(builder: B, key: &MyKeyType, variables: &MyVarType, value: &MyValueType) -> B {
    ///         builder
    ///             .value(&value.value1)
    ///             .value(&value.value2)
    ///             .value(key)
    ///             .value(variables)
    ///     }
    /// }
    /// # let (my_key, my_var, my_val) = (1, MyVarType::default(), MyValueType::default());
    /// MyKeyspace::new("my_keyspace")
    ///     .insert(&my_key, &my_var, &my_val)
    ///     .consistency(Consistency::One)
    ///     .build()?
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn insert(
        keyspace: &dyn Keyspace,
        key: &K,
    ) -> Result<InsertBuilder<StaticRequest, QueryFrameBuilder>, <QueryFrameBuilder as Binder>::Error>
    where
        Self: Insert<K>,
        K: Bindable,
    {
        let statement = Self::statement(keyspace);
        let keyspace = statement.get_keyspace();
        let token_indexes = statement.token_indexes::<Self>();
        let statement = statement.to_string();
        let mut builder = QueryFrameBuilder::default()
            .consistency(Consistency::Quorum)
            .statement(statement.clone());
        builder = Self::bind_values(builder, key)?;
        Ok(InsertBuilder {
            token_indexes,
            builder,
            keyspace,
            statement,
            _marker: PhantomData,
        })
    }

    /// Create a static insert prepared request from a keyspace with a `Insert<K, V>` definition.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// #[derive(Clone, Debug)]
    /// struct MyKeyspace {
    ///     pub name: String,
    /// }
    /// # impl MyKeyspace {
    /// #     pub fn new(name: &str) -> Self {
    /// #         Self {
    /// #             name: name.to_string().into(),
    /// #         }
    /// #     }
    /// # }
    /// impl Keyspace for MyKeyspace {
    ///     fn name(&self) -> String {
    ///         self.name.clone()
    ///     }
    ///
    ///     fn opts(&self) -> KeyspaceOpts {
    ///         KeyspaceOptsBuilder::default()
    ///             .replication(Replication::network_topology(maplit::btreemap! {
    ///                 "datacenter1" => 1,
    ///             }))
    ///             .durable_writes(true)
    ///             .build()
    ///             .unwrap()
    ///     }
    /// }
    /// # type MyKeyType = i32;
    /// # type MyVarType = String;
    /// # #[derive(Default)]
    /// struct MyValueType {
    ///     value1: f32,
    ///     value2: f32,
    /// }
    /// impl Insert<MyKeyType, MyVarType, MyValueType> for MyKeyspace {
    ///     type QueryOrPrepared = PreparedStatement;
    ///     fn statement(&self) -> InsertStatement {
    ///         parse_statement!("UPDATE my_table SET val1 = ?, val2 = ? WHERE key = ? AND var = ?")
    ///     }
    ///
    ///     fn bind_values<B: Binder>(builder: B, key: &MyKeyType, variables: &MyVarType, value: &MyValueType) -> B {
    ///         builder
    ///             .value(&value.value1)
    ///             .value(&value.value2)
    ///             .value(key)
    ///             .value(variables)
    ///     }
    /// }
    /// # let (my_key, my_var, my_val) = (1, MyVarType::default(), MyValueType::default());
    /// MyKeyspace::new("my_keyspace")
    ///     .insert_prepared(&my_key, &my_var, &my_val)
    ///     .consistency(Consistency::One)
    ///     .build()?
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn insert_prepared(
        keyspace: &dyn Keyspace,
        key: &K,
    ) -> Result<InsertBuilder<StaticRequest, ExecuteFrameBuilder>, <ExecuteFrameBuilder as Binder>::Error>
    where
        Self: Insert<K>,
        K: Bindable,
    {
        let statement = Self::statement(keyspace);
        let keyspace = statement.get_keyspace();
        let token_indexes = statement.token_indexes::<Self>();
        let statement = statement.to_string();
        let mut builder = ExecuteFrameBuilder::default()
            .consistency(Consistency::Quorum)
            .id(statement.id());
        builder = Self::bind_values(builder, key)?;
        Ok(InsertBuilder {
            token_indexes,
            builder,
            keyspace,
            statement,
            _marker: PhantomData,
        })
    }
}
impl<T: Table, K: Bindable> GetStaticInsertRequest<K> for T {}

/// Specifies helper functions for creating dynamic insert requests from anything that can be interpreted as a statement

pub trait AsDynamicInsertRequest: Sized {
    /// Create a dynamic insert request from a statement and variables. Can be specified as either
    /// a query or prepared statement.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// parse_statement!("UPDATE my_keyspace.my_table SET val1 = ?, val2 = ? WHERE key = ? AND var = ?")
    ///     .as_insert(&[&3], &[&4.0, &5.0], StatementType::Query)
    ///     .consistency(Consistency::One)
    ///     .build()?
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn query(&self) -> InsertBuilder<DynamicRequest, QueryFrameBuilder>;

    /// Create a dynamic insert prepared request from a statement and variables.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// parse_statement!("UPDATE my_keyspace.my_table SET val1 = ?, val2 = ? WHERE key = ? AND var = ?")
    ///     .as_insert_prepared(&[&3], &[&4.0, &5.0])
    ///     .consistency(Consistency::One)
    ///     .build()?
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn query_prepared(&self) -> InsertBuilder<DynamicRequest, ExecuteFrameBuilder>;
}
impl AsDynamicInsertRequest for InsertStatement {
    fn query(&self) -> InsertBuilder<DynamicRequest, QueryFrameBuilder> {
        let keyspace = self.get_keyspace();
        let statement = self.to_string();
        InsertBuilder {
            builder: QueryFrameBuilder::default()
                .consistency(Consistency::Quorum)
                .statement(statement.clone()),
            keyspace,
            statement,
            token_indexes: Default::default(),
            _marker: PhantomData,
        }
    }

    fn query_prepared(&self) -> InsertBuilder<DynamicRequest, ExecuteFrameBuilder> {
        let keyspace = self.get_keyspace();
        let statement = self.to_string();
        InsertBuilder {
            builder: ExecuteFrameBuilder::default()
                .consistency(Consistency::Quorum)
                .id(statement.id()),
            keyspace,
            statement,
            token_indexes: Default::default(),
            _marker: PhantomData,
        }
    }
}

pub struct InsertBuilder<R, B> {
    keyspace: Option<String>,
    statement: String,
    builder: B,
    token_indexes: Vec<usize>,
    _marker: PhantomData<fn(R) -> R>,
}

impl<R> InsertBuilder<R, QueryFrameBuilder> {
    pub fn consistency(mut self, consistency: Consistency) -> Self {
        self.builder = self.builder.consistency(consistency);
        self
    }

    pub fn timestamp(mut self, timestamp: i64) -> Self {
        self.builder = self.builder.timestamp(timestamp);
        self
    }

    pub fn build(self) -> anyhow::Result<InsertRequest> {
        let frame = self.builder.build()?;
        let mut token = TokenEncodeChain::default();
        for idx in self.token_indexes {
            if frame.values.len() <= idx {
                anyhow::bail!("No value bound at index {}", idx);
            }
            token.append(&frame.values[idx]);
        }
        Ok(CommonRequest {
            token: token.finish(),
            statement: frame.statement().clone(),
            payload: RequestFrame::from(frame).build_payload(),
            keyspace: self.keyspace,
        }
        .into())
    }
}

impl<R> InsertBuilder<R, ExecuteFrameBuilder> {
    pub fn consistency(mut self, consistency: Consistency) -> Self {
        self.builder = self.builder.consistency(consistency);
        self
    }

    pub fn timestamp(mut self, timestamp: i64) -> Self {
        self.builder = self.builder.timestamp(timestamp);
        self
    }

    pub fn build(self) -> anyhow::Result<InsertRequest> {
        let frame = self.builder.build()?;
        let mut token = TokenEncodeChain::default();
        for idx in self.token_indexes {
            if frame.values.len() <= idx {
                anyhow::bail!("No value bound at index {}", idx);
            }
            token.append(&frame.values[idx]);
        }
        Ok(CommonRequest {
            token: token.finish(),
            statement: self.statement,
            payload: RequestFrame::from(frame).build_payload(),
            keyspace: self.keyspace,
        }
        .into())
    }
}

impl<B: Binder> InsertBuilder<DynamicRequest, B> {
    pub fn bind<V: Bindable>(mut self, value: &V) -> Result<Self, B::Error> {
        self.builder = self.builder.bind(value)?;
        Ok(self)
    }
}

impl<R> From<PreparedQuery> for InsertBuilder<R, ExecuteFrameBuilder> {
    fn from(res: PreparedQuery) -> Self {
        Self {
            keyspace: res.keyspace,
            statement: res.statement,
            builder: ExecuteFrameBuilder::default()
                .id(res.result.id)
                .consistency(Consistency::Quorum),
            token_indexes: res.result.metadata().pk_indexes().iter().map(|v| *v as usize).collect(),
            _marker: PhantomData,
        }
    }
}

impl<R, B: std::fmt::Debug> std::fmt::Debug for InsertBuilder<R, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InsertBuilder")
            .field("keyspace", &self.keyspace)
            .field("statement", &self.statement)
            .field("builder", &self.builder)
            .field("token_indexes", &self.token_indexes)
            .finish()
    }
}

impl<R, B: Clone> Clone for InsertBuilder<R, B> {
    fn clone(&self) -> Self {
        Self {
            keyspace: self.keyspace.clone(),
            statement: self.statement.clone(),
            builder: self.builder.clone(),
            token_indexes: self.token_indexes.clone(),
            _marker: PhantomData,
        }
    }
}

impl<R, B> Request for InsertBuilder<R, B> {
    fn token(&self) -> i64 {
        todo!()
    }

    fn statement(&self) -> &String {
        todo!()
    }

    fn payload(&self) -> Vec<u8> {
        todo!()
    }

    fn keyspace(&self) -> Option<&String> {
        todo!()
    }
}

impl<R> TryInto<InsertRequest> for InsertBuilder<R, QueryFrameBuilder> {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<InsertRequest, Self::Error> {
        self.build()
    }
}
impl<R> TryInto<InsertRequest> for InsertBuilder<R, ExecuteFrameBuilder> {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<InsertRequest, Self::Error> {
        self.build()
    }
}
impl<R> SendAsRequestExt<InsertRequest> for InsertBuilder<R, QueryFrameBuilder> {}
impl<R> SendAsRequestExt<InsertRequest> for InsertBuilder<R, ExecuteFrameBuilder> {}

/// A request to insert a record which can be sent to the ring
#[derive(Debug, Clone)]
pub struct InsertRequest(CommonRequest);

impl From<CommonRequest> for InsertRequest {
    fn from(req: CommonRequest) -> Self {
        InsertRequest(req)
    }
}

impl Deref for InsertRequest {
    type Target = CommonRequest;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for InsertRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Request for InsertRequest {
    fn token(&self) -> i64 {
        self.0.token()
    }

    fn statement(&self) -> &String {
        self.0.statement()
    }

    fn payload(&self) -> Vec<u8> {
        self.0.payload()
    }
    fn keyspace(&self) -> Option<&String> {
        self.0.keyspace()
    }
}

impl SendRequestExt for InsertRequest {
    type Marker = DecodeVoid;
    type Worker = BasicRetryWorker<Self>;
    const TYPE: RequestType = RequestType::Insert;

    fn worker(self) -> Box<Self::Worker> {
        BasicRetryWorker::new(self)
    }

    fn marker(&self) -> Self::Marker {
        DecodeVoid
    }
}
