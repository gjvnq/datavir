use core::str::FromStr;
use rusqlite::types::FromSql;
use rusqlite::types::FromSqlError;
use rusqlite::types::ToSqlOutput;
use rusqlite::ToSql;

#[derive(Debug, PartialEq, Eq)]
pub enum ObjectType {
    Reserved,
    Filter,
    BundleRoot,
    BundleElement,
    OrgDir,
}

impl ObjectType {
    fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Reserved => "R",
            ObjectType::Filter => "F",
            ObjectType::BundleRoot => "BR",
            ObjectType::BundleElement => "BE",
            ObjectType::OrgDir => "O",
        }
    }
    #[allow(dead_code)]
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

impl ToSql for ObjectType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for ObjectType {
    fn column_result(
        val: rusqlite::types::ValueRef<'_>,
    ) -> Result<Self, rusqlite::types::FromSqlError> {
        match ObjectType::from_str(val.as_str().unwrap()) {
            Ok(v) => Ok(v),
            Err(err) => Err(FromSqlError::Other(Box::new(err))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InvalidObjectTypeError {
    got: String,
}

impl std::fmt::Display for InvalidObjectTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "invalid ObjectType: {}", self.got)
    }
}

impl std::error::Error for InvalidObjectTypeError {}

impl FromStr for ObjectType {
    type Err = InvalidObjectTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "R" => Ok(ObjectType::Reserved),
            "F" => Ok(ObjectType::Filter),
            "BR" => Ok(ObjectType::BundleRoot),
            "BE" => Ok(ObjectType::BundleElement),
            "O" => Ok(ObjectType::OrgDir),
            _ => Err(InvalidObjectTypeError { got: s.to_string() }),
        }
    }
}
