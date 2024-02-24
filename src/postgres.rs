use nutype::nutype;
use once_cell::sync::Lazy;
use sqlx::Column;
use std::str::FromStr;

pub static LAST_UPDATED_AT_COL: Lazy<TableColumn> =
    Lazy::new(|| TableColumn::from_str("last_updated_at").unwrap());

#[nutype(
    sanitize(trim, lowercase),
    validate(not_empty),
    derive(
        Debug,
        Display,
        Clone,
        Deref,
        Borrow,
        FromStr,
        Into,
        TryFrom,
        AsRef,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
    )
)]
pub struct TableName(String);

#[nutype(
    sanitize(trim, lowercase),
    validate(not_empty),
    derive(
        Debug,
        Display,
        Clone,
        Deref,
        Borrow,
        FromStr,
        Into,
        TryFrom,
        AsRef,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
    )
)]
pub struct TableColumn(String);

impl<R, DB> sqlx::ColumnIndex<R> for TableColumn
where
    DB: sqlx::Database,
    R: sqlx::Row<Database = DB>,
{
    fn index(&self, row: &R) -> Result<usize, sqlx::Error> {
        row.columns()
            .iter()
            .enumerate()
            .find(|(_, c)| self.as_str() == c.name())
            .map(|(i, _)| i)
            .ok_or_else(|| sqlx::Error::ColumnNotFound(self.to_string()))
    }
}
