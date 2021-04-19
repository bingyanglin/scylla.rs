// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[async_trait::async_trait]
impl<H: ScyllaScope> Terminating<ScyllaHandle<H>> for Websocket {
    async fn terminating(
        &mut self,
        _status: Result<(), Need>,
        _supervisor: &mut Option<ScyllaHandle<H>>,
    ) -> Result<(), Need> {
        _status
    }
}
