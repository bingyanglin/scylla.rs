// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::prelude::{
    Prepare,
    PrepareWorker,
};

/// Specifies helper functions for creating static prepare requests from a keyspace with any access trait definition

pub trait GetStaticPrepareRequest: Keyspace {
    /// Create a static prepare request from a keyspace with a `Select<K, V>` definition.
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
    /// }
    /// # type MyKeyType = i32;
    /// # type MyValueType = f32;
    /// impl Select<MyKeyType, MyValueType> for MyKeyspace {
    ///     type QueryOrPrepared = PreparedStatement;
    ///     fn statement(&self) -> Cow<'static, str> {
    ///         format!("SELECT val FROM {}.table where key = ?", self.name()).into()
    ///     }
    ///     fn bind_values<T: Values>(builder: T, key: &MyKeyType) -> T::Return {
    ///         builder.bind(key)
    ///     }
    /// }
    /// # let my_key = 1;
    /// MyKeyspace::new("my_keyspace")
    ///     .prepare_select::<MyKeyType, MyValueType>()
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn prepare_select<K, V>(&self) -> PrepareRequest
    where
        Self: Select<K, V>,
    {
        let statement = self.statement();
        PrepareRequest::new(statement)
    }

    /// Create a static prepare request from a keyspace with a `Insert<K, V>` definition.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// #[derive(Clone, Debug)]
    /// struct MyKeyspace {
    ///     pub name: String
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
    /// }
    /// # type MyKeyType = i32;
    /// # #[derive(Default)]
    /// struct MyValueType {
    ///     value1: f32,
    ///     value2: f32,
    /// }
    /// impl Insert<MyKeyType, MyValueType> for MyKeyspace {
    ///     type QueryOrPrepared = PreparedStatement;
    ///     fn statement(&self) -> Cow<'static, str> {
    ///         format!("INSERT INTO {}.table (key, val1, val2) VALUES (?,?,?)", self.name()).into()
    ///     }
    ///
    ///     fn bind_values<T: Values>(builder: T, key: &MyKeyType, value: &MyValueType) -> T::Return {
    ///         builder.value(key).value(&value.value1).value(&value.value2)
    ///     }
    /// }

    /// # let (my_key, my_val) = (1, MyValueType::default());
    /// MyKeyspace::new("my_keyspace")
    ///     .prepare_insert::<MyKeyType, MyValueType>()
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn prepare_insert<K, V>(&self) -> PrepareRequest
    where
        Self: Insert<K, V>,
    {
        let statement = self.statement();
        PrepareRequest::new(statement)
    }

    /// Create a static prepare request from a keyspace with a `Update<K, V>` definition.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// #[derive(Clone, Debug)]
    /// struct MyKeyspace {
    ///     pub name: String
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
    /// }
    /// # type MyKeyType = i32;
    /// # #[derive(Default)]
    /// struct MyValueType {
    ///     value1: f32,
    ///     value2: f32,
    /// }
    /// impl Update<MyKeyType, MyValueType> for MyKeyspace {
    ///     type QueryOrPrepared = PreparedStatement;
    ///     fn statement(&self) -> Cow<'static, str> {
    ///         format!("UPDATE {}.table SET val1 = ?, val2 = ? WHERE key = ?", self.name()).into()
    ///     }
    ///
    ///     fn bind_values<T: Values>(builder: T, key: &MyKeyType, value: &MyValueType) -> T::Return {
    ///         builder.bind(&value.value1).value(&value.value2).value(key)
    ///     }
    /// }

    /// # let (my_key, my_val) = (1, MyValueType::default());
    /// MyKeyspace::new("my_keyspace")
    ///     .prepare_update::<MyKeyType, MyValueType>()
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn prepare_update<K, V>(&self) -> PrepareRequest
    where
        Self: Update<K, V>,
    {
        let statement = self.statement();
        PrepareRequest::new(statement)
    }

    /// Create a static prepare request from a keyspace with a `Delete<K, V>` definition.
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
    /// }
    /// # type MyKeyType = i32;
    /// # type MyValueType = f32;
    /// impl Delete<MyKeyType, MyValueType> for MyKeyspace {
    ///     type QueryOrPrepared = PreparedStatement;
    ///     fn statement(&self) -> Cow<'static, str> {
    ///         format!("DELETE FROM {}.table WHERE key = ?", self.name()).into()
    ///     }
    ///     fn bind_values<T: Values>(builder: T, key: &MyKeyType) -> T::Return {
    ///         builder.bind(key)
    ///     }
    /// }
    /// # let my_key = 1;
    /// MyKeyspace::new("my_keyspace")
    ///     .prepare_delete::<MyKeyType, MyValueType>()
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn prepare_delete<K, V>(&self) -> PrepareRequest
    where
        Self: Delete<K, V>,
    {
        let statement = self.statement();
        PrepareRequest::new(statement)
    }
}

/// Specifies helper functions for creating dynamic prepare requests from anything that can be interpreted as a keyspace

pub trait GetDynamicPrepareRequest: Keyspace {
    /// Create a dynamic prepare request from a statement. The token `{{keyspace}}` will be replaced with the keyspace
    /// name.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// "my_keyspace"
    ///     .prepare_with("DELETE FROM {{keyspace}}.table WHERE key = ?")
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn prepare_with(&self, statement: &str) -> PrepareRequest {
        PrepareRequest::new(statement.to_string().into())
    }
}

/// Specifies helper functions for creating dynamic prepare requests from anything that can be interpreted as a
/// statement

pub trait AsDynamicPrepareRequest: ToStatement {
    /// Create a dynamic prepare request from a statement.
    /// name.
    ///
    /// ## Example
    /// ```no_run
    /// use scylla_rs::app::access::*;
    /// "DELETE FROM my_keyspace.table WHERE key = ?"
    ///     .prepare()
    ///     .get_local_blocking()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn prepare(&self) -> PrepareRequest {
        let statement = self.to_statement();
        PrepareRequest::new(statement)
    }
}

impl<S: Keyspace> GetStaticPrepareRequest for S {}
impl<S: Keyspace> GetDynamicPrepareRequest for S {}
impl<S: ToStatement> AsDynamicPrepareRequest for S {}

/// A request to prepare a record which can be sent to the ring
#[derive(Debug, Clone)]
pub struct PrepareRequest {
    pub(crate) statement: Cow<'static, str>,
    pub(crate) token: i64,
}

impl PrepareRequest {
    fn new(statement: Cow<'static, str>) -> Self {
        PrepareRequest {
            statement,
            token: rand::random(),
        }
    }
}

impl Request for PrepareRequest {
    fn token(&self) -> i64 {
        self.token
    }

    fn statement(&self) -> &Cow<'static, str> {
        &self.statement
    }

    fn payload(&self) -> Vec<u8> {
        Prepare::new().statement(&self.statement).build().unwrap().0
    }
}

#[async_trait::async_trait]
impl SendRequestExt for PrepareRequest {
    type Marker = DecodeVoid;
    type Worker = PrepareWorker;
    const TYPE: RequestType = RequestType::Execute;

    fn worker(self) -> Box<Self::Worker> {
        Box::new(PrepareWorker::from(self))
    }
}
