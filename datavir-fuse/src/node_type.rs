use core::str::FromStr;
use rusqlite::types::FromSql;
use rusqlite::types::FromSqlError;
use rusqlite::types::ToSqlOutput;
use rusqlite::ToSql;

#[derive(Debug, PartialEq, Eq)]
pub enum NodeType {
    Special,
    FilterDir,
    FilterSpecFile,
    FilterBundleLink,
    BundleRoot,
    BundleElement,
    OrgDir,
}

impl NodeType {
    fn as_str(&self) -> &'static str {
        match self {
            NodeType::Special => "S",
            NodeType::FilterDir => "FD",
            NodeType::FilterSpecFile => "FF",
            NodeType::FilterBundleLink => "FL",
            NodeType::BundleRoot => "BR",
            NodeType::BundleElement => "BE",
            NodeType::OrgDir => "OD",
        }
    }
    #[allow(dead_code)]
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

impl ToSql for NodeType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for NodeType {
    fn column_result(
        val: rusqlite::types::ValueRef<'_>,
    ) -> Result<Self, rusqlite::types::FromSqlError> {
        match NodeType::from_str(val.as_str().unwrap()) {
            Ok(v) => Ok(v),
            Err(err) => Err(FromSqlError::Other(Box::new(err))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InvalidNodeTypeError {
    got: String,
}

impl std::fmt::Display for InvalidNodeTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "invalid NodeType: {}", self.got)
    }
}

impl std::error::Error for InvalidNodeTypeError {}

impl FromStr for NodeType {
    type Err = InvalidNodeTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S" => Ok(NodeType::Special),
            "FD" => Ok(NodeType::FilterDir),
            "FF" => Ok(NodeType::FilterSpecFile),
            "FL" => Ok(NodeType::FilterBundleLink),
            "BR" => Ok(NodeType::BundleRoot),
            "BE" => Ok(NodeType::BundleElement),
            "OD" => Ok(NodeType::OrgDir),
            _ => Err(InvalidNodeTypeError { got: s.to_string() }),
        }
    }
}
