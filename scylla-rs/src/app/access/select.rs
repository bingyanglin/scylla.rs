// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

/// Select query trait which creates a `SelectRequest`
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
/// # type MyVarType = String;
/// # type MyValueType = f32;
/// impl Select<MyKeyType, MyVarType, MyValueType> for MyKeyspace {
///     type QueryOrPrepared = PreparedStatement;
///     fn statement(&self) -> SelectStatement {
///         parse_statement!("SELECT val FROM my_table where key = ? AND var = ?")
///     }
///     fn bind_values<B: Binder>(builder: B, key: &MyKeyType, variables: &MyVarType) -> B {
///         builder.bind(key).bind(variables)
///     }
/// }
/// # let (my_key, my_var) = (1, MyVarType::default());
/// let request = MyKeyspace::new("my_keyspace")
///     .select::<MyValueType>(&my_key, &my_var)
///     .consistency(Consistency::One)
///     .build()?;
/// let worker = request.worker();
/// # Ok::<(), anyhow::Error>(())
/// ```
pub trait Select<K: Bindable, O: RowsDecoder>: Table {
    /// Create your select statement here.
    fn statement(keyspace: &dyn Keyspace) -> SelectStatement;

    /// Bind the cql values to the builder
    fn bind_values<B: Binder>(binder: B, key: &K) -> Result<B, B::Error> {
        binder.bind(key)
    }
}

impl<T: Table + RowsDecoder> Select<T::PrimaryKey, T> for T
where
    T::PrimaryKey: Bindable,
{
    fn statement(keyspace: &dyn Keyspace) -> SelectStatement {
        let where_clause = T::PARTITION_KEY
            .iter()
            .chain(T::CLUSTERING_COLS.iter().map(|(c, _)| c))
            .map(|&c| Relation::normal(c, Operator::Equal, BindMarker::Anonymous))
            .collect::<Vec<_>>();
        parse_statement!("SELECT * FROM #.# #", keyspace.name(), T::NAME, where_clause)
    }
}

/// Specifies helper functions for creating static delete requests from a keyspace with a `Delete<K, V>` definition

pub trait GetStaticSelectRequest<K: Bindable>: Table {
    /// Create a static select request from a keyspace with a `Select<K, V>` definition. Will use the default `type
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
    /// # type MyValueType = f32;
    /// impl Select<MyKeyType, MyVarType, MyValueType> for MyKeyspace {
    ///     type QueryOrPrepared = PreparedStatement;
    ///     fn statement(&self) -> SelectStatement {
    ///         parse_statement!("SELECT val FROM my_table where key = ? AND var = ?")
    ///     }
    ///     fn bind_values<B: Binder>(builder: B, key: &MyKeyType, variables: &MyVarType) -> B {
    ///         builder.bind(key).bind(variables)
    ///     }
    /// }
    /// # let (my_key, my_var) = (1, MyVarType::default());
    /// let res: Option<MyValueType> = MyKeyspace::new("my_keyspace")
    ///     .select::<MyValueType>(&my_key, &my_var)
    ///     .consistency(Consistency::One)
    ///     .build()?
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn select<O>(
        keyspace: &dyn Keyspace,
        key: &K,
    ) -> Result<SelectBuilder<StaticRequest, O, QueryFrameBuilder>, <QueryFrameBuilder as Binder>::Error>
    where
        Self: Select<K, O>,
        O: RowsDecoder,
    {
        let statement = Self::statement(keyspace);
        let keyspace = statement.get_keyspace();
        let token_indexes = statement.token_indexes::<Self>();
        let statement = statement.to_string();
        let mut builder = QueryFrameBuilder::default()
            .consistency(Consistency::One)
            .statement(statement.clone());
        builder = Self::bind_values(builder, key)?;
        Ok(SelectBuilder {
            token_indexes,
            builder,
            statement,
            keyspace,
            _marker: PhantomData,
        })
    }

    /// Create a static select prepared request from a keyspace with a `Select<K, V>` definition.
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
    /// # type MyValueType = f32;
    /// impl Select<MyKeyType, MyVarType, MyValueType> for MyKeyspace {
    ///     type QueryOrPrepared = PreparedStatement;
    ///     fn statement(&self) -> SelectStatement {
    ///         parse_statement!("SELECT val FROM my_table where key = ? AND var = ?")
    ///     }
    ///     fn bind_values<B: Binder>(builder: B, key: &MyKeyType, variables: &MyVarType) -> B {
    ///         builder.bind(key).bind(variables)
    ///     }
    /// }
    /// # let (my_key, my_var) = (1, MyVarType::default());
    /// let res: Option<MyValueType> = MyKeyspace::new("my_keyspace")
    ///     .select_prepared::<MyValueType>(&my_key, &my_var)
    ///     .consistency(Consistency::One)
    ///     .build()?
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn select_prepared<O>(
        keyspace: &dyn Keyspace,
        key: &K,
    ) -> Result<SelectBuilder<StaticRequest, O, ExecuteFrameBuilder>, <ExecuteFrameBuilder as Binder>::Error>
    where
        Self: Select<K, O>,
        O: RowsDecoder,
    {
        let statement = Self::statement(keyspace);
        let keyspace = statement.get_keyspace();
        let token_indexes = statement.token_indexes::<Self>();
        let statement = statement.to_string();
        let mut builder = ExecuteFrameBuilder::default()
            .consistency(Consistency::One)
            .id(statement.id());
        builder = Self::bind_values(builder, key)?;
        Ok(SelectBuilder {
            token_indexes,
            builder,
            statement,
            keyspace,
            _marker: PhantomData,
        })
    }
}
impl<T: Table, K: Bindable> GetStaticSelectRequest<K> for T {}

/// Specifies helper functions for creating dynamic select requests from anything that can be interpreted as a statement

pub trait AsDynamicSelectRequest: Sized {
    /// Create a dynamic select query request from a statement and variables.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// let res: Option<f32> = parse_statement!("SELECT val FROM my_keyspace.my_table where key = ? AND var = ?")
    ///     .as_select_query::<f32>(&[&3], &[&"hello"])
    ///     .consistency(Consistency::One)
    ///     .build()?
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn query<O: RowsDecoder>(&self) -> SelectBuilder<DynamicRequest, O, QueryFrameBuilder>;

    /// Create a dynamic select prepared request from a statement and variables.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// let res: Option<f32> = parse_statement!("SELECT val FROM my_keyspace.my_table where key = ? AND var = ?")
    ///     .as_select_prepared::<f32>(&[&3], &[&"hello"])
    ///     .consistency(Consistency::One)
    ///     .build()?
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn query_prepared<O: RowsDecoder>(&self) -> SelectBuilder<DynamicRequest, O, ExecuteFrameBuilder>;
}
impl AsDynamicSelectRequest for SelectStatement {
    fn query<O: RowsDecoder>(&self) -> SelectBuilder<DynamicRequest, O, QueryFrameBuilder> {
        let keyspace = self.get_keyspace();
        let statement = self.to_string();
        SelectBuilder {
            builder: QueryFrameBuilder::default()
                .consistency(Consistency::One)
                .statement(statement.clone()),
            statement,
            keyspace,
            token_indexes: Default::default(),
            _marker: PhantomData,
        }
    }

    fn query_prepared<O: RowsDecoder>(&self) -> SelectBuilder<DynamicRequest, O, ExecuteFrameBuilder> {
        let keyspace = self.get_keyspace();
        let statement = self.to_string();
        SelectBuilder {
            builder: ExecuteFrameBuilder::default()
                .consistency(Consistency::One)
                .id(statement.id()),
            statement,
            keyspace,
            token_indexes: Default::default(),
            _marker: PhantomData,
        }
    }
}

pub struct SelectBuilder<R, O: RowsDecoder, B> {
    keyspace: Option<String>,
    statement: String,
    builder: B,
    token_indexes: Vec<usize>,
    _marker: PhantomData<fn(R, O, B) -> (R, O, B)>,
}

impl<R, O: RowsDecoder> SelectBuilder<R, O, QueryFrameBuilder> {
    pub fn consistency(mut self, consistency: Consistency) -> Self {
        self.builder = self.builder.consistency(consistency);
        self
    }

    pub fn page_size(mut self, page_size: i32) -> Self {
        self.builder = self.builder.page_size(page_size);
        self
    }
    /// Set the paging state.
    pub fn paging_state(mut self, paging_state: Vec<u8>) -> Self {
        self.builder = self.builder.paging_state(paging_state);
        self
    }
    pub fn timestamp(mut self, timestamp: i64) -> Self {
        self.builder = self.builder.timestamp(timestamp);
        self
    }

    pub fn build(self) -> anyhow::Result<QuerySelectRequest<O>> {
        let frame = self.builder.build()?;
        let mut token = TokenEncodeChain::default();
        for idx in self.token_indexes {
            if frame.values.len() <= idx {
                anyhow::bail!("No value bound at index {}", idx);
            }
            token.append(&frame.values[idx]);
        }
        Ok(QuerySelectRequest::new(frame, token.finish(), self.keyspace))
    }
}

impl<R, O: RowsDecoder> SelectBuilder<R, O, ExecuteFrameBuilder> {
    pub fn consistency(mut self, consistency: Consistency) -> Self {
        self.builder = self.builder.consistency(consistency);
        self
    }

    pub fn page_size(mut self, page_size: i32) -> Self {
        self.builder = self.builder.page_size(page_size);
        self
    }
    /// Set the paging state.
    pub fn paging_state(mut self, paging_state: Vec<u8>) -> Self {
        self.builder = self.builder.paging_state(paging_state);
        self
    }
    pub fn timestamp(mut self, timestamp: i64) -> Self {
        self.builder = self.builder.timestamp(timestamp);
        self
    }

    pub fn build(self) -> anyhow::Result<ExecuteSelectRequest<O>> {
        let frame = self.builder.build()?;
        let mut token = TokenEncodeChain::default();
        for idx in self.token_indexes {
            if frame.values.len() <= idx {
                anyhow::bail!("No value bound at index {}", idx);
            }
            token.append(&frame.values[idx]);
        }
        Ok(ExecuteSelectRequest::new(
            frame,
            token.finish(),
            self.keyspace,
            self.statement,
        ))
    }
}

impl<O: RowsDecoder, B: Binder> SelectBuilder<DynamicRequest, O, B> {
    pub fn bind<V: Bindable>(mut self, value: &V) -> Result<Self, B::Error> {
        self.builder = self.builder.bind(value)?;
        Ok(self)
    }
}

impl<R, O: RowsDecoder> From<PreparedQuery> for SelectBuilder<R, O, ExecuteFrameBuilder> {
    fn from(res: PreparedQuery) -> Self {
        Self {
            keyspace: res.keyspace,
            statement: res.statement,
            builder: ExecuteFrameBuilder::default()
                .id(res.result.id)
                .consistency(Consistency::One),
            token_indexes: res.result.metadata().pk_indexes().iter().map(|v| *v as usize).collect(),
            _marker: PhantomData,
        }
    }
}

impl<R, O: RowsDecoder, B: std::fmt::Debug> std::fmt::Debug for SelectBuilder<R, O, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectBuilder")
            .field("keyspace", &self.keyspace)
            .field("statement", &self.statement)
            .field("builder", &self.builder)
            .field("token_indexes", &self.token_indexes)
            .finish()
    }
}

impl<R, O: RowsDecoder, B: Clone> Clone for SelectBuilder<R, O, B> {
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

impl<R, O: RowsDecoder> TryInto<QuerySelectRequest<O>> for SelectBuilder<R, O, QueryFrameBuilder> {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<QuerySelectRequest<O>, Self::Error> {
        self.build()
    }
}
impl<R, O: RowsDecoder> TryInto<ExecuteSelectRequest<O>> for SelectBuilder<R, O, ExecuteFrameBuilder> {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<ExecuteSelectRequest<O>, Self::Error> {
        self.build()
    }
}
impl<R, O: 'static + RowsDecoder + Send + Sync> SendAsRequestExt<QuerySelectRequest<O>>
    for SelectBuilder<R, O, QueryFrameBuilder>
{
}
impl<R, O: 'static + RowsDecoder + Send + Sync> SendAsRequestExt<ExecuteSelectRequest<O>>
    for SelectBuilder<R, O, ExecuteFrameBuilder>
{
}

pub struct QuerySelectRequest<O> {
    frame: QueryFrame,
    token: i64,
    keyspace: Option<String>,
    _marker: PhantomData<fn(O) -> O>,
}

impl<O> QuerySelectRequest<O> {
    pub fn new(frame: QueryFrame, token: i64, keyspace: Option<String>) -> Self {
        Self {
            frame,
            token,
            keyspace,
            _marker: PhantomData,
        }
    }
}

impl<O> RequestFrameExt for QuerySelectRequest<O> {
    type Frame = QueryFrame;

    fn frame(&self) -> &Self::Frame {
        &self.frame
    }

    fn into_frame(self) -> RequestFrame {
        self.frame.into()
    }
}

impl<O> ShardAwareExt for QuerySelectRequest<O> {
    fn token(&self) -> i64 {
        self.token
    }

    fn keyspace(&self) -> Option<&String> {
        self.keyspace.as_ref()
    }
}

impl<O> Deref for QuerySelectRequest<O> {
    type Target = QueryFrame;

    fn deref(&self) -> &Self::Target {
        &self.frame
    }
}

impl<O> DerefMut for QuerySelectRequest<O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.frame
    }
}

impl<O: 'static + Send + Sync + RowsDecoder> SendRequestExt for QuerySelectRequest<O> {
    type Worker = BasicRetryWorker<Self>;
    type Marker = DecodeRows<O>;
    const TYPE: RequestType = RequestType::Select;

    fn marker(&self) -> Self::Marker {
        DecodeRows::<O>::new()
    }

    fn event(self) -> (Self::Worker, RequestFrame) {
        (BasicRetryWorker::new(self.clone()), self.into_frame())
    }

    fn worker(self) -> Self::Worker {
        BasicRetryWorker::new(self)
    }
}

impl<O> Debug for QuerySelectRequest<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectRequest")
            .field("frame", &self.frame)
            .field("token", &self.token)
            .field("keyspace", &self.keyspace)
            .finish()
    }
}

impl<O> Clone for QuerySelectRequest<O> {
    fn clone(&self) -> Self {
        Self {
            frame: self.frame.clone(),
            token: self.token,
            keyspace: self.keyspace.clone(),
            _marker: PhantomData,
        }
    }
}

impl<O> QuerySelectRequest<O> {
    /// Return DecodeResult marker type, useful in case the worker struct wants to hold the
    /// decoder in order to decode the response inside handle_response method.
    pub fn result_decoder(&self) -> DecodeResult<DecodeRows<O>> {
        DecodeResult::select()
    }
}

/// A request to delete a record which can be sent to the ring
pub struct ExecuteSelectRequest<O> {
    frame: ExecuteFrame,
    token: i64,
    keyspace: Option<String>,
    statement: String,
    _marker: PhantomData<fn(O) -> O>,
}

impl<O> ExecuteSelectRequest<O> {
    pub fn new(frame: ExecuteFrame, token: i64, keyspace: Option<String>, statement: String) -> Self {
        Self {
            frame,
            token,
            keyspace,
            statement,
            _marker: PhantomData,
        }
    }
}

impl<O> RequestFrameExt for ExecuteSelectRequest<O> {
    type Frame = ExecuteFrame;

    fn frame(&self) -> &Self::Frame {
        &self.frame
    }

    fn into_frame(self) -> RequestFrame {
        self.frame.into()
    }
}

impl<O> ShardAwareExt for ExecuteSelectRequest<O> {
    fn token(&self) -> i64 {
        self.token
    }

    fn keyspace(&self) -> Option<&String> {
        self.keyspace.as_ref()
    }
}

impl<O: 'static + Send + Sync + RowsDecoder> ReprepareExt for ExecuteSelectRequest<O> {
    type OutRequest = QuerySelectRequest<O>;
    fn convert(self) -> Self::OutRequest {
        QuerySelectRequest {
            token: self.token,
            frame: QueryFrame::from_execute(self.frame, self.statement),
            keyspace: self.keyspace,
            _marker: PhantomData,
        }
    }

    fn statement(&self) -> &String {
        &self.statement
    }
}

impl<O> Deref for ExecuteSelectRequest<O> {
    type Target = ExecuteFrame;

    fn deref(&self) -> &Self::Target {
        &self.frame
    }
}

impl<O> DerefMut for ExecuteSelectRequest<O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.frame
    }
}

impl<O: 'static + Send + Sync + RowsDecoder> SendRequestExt for ExecuteSelectRequest<O> {
    type Worker = BasicRetryWorker<Self>;
    type Marker = DecodeRows<O>;
    const TYPE: RequestType = RequestType::Select;

    fn marker(&self) -> Self::Marker {
        DecodeRows::<O>::new()
    }

    fn event(self) -> (Self::Worker, RequestFrame) {
        (BasicRetryWorker::new(self.clone()), self.into_frame())
    }

    fn worker(self) -> Self::Worker {
        BasicRetryWorker::new(self)
    }
}

impl<O> Debug for ExecuteSelectRequest<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectRequest")
            .field("frame", &self.frame)
            .field("token", &self.token)
            .field("keyspace", &self.keyspace)
            .field("statement", &self.statement)
            .finish()
    }
}

impl<O> Clone for ExecuteSelectRequest<O> {
    fn clone(&self) -> Self {
        Self {
            frame: self.frame.clone(),
            token: self.token,
            keyspace: self.keyspace.clone(),
            statement: self.statement.clone(),
            _marker: PhantomData,
        }
    }
}

/// A request to select a record which can be sent to the ring
impl<O> ExecuteSelectRequest<O> {
    /// Return DecodeResult marker type, useful in case the worker struct wants to hold the
    /// decoder in order to decode the response inside handle_response method.
    pub fn result_decoder(&self) -> DecodeResult<DecodeRows<O>> {
        DecodeResult::select()
    }
}
