// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

/// A value selecting worker
#[derive(Clone)]
pub struct ValueWorker<H, S: Select<K, V>, K, V>
where
    S: 'static + Select<K, V> + Clone,
    K: 'static + Send + Clone,
    V: 'static + Send + Clone,
    H: 'static + Send + HandleResponse<Self, Response = Option<V>> + HandleError<Self> + Clone,
{
    /// A handle which can be used to return the queried value
    pub handle: H,
    /// The keyspace this worker operates on
    pub keyspace: S,
    /// The key used to lookup the value
    pub key: K,
    /// The query page size, used when retying due to failure
    pub page_size: Option<i32>,
    /// The query paging state, used when retrying due to failure
    pub paging_state: Option<Vec<u8>>,
    /// The number of times this worker will retry on failure
    pub retries: usize,
    _marker: std::marker::PhantomData<V>,
}

impl<H, S: Select<K, V>, K, V> ValueWorker<H, S, K, V>
where
    S: 'static + Select<K, V> + Clone,
    K: 'static + Send + Clone,
    V: 'static + Send + Clone,
    H: 'static + Send + HandleResponse<Self, Response = Option<V>> + HandleError<Self> + Clone,
{
    /// Create a new value selecting worker with a number of retries and a response handle
    pub fn new(handle: H, keyspace: S, key: K, retries: usize, _marker: std::marker::PhantomData<V>) -> Self {
        Self {
            handle,
            keyspace,
            key,
            page_size: None,
            paging_state: None,
            retries,
            _marker,
        }
    }
    /// Create a new boxed value selecting worker with a number of retries and a response handle
    pub fn boxed(handle: H, keyspace: S, key: K, retries: usize, _marker: std::marker::PhantomData<V>) -> Box<Self> {
        Box::new(Self::new(handle, keyspace, key, retries, _marker))
    }
    /// Add paging information to this worker
    pub fn with_paging<P: Into<Option<Vec<u8>>>>(mut self, page_size: i32, paging_state: P) -> Self {
        self.page_size = Some(page_size);
        self.paging_state = paging_state.into();
        self
    }
}

impl<H, S, K, V> DecodeResponse<Option<V>> for ValueWorker<H, S, K, V>
where
    H: Send + HandleResponse<Self, Response = Option<V>> + HandleError<ValueWorker<H, S, K, V>> + Clone,
    S: Select<K, V> + Clone,
    K: Send + Clone,
    V: Send + Clone,
{
    fn decode_response(decoder: Decoder) -> Option<V> {
        S::decode(decoder)
    }
}

impl<S, H, K, V> Worker for ValueWorker<H, S, K, V>
where
    S: 'static + Select<K, V> + Clone,
    K: 'static + Send + Clone,
    V: 'static + Send + Clone,
    H: 'static + Send + HandleResponse<Self, Response = Option<V>> + HandleError<Self> + Clone,
{
    fn handle_response(self: Box<Self>, giveload: Vec<u8>) {
        let rows = Self::decode_response(Decoder::from(giveload));
        H::handle_response(self, rows)
    }

    fn handle_error(mut self: Box<Self>, mut error: WorkerError, reporter: &Option<ReporterHandle>) {
        if let WorkerError::Cql(ref mut cql_error) = error {
            if let (Some(id), Some(reporter)) = (cql_error.take_unprepared_id(), reporter) {
                handle_select_unprepared_error(
                    &self,
                    &self.keyspace,
                    &self.key,
                    id,
                    self.page_size,
                    &self.paging_state,
                    reporter,
                );
                return ();
            }
        }
        if self.retries > 0 {
            self.retries -= 1;
            // currently we assume all cql/worker errors are retryable, but we might change this in future
            let req = self.keyspace.select_query::<V>(&self.key).consistency(Consistency::One);
            let req = if let Some(page_size) = self.page_size {
                req.page_size(page_size).paging_state(&self.paging_state)
            } else {
                req.paging_state(&self.paging_state)
            }
            .build();
            tokio::spawn(async { req.send_global(self) });
        } else {
            // no more retries
            // print error!
            error!("{:?}, reporter running: {}", error, reporter.is_some());
            H::handle_error(self, error);
        }
    }
}

impl<S, K, V> HandleResponse<ValueWorker<UnboundedSender<Result<Option<V>, WorkerError>>, S, K, V>>
    for UnboundedSender<Result<Option<V>, WorkerError>>
where
    S: 'static + Send + Select<K, V> + Clone,
    K: 'static + Send + Clone,
    V: 'static + Send + Clone,
{
    type Response = Option<V>;
    fn handle_response(
        worker: Box<ValueWorker<UnboundedSender<Result<Option<V>, WorkerError>>, S, K, V>>,
        response: Self::Response,
    ) {
        let _ = worker.handle.send(Ok(response));
    }
}

impl<S, K, V> HandleError<ValueWorker<UnboundedSender<Result<Option<V>, WorkerError>>, S, K, V>>
    for UnboundedSender<Result<Option<V>, WorkerError>>
where
    S: 'static + Send + Select<K, V> + Clone,
    K: 'static + Send + Clone,
    V: 'static + Send + Clone,
{
    fn handle_error(
        worker: Box<ValueWorker<UnboundedSender<Result<Option<V>, WorkerError>>, S, K, V>>,
        worker_error: WorkerError,
    ) {
        let _ = worker.handle.send(Err(worker_error));
    }
}
