// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::prelude::PreparedResult;
use std::fmt::Debug;

/// A statement prepare worker
#[derive(Debug)]
pub struct PrepareWorker<P> {
    /// The expected id for this statement
    pub(crate) id: [u8; 16],
    pub(crate) retries: usize,
    pub(crate) request: PrepareRequest<P>,
}
impl<P> PrepareWorker<P> {
    /// Create a new prepare worker
    pub fn new(keyspace: Option<String>, id: [u8; 16], statement: String) -> Box<Self> {
        Box::new(Self {
            id,
            retries: 0,
            request: PrepareRequest {
                keyspace,
                statement,
                token: rand::random(),
                _marker: std::marker::PhantomData,
            },
        })
    }
}

impl<P> From<PrepareRequest<P>> for PrepareWorker<P> {
    fn from(request: PrepareRequest<P>) -> Self {
        Self {
            id: md5::compute(request.statement.as_bytes()).into(),
            retries: 0,
            request,
        }
    }
}
impl<P> Worker for PrepareWorker<P>
where
    P: 'static + Debug,
{
    fn handle_response(self: Box<Self>, _body: ResponseBody) -> anyhow::Result<()> {
        info!("Successfully prepared statement: '{}'", self.request.statement);
        Ok(())
    }
    fn handle_error(self: Box<Self>, error: WorkerError, _reporter: Option<&ReporterHandle>) -> anyhow::Result<()> {
        error!(
            "Failed to prepare statement: {}, error: {}",
            self.request.statement, error
        );
        self.retry().ok();
        Ok(())
    }
}

impl<P> RetryableWorker<PrepareRequest<P>> for PrepareWorker<P>
where
    P: 'static + Debug,
{
    fn retries(&self) -> usize {
        self.retries
    }

    fn retries_mut(&mut self) -> &mut usize {
        &mut self.retries
    }

    fn request(&self) -> &PrepareRequest<P> {
        &self.request
    }
}

impl<H, P> IntoRespondingWorker<PrepareRequest<P>, H, ResponseBody> for PrepareWorker<P>
where
    H: 'static + HandleResponse<ResponseBody> + HandleError + Debug + Send + Sync,
    P: 'static + From<PreparedQuery> + Debug + Send + Sync,
{
    type Output = RespondingPrepareWorker<H, P>;

    fn with_handle(self: Box<Self>, handle: H) -> Box<Self::Output> {
        Box::new(RespondingPrepareWorker {
            id: self.id,
            retries: self.retries,
            request: self.request,
            handle,
        })
    }
}

/// A statement prepare worker
#[derive(Debug)]
pub struct RespondingPrepareWorker<H, P> {
    /// The expected id for this statement
    pub(crate) id: [u8; 16],
    pub(crate) request: PrepareRequest<P>,
    pub(crate) retries: usize,
    pub(crate) handle: H,
}

impl<H, P> Worker for RespondingPrepareWorker<H, P>
where
    H: 'static + HandleResponse<ResponseBody> + HandleError + Debug + Send + Sync,
    P: 'static + Debug,
{
    fn handle_response(self: Box<Self>, body: ResponseBody) -> anyhow::Result<()> {
        self.handle.handle_response(body)
    }
    fn handle_error(self: Box<Self>, error: WorkerError, _reporter: Option<&ReporterHandle>) -> anyhow::Result<()> {
        error!("{}", error);
        match self.retry() {
            Ok(_) => Ok(()),
            Err(worker) => worker.handle.handle_error(error),
        }
    }
}

impl<H, P> RetryableWorker<PrepareRequest<P>> for RespondingPrepareWorker<H, P>
where
    H: 'static + HandleResponse<ResponseBody> + HandleError + Debug + Send + Sync,
    P: 'static + Debug,
{
    fn retries(&self) -> usize {
        self.retries
    }

    fn retries_mut(&mut self) -> &mut usize {
        &mut self.retries
    }

    fn request(&self) -> &PrepareRequest<P> {
        &self.request
    }
}

impl<H, P> RespondingWorker<PrepareRequest<P>, H, ResponseBody> for RespondingPrepareWorker<H, P>
where
    H: 'static + HandleResponse<ResponseBody> + HandleError + Debug + Send + Sync,
    P: 'static + Debug,
{
    fn handle(&self) -> &H {
        &self.handle
    }
}
