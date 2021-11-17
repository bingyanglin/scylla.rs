use super::*;
use crate::{
    ArithmeticOp,
    BindMarker,
    DurationLiteral,
    ListLiteral,
    Operator,
    ReservedKeyword,
    TupleLiteral,
};

#[derive(ParseFromStr, Clone, Debug, TryInto, From)]
pub enum DataManipulationStatement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Batch(BatchStatement),
}

impl Parse for DataManipulationStatement {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if let Some(keyword) = s.find::<ReservedKeyword>() {
            match keyword {
                ReservedKeyword::SELECT => Self::Select(s.parse()?),
                ReservedKeyword::INSERT => Self::Insert(s.parse()?),
                ReservedKeyword::UPDATE => Self::Update(s.parse()?),
                ReservedKeyword::DELETE => Self::Delete(s.parse()?),
                ReservedKeyword::BATCH => Self::Batch(s.parse()?),
                _ => anyhow::bail!("Expected a data manipulation statement!"),
            }
        } else {
            anyhow::bail!("Expected a data manipulation statement!")
        })
    }
}

impl Peek for DataManipulationStatement {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<InsertStatement>()
            || s.check::<UpdateStatement>()
            || s.check::<DeleteStatement>()
            || s.check::<SelectStatement>()
            || s.check::<BatchStatement>()
    }
}

impl Display for DataManipulationStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Select(s) => s.fmt(f),
            Self::Insert(s) => s.fmt(f),
            Self::Update(s) => s.fmt(f),
            Self::Delete(s) => s.fmt(f),
            Self::Batch(s) => s.fmt(f),
        }
    }
}

#[derive(ParseFromStr, Builder, Clone, Debug)]
pub struct SelectStatement {
    #[builder(default)]
    pub distinct: bool,
    pub select_clause: SelectClauseKind,
    pub from: FromClause,
    #[builder(default)]
    pub where_clause: Option<WhereClause>,
    #[builder(default)]
    pub group_by_clause: Option<GroupByClause>,
    #[builder(default)]
    pub order_by_clause: Option<OrderingClause>,
    #[builder(default)]
    pub per_partition_limit: Option<Limit>,
    #[builder(default)]
    pub limit: Option<Limit>,
    #[builder(default)]
    pub allow_filtering: bool,
    #[builder(default)]
    pub bypass_cache: bool,
    #[builder(default)]
    pub timeout: Option<DurationLiteral>,
}

impl Parse for SelectStatement {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output>
    where
        Self: Sized,
    {
        s.parse::<SELECT>()?;
        let mut res = SelectStatementBuilder::default();
        res.distinct(s.parse::<Option<DISTINCT>>()?.is_some())
            .select_clause(s.parse()?)
            .from(s.parse()?);
        loop {
            if let Some(where_clause) = s.parse_if() {
                if res.where_clause.is_some() {
                    anyhow::bail!("Duplicate WHERE clause!");
                }
                res.where_clause(where_clause?);
            } else if let Some(group_by_clause) = s.parse_if() {
                if res.group_by_clause.is_some() {
                    anyhow::bail!("Duplicate GROUP BY clause!");
                }
                res.group_by_clause(group_by_clause?);
            } else if let Some(order_by_clause) = s.parse_if() {
                if res.order_by_clause.is_some() {
                    anyhow::bail!("Duplicate ORDER BY clause!");
                }
                res.order_by_clause(order_by_clause?);
            } else if s.parse_if::<(PER, PARTITION, LIMIT)>().is_some() {
                if res.per_partition_limit.is_some() {
                    anyhow::bail!("Duplicate PER PARTITION LIMIT clause!");
                }
                res.per_partition_limit(Some(s.parse::<Limit>()?));
            } else if s.parse_if::<LIMIT>().is_some() {
                if res.limit.is_some() {
                    anyhow::bail!("Duplicate LIMIT clause!");
                }
                res.limit(Some(s.parse::<Limit>()?));
            } else if s.parse_if::<(ALLOW, FILTERING)>().is_some() {
                if res.allow_filtering.is_some() {
                    anyhow::bail!("Duplicate ALLOW FILTERING clause!");
                }
                res.allow_filtering(true);
            } else if s.parse_if::<(BYPASS, CACHE)>().is_some() {
                if res.bypass_cache.is_some() {
                    anyhow::bail!("Duplicate BYPASS CACHE clause!");
                }
                res.bypass_cache(true);
            } else if s.parse_if::<(USING, TIMEOUT)>().is_some() {
                if res.timeout.is_some() {
                    anyhow::bail!("Duplicate USING TIMEOUT clause!");
                }
                res.timeout(Some(s.parse::<DurationLiteral>()?));
            } else {
                break;
            }
        }
        s.parse::<Option<Semicolon>>()?;
        Ok(res
            .build()
            .map_err(|e| anyhow::anyhow!("Invalid SELECT statement: {}", e))?)
    }
}

impl Peek for SelectStatement {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<SELECT>()
    }
}

impl Display for SelectStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SELECT {}{} {}",
            if self.distinct { "DISTINCT " } else { "" },
            self.select_clause,
            self.from
        )?;
        if let Some(where_clause) = &self.where_clause {
            write!(f, " {}", where_clause)?;
        }
        if let Some(group_by_clause) = &self.group_by_clause {
            write!(f, " {}", group_by_clause)?;
        }
        if let Some(order_by_clause) = &self.order_by_clause {
            write!(f, " {}", order_by_clause)?;
        }
        if let Some(per_partition_limit) = &self.per_partition_limit {
            write!(f, " PER PARTITION LIMIT {}", per_partition_limit)?;
        }
        if let Some(limit) = &self.limit {
            write!(f, " LIMIT {}", limit)?;
        }
        if self.allow_filtering {
            write!(f, " ALLOW FILTERING")?;
        }
        if self.bypass_cache {
            write!(f, " BYPASS CACHE")?;
        }
        if let Some(timeout) = &self.timeout {
            write!(f, " USING TIMEOUT {}", timeout)?;
        }
        Ok(())
    }
}

impl KeyspaceExt for SelectStatement {
    fn get_keyspace(&self) -> Option<String> {
        self.from.table.keyspace.as_ref().map(|n| n.to_string())
    }

    fn set_keyspace(&mut self, keyspace: &str) {
        self.from.table.keyspace = Some(Name::Quoted(keyspace.to_string()));
    }
}

impl WhereExt for SelectStatement {
    fn iter_where(&self) -> Option<std::slice::Iter<Relation>> {
        self.where_clause.as_ref().map(|w| w.relations.iter())
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum SelectClauseKind {
    All,
    Selectors(Vec<Selector>),
}

impl Parse for SelectClauseKind {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output>
    where
        Self: Sized,
    {
        Ok(if s.parse_if::<Star>().is_some() {
            SelectClauseKind::All
        } else {
            SelectClauseKind::Selectors(s.parse_from::<List<Selector, Comma>>()?)
        })
    }
}

impl Display for SelectClauseKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectClauseKind::All => write!(f, "*"),
            SelectClauseKind::Selectors(selectors) => {
                for (i, selector) in selectors.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    selector.fmt(f)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub struct Selector {
    pub kind: SelectorKind,
    pub as_id: Option<Name>,
}

impl Parse for Selector {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output>
    where
        Self: Sized,
    {
        let (kind, as_id) = s.parse::<(SelectorKind, Option<(AS, Name)>)>()?;
        Ok(Self {
            kind,
            as_id: as_id.map(|(_, id)| id),
        })
    }
}

impl Display for Selector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)?;
        if let Some(id) = &self.as_id {
            write!(f, " AS {}", id)?;
        }
        Ok(())
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub struct SelectorFunction {
    pub function: Name,
    pub args: Vec<Selector>,
}

impl Parse for SelectorFunction {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output>
    where
        Self: Sized,
    {
        let (function, args) = s.parse_from::<(Name, Parens<List<Selector, Comma>>)>()?;
        Ok(SelectorFunction { function, args })
    }
}

impl Peek for SelectorFunction {
    fn peek(mut s: StatementStream<'_>) -> bool {
        if s.parse_if::<Name>().is_some() {
            s.check::<LeftParen>()
        } else {
            false
        }
    }
}

impl Display for SelectorFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({})",
            self.function,
            self.args.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
        )
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum SelectorKind {
    Column(Name),
    Term(Term),
    Cast(Box<Selector>, CqlType),
    Function(SelectorFunction),
    Count,
}

impl Parse for SelectorKind {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output>
    where
        Self: Sized,
    {
        Ok(if s.parse_if::<CAST>().is_some() {
            let (selector, _, cql_type) = s.parse_from::<Parens<(Selector, AS, CqlType)>>()?;
            Self::Cast(Box::new(selector), cql_type)
        } else if s.parse_if::<COUNT>().is_some() {
            // TODO: Double check that this is ok
            s.parse_from::<Parens<char>>()?;
            Self::Count
        } else if let Some(f) = s.parse_if() {
            Self::Function(f?)
        } else if let Some(id) = s.parse_if() {
            Self::Column(id?)
        } else if let Some(term) = s.parse_if() {
            Self::Term(term?)
        } else {
            anyhow::bail!("Invalid selector: {}", s.parse_from::<Token>()?)
        })
    }
}

impl Display for SelectorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorKind::Column(id) => id.fmt(f),
            SelectorKind::Term(term) => term.fmt(f),
            SelectorKind::Cast(selector, cql_type) => write!(f, "CAST({} AS {})", selector, cql_type),
            SelectorKind::Function(func) => func.fmt(f),
            SelectorKind::Count => write!(f, "COUNT(*)"),
        }
    }
}

#[derive(ParseFromStr, Builder, Clone, Debug)]
pub struct InsertStatement {
    pub table: KeyspaceQualifiedName,
    pub kind: InsertKind,
    #[builder(default)]
    pub if_not_exists: bool,
    #[builder(default)]
    pub using: Option<Vec<UpdateParameter>>,
}

impl Parse for InsertStatement {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        s.parse::<(INSERT, INTO)>()?;
        let mut res = InsertStatementBuilder::default();
        res.table(s.parse::<KeyspaceQualifiedName>()?)
            .kind(s.parse::<InsertKind>()?);
        loop {
            if s.parse_if::<(IF, NOT, EXISTS)>().is_some() {
                if res.if_not_exists.is_some() {
                    anyhow::bail!("Duplicate IF NOT EXISTS clause!");
                }
                res.if_not_exists(true);
            } else if s.parse_if::<USING>().is_some() {
                if res.using.is_some() {
                    anyhow::bail!("Duplicate USING clause!");
                }
                res.using(Some(s.parse_from::<List<UpdateParameter, AND>>()?));
            } else {
                break;
            }
        }
        s.parse::<Option<Semicolon>>()?;
        Ok(res
            .build()
            .map_err(|e| anyhow::anyhow!("Invalid INSERT statement: {}", e))?)
    }
}

impl Peek for InsertStatement {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<INSERT>()
    }
}

impl Display for InsertStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "INSERT INTO {} {}", self.table, self.kind)?;
        if self.if_not_exists {
            write!(f, " IF NOT EXISTS")?;
        }
        if let Some(using) = &self.using {
            write!(
                f,
                " USING {}",
                using.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(" AND ")
            )?;
        }
        Ok(())
    }
}

impl KeyspaceExt for InsertStatement {
    fn get_keyspace(&self) -> Option<String> {
        self.table.keyspace.as_ref().map(|n| n.to_string())
    }

    fn set_keyspace(&mut self, keyspace: &str) {
        self.table.keyspace = Some(Name::Quoted(keyspace.to_string()));
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum InsertKind {
    NameValue {
        names: Vec<Name>,
        values: TupleLiteral,
    },
    Json {
        json: String,
        default: Option<ColumnDefault>,
    },
}

impl Parse for InsertKind {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        if s.parse_if::<JSON>().is_some() {
            let (json, default) = s.parse_from::<(String, Option<(DEFAULT, ColumnDefault)>)>()?;
            Ok(Self::Json {
                json,
                default: default.map(|(_, d)| d),
            })
        } else {
            let (names, _, values) = s.parse_from::<(Parens<List<Name, Comma>>, VALUES, TupleLiteral)>()?;
            Ok(Self::NameValue { names, values })
        }
    }
}

impl Display for InsertKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertKind::NameValue { names, values } => write!(
                f,
                "({}) VALUES {}",
                names.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "),
                values
            ),
            InsertKind::Json { json, default } => {
                write!(f, "JSON '{}'", json)?;
                if let Some(default) = default {
                    write!(f, " DEFAULT {}", default)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum UpdateParameter {
    TTL(Limit),
    Timestamp(Limit),
    Timeout(DurationLiteral),
}

impl Parse for UpdateParameter {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        if s.parse_if::<TTL>().is_some() {
            Ok(UpdateParameter::TTL(s.parse()?))
        } else if s.parse_if::<TIMESTAMP>().is_some() {
            Ok(UpdateParameter::Timestamp(s.parse()?))
        } else if s.parse_if::<TIMEOUT>().is_some() {
            Ok(UpdateParameter::Timeout(s.parse()?))
        } else {
            anyhow::bail!("Invalid update parameter: {}", s.parse_from::<Token>()?)
        }
    }
}

impl Peek for UpdateParameter {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<(TTL, Limit)>() || s.check::<(TIMESTAMP, Limit)>() || s.check::<(TIMEOUT, DurationLiteral)>()
    }
}

impl Display for UpdateParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateParameter::TTL(limit) => write!(f, "TTL {}", limit),
            UpdateParameter::Timestamp(limit) => write!(f, "TIMESTAMP {}", limit),
            UpdateParameter::Timeout(duration) => write!(f, "TIMEOUT {}", duration),
        }
    }
}

#[derive(ParseFromStr, Builder, Clone, Debug)]
pub struct UpdateStatement {
    pub table: KeyspaceQualifiedName,
    #[builder(default)]
    pub using: Option<Vec<UpdateParameter>>,
    pub set_clause: Vec<Assignment>,
    pub where_clause: WhereClause,
    #[builder(default)]
    pub if_clause: Option<IfClause>,
}

impl Parse for UpdateStatement {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        s.parse::<UPDATE>()?;
        let mut res = UpdateStatementBuilder::default();
        res.table(s.parse::<KeyspaceQualifiedName>()?)
            .using(
                s.parse_from::<Option<(USING, List<UpdateParameter, AND>)>>()?
                    .map(|(_, v)| v),
            )
            .set_clause(s.parse_from::<(SET, List<Assignment, Comma>)>()?.1)
            .where_clause(s.parse()?)
            .if_clause(s.parse()?);
        s.parse::<Option<Semicolon>>()?;
        Ok(res
            .build()
            .map_err(|e| anyhow::anyhow!("Invalid UPDATE statement: {}", e))?)
    }
}

impl Peek for UpdateStatement {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<UPDATE>()
    }
}

impl Display for UpdateStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "UPDATE {}", self.table)?;
        if let Some(using) = &self.using {
            write!(
                f,
                " USING {}",
                using.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(" AND ")
            )?;
        }
        write!(
            f,
            " SET {} {}",
            self.set_clause
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.where_clause
        )?;
        if let Some(if_clause) = &self.if_clause {
            write!(f, " {}", if_clause)?;
        }
        Ok(())
    }
}

impl KeyspaceExt for UpdateStatement {
    fn get_keyspace(&self) -> Option<String> {
        self.table.keyspace.as_ref().map(|n| n.to_string())
    }

    fn set_keyspace(&mut self, keyspace: &str) {
        self.table.keyspace = Some(Name::Quoted(keyspace.to_string()));
    }
}

impl WhereExt for UpdateStatement {
    fn iter_where(&self) -> Option<std::slice::Iter<Relation>> {
        Some(self.where_clause.relations.iter())
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum Assignment {
    Simple {
        selection: SimpleSelection,
        term: Term,
    },
    Arithmetic {
        assignee: Name,
        lhs: Name,
        op: ArithmeticOp,
        rhs: Term,
    },
    Append {
        assignee: Name,
        list: ListLiteral,
        item: Name,
    },
}

impl Parse for Assignment {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if let Some(a) = s.parse_if::<(_, Equals, _, Plus, _)>() {
            let (assignee, _, list, _, item) = a?;
            Self::Append { assignee, list, item }
        } else if let Some(a) = s.parse_if::<(_, Equals, _, _, _)>() {
            let (assignee, _, lhs, op, rhs) = a?;
            Self::Arithmetic { assignee, lhs, op, rhs }
        } else {
            let (selection, _, term) = s.parse::<(_, Equals, _)>()?;
            Self::Simple { selection, term }
        })
    }
}

impl Display for Assignment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Assignment::Simple { selection, term } => write!(f, "{} = {}", selection, term),
            Assignment::Arithmetic { assignee, lhs, op, rhs } => write!(f, "{} = {} {} {}", assignee, lhs, op, rhs),
            Assignment::Append { assignee, list, item } => {
                write!(f, "{} = {} + {}", assignee, list, item)
            }
        }
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum SimpleSelection {
    Column(Name),
    Term(Name, Term),
    Field(Name, Name),
}

impl Parse for SimpleSelection {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if let Some(res) = s.parse_if::<(_, Dot, _)>() {
            let (column, _, field) = res?;
            Self::Field(column, field)
        } else if let Some(res) = s.parse_from_if::<(Name, Brackets<Term>)>() {
            let (column, term) = res?;
            Self::Term(column, term)
        } else {
            Self::Column(s.parse::<Name>()?)
        })
    }
}

impl Display for SimpleSelection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Column(name) => name.fmt(f),
            Self::Term(name, term) => write!(f, "{}[{}]", name, term),
            Self::Field(column, field) => write!(f, "{}.{}", column, field),
        }
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub struct Condition {
    pub lhs: SimpleSelection,
    pub op: Operator,
    pub rhs: Term,
}

impl Parse for Condition {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        let (lhs, op, rhs) = s.parse()?;
        Ok(Condition { lhs, op, rhs })
    }
}

impl Display for Condition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.lhs, self.op, self.rhs)
    }
}

#[derive(ParseFromStr, Clone, Debug)]
pub enum IfClause {
    Exists,
    Conditions(Vec<Condition>),
}

impl Parse for IfClause {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        s.parse::<IF>()?;
        Ok(if s.parse_if::<EXISTS>().is_some() {
            IfClause::Exists
        } else {
            IfClause::Conditions(s.parse_from::<List<Condition, AND>>()?)
        })
    }
}
impl Peek for IfClause {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<IF>()
    }
}

impl Display for IfClause {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exists => write!(f, "IF EXISTS"),
            Self::Conditions(conditions) => {
                write!(
                    f,
                    "IF {}",
                    conditions
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(" AND ")
                )
            }
        }
    }
}

#[derive(ParseFromStr, Builder, Clone, Debug)]
pub struct DeleteStatement {
    #[builder(default)]
    pub selections: Option<Vec<SimpleSelection>>,
    pub from: FromClause,
    #[builder(default)]
    pub using: Option<Vec<UpdateParameter>>,
    pub where_clause: WhereClause,
    #[builder(default)]
    pub if_clause: Option<IfClause>,
}

impl Parse for DeleteStatement {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        s.parse::<DELETE>()?;
        let mut res = DeleteStatementBuilder::default();
        res.selections(s.parse_from::<Option<List<SimpleSelection, Comma>>>()?)
            .from(s.parse()?)
            .using(
                s.parse_from::<Option<(USING, List<UpdateParameter, AND>)>>()?
                    .map(|(_, v)| v),
            )
            .where_clause(s.parse()?)
            .if_clause(s.parse()?);
        s.parse::<Option<Semicolon>>()?;
        Ok(res
            .build()
            .map_err(|e| anyhow::anyhow!("Invalid DELETE statement: {}", e))?)
    }
}

impl Peek for DeleteStatement {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<DELETE>()
    }
}

impl Display for DeleteStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DELETE")?;
        if let Some(selections) = &self.selections {
            write!(
                f,
                " {}",
                selections.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
            )?;
        }
        write!(f, " {}", self.from)?;
        if let Some(using) = &self.using {
            write!(
                f,
                " USING {}",
                using.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
            )?;
        }
        write!(f, " {}", self.where_clause)?;
        if let Some(if_clause) = &self.if_clause {
            write!(f, " {}", if_clause)?;
        }
        Ok(())
    }
}

impl KeyspaceExt for DeleteStatement {
    fn get_keyspace(&self) -> Option<String> {
        self.from.table.keyspace.as_ref().map(|n| n.to_string())
    }

    fn set_keyspace(&mut self, keyspace: &str) {
        self.from.table.keyspace = Some(Name::Quoted(keyspace.to_string()));
    }
}

impl WhereExt for DeleteStatement {
    fn iter_where(&self) -> Option<std::slice::Iter<Relation>> {
        Some(self.where_clause.relations.iter())
    }
}

#[derive(ParseFromStr, Builder, Clone, Debug)]
pub struct BatchStatement {
    pub kind: BatchKind,
    pub using: Option<Vec<UpdateParameter>>,
    pub statements: Vec<ModificationStatement>,
}

impl BatchStatement {
    pub fn add_statement(&mut self, statement: &str) -> anyhow::Result<()> {
        self.statements
            .push(StatementStream::new(statement).parse::<ModificationStatement>()?);
        Ok(())
    }
}

impl Parse for BatchStatement {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        s.parse::<BEGIN>()?;
        let mut res = BatchStatementBuilder::default();
        res.kind(s.parse()?);
        s.parse::<BATCH>()?;
        res.using(
            s.parse_from::<Option<(USING, List<UpdateParameter, AND>)>>()?
                .map(|(_, v)| v),
        );
        let mut statements = Vec::new();
        while let Some(res) = s.parse_if::<ModificationStatement>() {
            statements.push(res?);
        }
        res.statements(statements);
        s.parse::<(APPLY, BATCH)>()?;
        s.parse::<Option<Semicolon>>()?;
        Ok(res
            .build()
            .map_err(|e| anyhow::anyhow!("Invalid BATCH statement: {}", e))?)
    }
}

impl Peek for BatchStatement {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<BEGIN>()
    }
}

impl Display for BatchStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BEGIN")?;
        match self.kind {
            BatchKind::Logged => (),
            BatchKind::Unlogged => write!(f, " UNLOGGED")?,
            BatchKind::Counter => write!(f, " COUNTER")?,
        };
        write!(f, " BATCH")?;
        if let Some(using) = &self.using {
            write!(
                f,
                " USING {}",
                using.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
            )?;
        }
        write!(
            f,
            " {}",
            self.statements
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )?;
        write!(f, " APPLY BATCH")?;
        Ok(())
    }
}

#[derive(ParseFromStr, Clone, Debug, TryInto, From)]
pub enum ModificationStatement {
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
}

impl Parse for ModificationStatement {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if let Some(keyword) = s.find::<ReservedKeyword>() {
            match keyword {
                ReservedKeyword::INSERT => Self::Insert(s.parse()?),
                ReservedKeyword::UPDATE => Self::Update(s.parse()?),
                ReservedKeyword::DELETE => Self::Delete(s.parse()?),
                _ => anyhow::bail!(
                    "Expected a data modification statement (INSERT / UPDATE / DELETE)! Found {}",
                    keyword
                ),
            }
        } else {
            anyhow::bail!("Expected a data modification statement (INSERT / UPDATE / DELETE)!")
        })
    }
}
impl Peek for ModificationStatement {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<InsertStatement>() || s.check::<UpdateStatement>() || s.check::<DeleteStatement>()
    }
}

impl Display for ModificationStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Insert(s) => s.fmt(f),
            Self::Update(s) => s.fmt(f),
            Self::Delete(s) => s.fmt(f),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BatchKind {
    Logged,
    Unlogged,
    Counter,
}

impl Parse for BatchKind {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        Ok(if s.parse_if::<UNLOGGED>().is_some() {
            BatchKind::Unlogged
        } else if s.parse_if::<COUNTER>().is_some() {
            BatchKind::Counter
        } else {
            BatchKind::Logged
        })
    }
}

#[derive(Clone, Debug)]
pub struct FromClause {
    pub table: KeyspaceQualifiedName,
}

impl Parse for FromClause {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        let (_, table) = s.parse::<(FROM, KeyspaceQualifiedName)>()?;
        Ok(FromClause { table })
    }
}

impl Peek for FromClause {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<FROM>()
    }
}

impl Display for FromClause {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FROM {}", self.table)
    }
}

#[derive(Clone, Debug)]
pub struct WhereClause {
    pub relations: Vec<Relation>,
}

impl Parse for WhereClause {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        let (_, relations) = s.parse_from::<(WHERE, List<Relation, AND>)>()?;
        Ok(WhereClause { relations })
    }
}

impl Peek for WhereClause {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<WHERE>()
    }
}

impl Display for WhereClause {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WHERE {}",
            self.relations
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(" AND ")
        )
    }
}

#[derive(Clone, Debug)]
pub struct GroupByClause {
    pub columns: Vec<Name>,
}

impl Parse for GroupByClause {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        let (_, _, columns) = s.parse_from::<(GROUP, BY, List<Name, Comma>)>()?;
        Ok(GroupByClause { columns })
    }
}

impl Peek for GroupByClause {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<(GROUP, BY)>()
    }
}

impl Display for GroupByClause {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GROUP BY {}",
            self.columns
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Clone, Debug)]
pub struct OrderingClause {
    pub columns: Vec<ColumnOrder>,
}

impl Parse for OrderingClause {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self> {
        let (_, _, columns) = s.parse_from::<(GROUP, BY, List<ColumnOrder, Comma>)>()?;
        Ok(OrderingClause { columns })
    }
}

impl Peek for OrderingClause {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<(ORDER, BY)>()
    }
}

impl Display for OrderingClause {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ORDER BY {}",
            self.columns
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Clone, Debug)]
pub enum Limit {
    Literal(i32),
    BindMarker(BindMarker),
}

impl Parse for Limit {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        if let Some(bind) = s.parse_if::<BindMarker>() {
            Ok(Limit::BindMarker(bind?))
        } else {
            Ok(Limit::Literal(s.parse::<i32>()?))
        }
    }
}

impl Peek for Limit {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<i32>() || s.check::<BindMarker>()
    }
}

impl Display for Limit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Limit::Literal(i) => i.fmt(f),
            Limit::BindMarker(b) => b.fmt(f),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ColumnDefault {
    Null,
    Unset,
}

impl Parse for ColumnDefault {
    type Output = Self;
    fn parse(s: &mut StatementStream<'_>) -> anyhow::Result<Self::Output> {
        if s.parse_if::<NULL>().is_some() {
            Ok(ColumnDefault::Null)
        } else if s.parse_if::<UNSET>().is_some() {
            Ok(ColumnDefault::Unset)
        } else {
            anyhow::bail!("Invalid column default: {}", s.parse_from::<Token>()?)
        }
    }
}

impl Peek for ColumnDefault {
    fn peek(s: StatementStream<'_>) -> bool {
        s.check::<NULL>() || s.check::<UNSET>()
    }
}

impl Display for ColumnDefault {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnDefault::Null => write!(f, "NULL"),
            ColumnDefault::Unset => write!(f, "UNSET"),
        }
    }
}

mod test {
    #[allow(unused)]
    use super::*;

    #[test]
    fn test_parse_select() {
        let statement = "SELECT movie, director \
            FROM movies.NerdMovies \
            WHERE main_actor = 'Nathan Fillion' \
            AND year > 2011 \
            AND year <= 2012";
        let mut stream = StatementStream::new(statement);
        let select_statement = stream.parse::<SelectStatement>().unwrap();
        // println!("{:#?}", select_statement);
        // println!("{}", select_statement);
        assert_eq!(statement.to_string().replace("\n", ""), select_statement.to_string())
    }

    #[test]
    fn test_parse_insert() {
        let statement = "INSERT INTO NerdMovies (movie, director, main_actor, year) \
            VALUES ('Serenity', 'Joss Whedon', 'Nathan Fillion', 2005) \
            IF NOT EXISTS USING TTL 86400";
        let mut stream = StatementStream::new(statement);
        let insert_statement = stream.parse::<InsertStatement>().unwrap();
        // println!("{:#?}", insert_statement);
        // println!("{}", insert_statement);
        assert_eq!(statement.to_string().replace("\n", ""), insert_statement.to_string())
    }

    #[test]
    fn test_parse_update() {
        let statement = "UPDATE NerdMovies \
            SET director = 'Joss Whedon', main_actor = 'Nathan Fillion' \
            WHERE movie = 'Serenity' \
            IF EXISTS";
        let mut stream = StatementStream::new(statement);
        let update_statement = stream.parse::<UpdateStatement>().unwrap();
        // println!("{:#?}", update_statement);
        // println!("{}", update_statement);
        assert_eq!(statement.to_string().replace("\n", ""), update_statement.to_string())
    }

    #[test]
    fn test_parse_delete() {
        let statement = "DELETE FROM NerdMovies \
            WHERE movie = 'Serenity' \
            IF EXISTS";
        let mut stream = StatementStream::new(statement);
        let delete_statement = stream.parse::<DeleteStatement>().unwrap();
        // println!("{:#?}", delete_statement);
        // println!("{}", delete_statement);
        assert_eq!(statement.to_string().replace("\n", ""), delete_statement.to_string())
    }

    #[test]
    fn test_parse_batch() {
        let statement = "BEGIN BATCH \
            INSERT INTO NerdMovies (movie, director, main_actor, year) \
            VALUES ('Serenity', 'Joss Whedon', 'Nathan Fillion', 2005) \
            IF NOT EXISTS USING TTL 86400; \
            UPDATE NerdMovies \
            SET director = 'Joss Whedon', main_actor = 'Nathan Fillion' \
            WHERE movie = 'Serenity' \
            IF EXISTS; \
            DELETE FROM NerdMovies \
            WHERE movie = 'Serenity' \
            IF EXISTS \
            APPLY BATCH";
        let mut stream = StatementStream::new(statement);
        let batch_statement = stream.parse::<BatchStatement>().unwrap();
        // println!("{:#?}", batch_statement);
        // println!("{}", batch_statement);
        assert_eq!(statement.to_string().replace("\n", ""), batch_statement.to_string())
    }
}
