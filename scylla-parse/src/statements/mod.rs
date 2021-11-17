use crate::{
    keywords::*,
    Brackets,
    Caching,
    ColumnOrder,
    Compaction,
    CompactionType,
    Compression,
    CqlType,
    If,
    KeyspaceQualifiedName,
    List,
    LitStr,
    MapLiteral,
    Name,
    Nothing,
    Parens,
    Parse,
    Peek,
    Relation,
    Replication,
    SetLiteral,
    SpeculativeRetry,
    StatementOpt,
    StatementStream,
    TableOpts,
    Term,
    Token,
};
use derive_builder::Builder;
use derive_more::{
    From,
    TryInto,
};
use scylla_parse_macros::ParseFromStr;
use std::{
    convert::TryFrom,
    fmt::{
        Display,
        Formatter,
    },
    str::FromStr,
};

mod ddl;
pub use ddl::*;

mod dml;
pub use dml::*;

mod index;
pub use index::*;

mod views;
pub use views::*;

mod function;
pub use function::*;

mod security;
pub use security::*;

mod trigger;
pub use trigger::*;

#[derive(ParseFromStr, Clone, Debug, TryInto, From)]
pub enum Statement {
    DataDefinition(DataDefinitionStatement),
    DataManipulation(DataManipulationStatement),
    SecondaryIndex(SecondaryIndexStatement),
    MaterializedView(MaterializedViewStatement),
    Role(RoleStatement),
    Permission(PermissionStatement),
    User(UserStatement),
    UserDefinedFunction(UserDefinedFunctionStatement),
    UserDefinedType(UserDefinedTypeStatement),
    Trigger(TriggerStatement),
}

macro_rules! impl_try_into_statements {
    ($($via:ty => {$($stmt:ty),*}),*) => {
        $($(
            impl std::convert::TryInto<$stmt> for Statement {
                type Error = anyhow::Error;

                fn try_into(self) -> Result<$stmt, Self::Error> {
                    match <$via>::try_from(self) {
                        Ok(v) => v.try_into().map_err(|e: &str| anyhow::anyhow!(e)),
                        Err(err) => Err(anyhow::anyhow!(
                            "Could not convert Statement to {}: {}",
                            std::any::type_name::<$stmt>(),
                            err
                        )),
                    }
                }
            }
        )*)*
    };
}

impl_try_into_statements!(
    DataDefinitionStatement => {UseStatement, CreateKeyspaceStatement, AlterKeyspaceStatement, DropKeyspaceStatement, CreateTableStatement, AlterTableStatement, DropTableStatement, TruncateStatement},
    DataManipulationStatement => {InsertStatement, UpdateStatement, DeleteStatement, SelectStatement, BatchStatement},
    SecondaryIndexStatement => {CreateIndexStatement, DropIndexStatement},
    MaterializedViewStatement => {CreateMaterializedViewStatement, AlterMaterializedViewStatement, DropMaterializedViewStatement},
    RoleStatement => {CreateRoleStatement, AlterRoleStatement, DropRoleStatement, GrantRoleStatement, RevokeRoleStatement, ListRolesStatement},
    PermissionStatement => {GrantPermissionStatement, RevokePermissionStatement, ListPermissionsStatement},
    UserStatement => {CreateUserStatement, AlterUserStatement, DropUserStatement, ListUsersStatement},
    UserDefinedFunctionStatement => {CreateFunctionStatement, DropFunctionStatement, CreateAggregateFunctionStatement, DropAggregateFunctionStatement},
    UserDefinedTypeStatement => {CreateUserDefinedTypeStatement, AlterUserDefinedTypeStatement, DropUserDefinedTypeStatement},
    TriggerStatement => {CreateTriggerStatement, DropTriggerStatement}
);

impl Parse for Statement {
    type Output = Statement;

    fn parse(s: &mut crate::StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if let Some(stmt) = s.parse()? {
            Self::DataDefinition(stmt)
        } else if let Some(stmt) = s.parse()? {
            Self::DataManipulation(stmt)
        } else if let Some(stmt) = s.parse()? {
            Self::SecondaryIndex(stmt)
        } else if let Some(stmt) = s.parse()? {
            Self::MaterializedView(stmt)
        } else if let Some(stmt) = s.parse()? {
            Self::Role(stmt)
        } else if let Some(stmt) = s.parse()? {
            Self::Permission(stmt)
        } else if let Some(stmt) = s.parse()? {
            Self::User(stmt)
        } else if let Some(stmt) = s.parse()? {
            Self::UserDefinedFunction(stmt)
        } else if let Some(stmt) = s.parse()? {
            Self::UserDefinedType(stmt)
        } else if let Some(stmt) = s.parse()? {
            Self::Trigger(stmt)
        } else {
            anyhow::bail!("Invalid statement: {}", s.parse_from::<Token>()?)
        })
    }
}

// TODO: Add more usability APIs
pub trait KeyspaceExt {
    fn get_keyspace(&self) -> Option<String>;

    fn set_keyspace(&mut self, keyspace: &str);
}

pub trait WhereExt {
    fn iter_where(&self) -> Option<std::slice::Iter<Relation>>;
}

pub trait KeyspaceOptionsExt {
    fn keyspace_opts(&self) -> &KeyspaceOpts;
    fn keyspace_opts_mut(&mut self) -> &mut KeyspaceOpts;
    fn set_replication(&mut self, replication: Replication) {
        self.keyspace_opts_mut().replication = replication;
    }
    fn set_durable_writes(&mut self, durable_writes: bool) {
        self.keyspace_opts_mut().durable_writes.replace(durable_writes);
    }

    fn with_replication(mut self, replication: Replication) -> Self
    where
        Self: Sized,
    {
        self.set_replication(replication);
        self
    }
    fn with_durable_writes(mut self, durable_writes: bool) -> Self
    where
        Self: Sized,
    {
        self.set_durable_writes(durable_writes);
        self
    }

    fn get_replication(&self) -> &Replication {
        &self.keyspace_opts().replication
    }
    fn get_durable_writes(&self) -> Option<bool> {
        self.keyspace_opts().durable_writes
    }
}

pub trait TableOptionsExt {
    fn table_opts(&self) -> &Option<TableOpts>;
    fn table_opts_mut(&mut self) -> &mut Option<TableOpts>;
    fn set_compact_storage(&mut self, compact_storage: bool) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .compact_storage = compact_storage;
    }
    fn set_clustering_order(&mut self, clustering_order: Vec<ColumnOrder>) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .clustering_order
            .replace(clustering_order);
    }
    fn set_comment(&mut self, comment: &str) {
        let c = comment.to_string().into();
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .comment
            .replace(c);
    }

    fn set_speculative_retry(&mut self, speculative_retry: SpeculativeRetry) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .speculative_retry
            .replace(speculative_retry);
    }

    fn set_change_data_capture(&mut self, cdc: bool) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .change_data_capture
            .replace(cdc);
    }

    fn set_gc_grace_seconds(&mut self, gc_grace_seconds: i32) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .gc_grace_seconds
            .replace(gc_grace_seconds);
    }

    fn set_bloom_filter_fp_chance(&mut self, bloom_filter_fp_chance: f32) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .bloom_filter_fp_chance
            .replace(bloom_filter_fp_chance);
    }

    fn set_default_time_to_live(&mut self, default_time_to_live: i32) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .default_time_to_live
            .replace(default_time_to_live);
    }

    fn set_compaction(&mut self, compaction: impl CompactionType) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .compaction
            .replace(compaction.into());
    }

    fn set_compression(&mut self, compression: Compression) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .compression
            .replace(compression);
    }

    fn set_caching(&mut self, caching: Caching) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .caching
            .replace(caching);
    }

    fn set_memtable_flush_period_in_ms(&mut self, memtable_flush_period_in_ms: i32) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .memtable_flush_period_in_ms
            .replace(memtable_flush_period_in_ms);
    }

    fn set_read_repair(&mut self, read_repair: bool) {
        self.table_opts_mut()
            .get_or_insert_with(Default::default)
            .read_repair
            .replace(read_repair);
    }
    fn set_additional_write_policy(&mut self, speculative_retry: SpeculativeRetry) {
        self.set_speculative_retry(speculative_retry)
    }

    fn with_compact_storage(mut self, compact_storage: bool) -> Self
    where
        Self: Sized,
    {
        self.set_compact_storage(compact_storage);
        self
    }
    fn with_clustering_order(mut self, clustering_order: Vec<ColumnOrder>) -> Self
    where
        Self: Sized,
    {
        self.set_clustering_order(clustering_order);
        self
    }
    fn with_comment(mut self, comment: &str) -> Self
    where
        Self: Sized,
    {
        self.set_comment(comment);
        self
    }
    fn with_speculative_retry(mut self, speculative_retry: SpeculativeRetry) -> Self
    where
        Self: Sized,
    {
        self.set_speculative_retry(speculative_retry);
        self
    }
    fn with_change_data_capture(mut self, cdc: bool) -> Self
    where
        Self: Sized,
    {
        self.set_change_data_capture(cdc);
        self
    }
    fn with_additional_write_policy(mut self, speculative_retry: SpeculativeRetry) -> Self
    where
        Self: Sized,
    {
        self.set_additional_write_policy(speculative_retry);
        self
    }
    fn with_gc_grace_seconds(mut self, gc_grace_seconds: i32) -> Self
    where
        Self: Sized,
    {
        self.set_gc_grace_seconds(gc_grace_seconds);
        self
    }
    fn with_bloom_filter_fp_chance(mut self, bloom_filter_fp_chance: f32) -> Self
    where
        Self: Sized,
    {
        self.set_bloom_filter_fp_chance(bloom_filter_fp_chance);
        self
    }
    fn with_default_time_to_live(mut self, default_time_to_live: i32) -> Self
    where
        Self: Sized,
    {
        self.set_default_time_to_live(default_time_to_live);
        self
    }
    fn with_compaction(mut self, compaction: impl CompactionType) -> Self
    where
        Self: Sized,
    {
        self.set_compaction(compaction);
        self
    }
    fn with_compression(mut self, compression: Compression) -> Self
    where
        Self: Sized,
    {
        self.set_compression(compression);
        self
    }
    fn with_caching(mut self, caching: Caching) -> Self
    where
        Self: Sized,
    {
        self.set_caching(caching);
        self
    }
    fn with_memtable_flush_period_in_ms(mut self, memtable_flush_period_in_ms: i32) -> Self
    where
        Self: Sized,
    {
        self.set_memtable_flush_period_in_ms(memtable_flush_period_in_ms);
        self
    }
    fn with_read_repair(mut self, read_repair: bool) -> Self
    where
        Self: Sized,
    {
        self.set_read_repair(read_repair);
        self
    }

    fn get_compact_storage(&self) -> Option<&bool> {
        self.table_opts().as_ref().map(|t| &t.compact_storage)
    }
    fn get_clustering_order(&self) -> Option<&Vec<ColumnOrder>> {
        self.table_opts().as_ref().and_then(|t| t.clustering_order.as_ref())
    }
    fn get_comment(&self) -> Option<&String> {
        self.table_opts()
            .as_ref()
            .and_then(|t| t.comment.as_ref().map(|s| &s.value))
    }
    fn get_speculative_retry(&self) -> Option<&SpeculativeRetry> {
        self.table_opts().as_ref().and_then(|t| t.speculative_retry.as_ref())
    }
    fn get_change_data_capture(&self) -> Option<bool> {
        self.table_opts().as_ref().and_then(|t| t.change_data_capture)
    }
    fn get_additional_write_policy(&self) -> Option<&SpeculativeRetry> {
        self.table_opts().as_ref().and_then(|t| t.speculative_retry.as_ref())
    }
    fn get_gc_grace_seconds(&self) -> Option<i32> {
        self.table_opts().as_ref().and_then(|t| t.gc_grace_seconds)
    }
    fn get_bloom_filter_fp_chance(&self) -> Option<f32> {
        self.table_opts().as_ref().and_then(|t| t.bloom_filter_fp_chance)
    }
    fn get_default_time_to_live(&self) -> Option<i32> {
        self.table_opts().as_ref().and_then(|t| t.default_time_to_live)
    }
    fn get_compaction(&self) -> Option<&Compaction> {
        self.table_opts().as_ref().and_then(|t| t.compaction.as_ref())
    }
    fn get_compression(&self) -> Option<&Compression> {
        self.table_opts().as_ref().and_then(|t| t.compression.as_ref())
    }
    fn get_caching(&self) -> Option<&Caching> {
        self.table_opts().as_ref().and_then(|t| t.caching.as_ref())
    }
    fn get_memtable_flush_period_in_ms(&self) -> Option<i32> {
        self.table_opts().as_ref().and_then(|t| t.memtable_flush_period_in_ms)
    }
    fn get_read_repair(&self) -> Option<bool> {
        self.table_opts().as_ref().and_then(|t| t.read_repair)
    }
}
