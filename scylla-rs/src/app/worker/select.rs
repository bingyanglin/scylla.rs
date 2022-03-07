// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::prelude::ErrorCode;
use std::fmt::Debug;

/// A select worker
#[derive(Clone, Debug)]
pub struct SelectWorker<H, R> {
    /// The worker's request
    pub request: R,
    /// A handle which can be used to return the queried value
    pub handle: H,
    /// The query page size, used when retying due to failure
    pub page_size: Option<i32>,
    /// The query paging state, used when retrying due to failure
    pub paging_state: Option<Vec<u8>>,
    /// The number of times this worker will retry on failure
    pub retries: usize,
}

impl<H, R> SelectWorker<H, R>
where
    H: 'static,
    R: Request,
{
    /// Create a new value selecting worker with a number of retries and a response handle
    pub fn new(request: R, handle: H) -> Box<Self> {
        Box::new(Self {
            request,
            handle,
            page_size: None,
            paging_state: None,
            retries: 0,
        })
    }

    pub(crate) fn from(BasicRetryWorker { request, retries }: BasicRetryWorker<R>, handle: H) -> Box<Self>
    where
        H: HandleResponse<ResponseBody> + HandleError + Debug + Send + Sync,
        R: 'static + Request + Debug + Send + Sync,
    {
        Self::new(request, handle).with_retries(retries)
    }

    /// Add paging information to this worker
    pub fn with_paging<P: Into<Option<Vec<u8>>>>(mut self: Box<Self>, page_size: i32, paging_state: P) -> Box<Self> {
        self.page_size = Some(page_size);
        self.paging_state = paging_state.into();
        self
    }
}

impl<H, R> Worker for SelectWorker<H, R>
where
    H: 'static + HandleResponse<ResponseBody> + HandleError + Debug + Send + Sync,
    R: 'static + Send + Debug + Request + Sync,
{
    fn handle_response(self: Box<Self>, body: ResponseBody) -> anyhow::Result<()> {
        self.handle.handle_response(body)
    }

    fn handle_error(self: Box<Self>, mut error: WorkerError, reporter: Option<&ReporterHandle>) -> anyhow::Result<()> {
        error!("{}", error);
        if let WorkerError::Cql(ref mut cql_error) = error {
            match cql_error.code() {
                ErrorCode::Unprepared => {
                    if let Some(reporter) = reporter {
                        handle_unprepared_error(self, cql_error.unprepared_id().unwrap(), reporter).or_else(|worker| {
                            error!("Error trying to reprepare query: {:?}", worker.request().statement());
                            worker.handle.handle_error(error)
                        })
                    } else {
                        match self.retry() {
                            Ok(_) => Ok(()),
                            Err(worker) => worker.handle.handle_error(error),
                        }
                    }
                }
                _ => match self.retry() {
                    Ok(_) => Ok(()),
                    Err(worker) => worker.handle.handle_error(error),
                },
            }
        } else {
            match self.retry() {
                Ok(_) => Ok(()),
                Err(worker) => worker.handle.handle_error(error),
            }
        }
    }
}

impl<H, R> RetryableWorker<R> for SelectWorker<H, R>
where
    H: 'static + HandleResponse<ResponseBody> + HandleError + Debug + Send + Sync,
    R: 'static + Send + Debug + Request + Sync,
{
    fn retries(&self) -> usize {
        self.retries
    }

    fn request(&self) -> &R {
        &self.request
    }

    fn retries_mut(&mut self) -> &mut usize {
        &mut self.retries
    }
}

impl<R, H> RespondingWorker<R, H, ResponseBody> for SelectWorker<H, R>
where
    H: 'static + HandleResponse<ResponseBody> + HandleError + Debug + Send + Sync,
    R: 'static + Send + Debug + Request + Sync,
{
    fn handle(&self) -> &H {
        &self.handle
    }
}
