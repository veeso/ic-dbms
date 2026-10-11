//! Untyped select results and borrowed row views for the canister API.

use candid::CandidType;
use serde::{Deserialize, Serialize};
use wasm_dbms_api::prelude::{JoinColumnDef, JoinResultSet, JoinRow, Value};

/// Result of an untyped select, retaining the query's native row representation.
///
/// Queries without joins return [`Self::Raw`]; queries with joins return
/// [`Self::Joined`]. Joined results store column metadata once, including
/// when no rows match. Use [`Self::iter`] to borrow either representation.
///
/// # Examples
///
/// ```
/// use ic_dbms_api::prelude::SelectOutput;
///
/// let result = SelectOutput::Raw(vec![]);
/// assert!(result.is_empty());
/// for row in result.iter() {
///     for (column, value) in row.iter() {
///         println!("{name}: {value:?}", name = column.name);
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub enum SelectOutput {
    /// Single-table rows containing serializable column-value pairs.
    Raw(Vec<Vec<(JoinColumnDef, Value)>>),
    /// Joined rows sharing column metadata with source table names.
    Joined(JoinResultSet),
}

impl SelectOutput {
    /// Borrows raw rows, or returns `None` for joined results.
    pub fn as_raw(&self) -> Option<&[Vec<(JoinColumnDef, Value)>]> {
        match self {
            Self::Raw(rows) => Some(rows),
            Self::Joined(_) => None,
        }
    }

    /// Borrows the joined result set, or returns `None` for raw results.
    pub fn as_joined(&self) -> Option<&JoinResultSet> {
        match self {
            Self::Raw(_) => None,
            Self::Joined(rows) => Some(rows),
        }
    }

    /// Returns the number of rows.
    pub fn len(&self) -> usize {
        match self {
            Self::Raw(rows) => rows.len(),
            Self::Joined(rows) => rows.len(),
        }
    }

    /// Returns `true` when no rows matched the query.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterates over borrowed row views without cloning columns or values.
    pub fn iter(&self) -> impl Iterator<Item = SelectRow<'_>> {
        self.as_raw()
            .into_iter()
            .flatten()
            .map(|row| SelectRow::Raw(row))
            .chain(
                self.as_joined()
                    .into_iter()
                    .flat_map(JoinResultSet::iter)
                    .map(SelectRow::Joined),
            )
    }
}

/// Borrowed row from either representation of a [`SelectOutput`].
#[derive(Debug, Clone, Copy)]
pub enum SelectRow<'a> {
    /// Column-value pairs from a single-table query.
    Raw(&'a [(JoinColumnDef, Value)]),
    /// Values paired with the joined result set's shared columns.
    Joined(JoinRow<'a>),
}

impl<'a> SelectRow<'a> {
    /// Iterates over borrowed column-value pairs in output order.
    pub fn iter(&self) -> impl Iterator<Item = (&'a JoinColumnDef, &'a Value)> + use<'a> {
        let (raw, joined) = match self {
            Self::Raw(row) => (Some(*row), None),
            Self::Joined(row) => (None, Some(*row)),
        };
        raw.into_iter()
            .flatten()
            .map(|(column, value)| (column, value))
            .chain(joined.into_iter().flat_map(|row| row.iter()))
    }
}
