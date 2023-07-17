use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool};

pub struct Mysql;

#[async_trait]
impl super::Database for Mysql {
    async fn tables(
        &self,
        pool: &Pool<sqlx::mysql::MySql>,
        table_names: &[&str],
    ) -> Vec<super::Table> {
        let mut sql = r#"
    SELECT
        TABLE_CATALOG as table_catalog,
        TABLE_SCHEMA as table_schema,
        TABLE_NAME as table_name,
        TABLE_TYPE as table_type,
        `ENGINE` as engine,
        VERSION as version,
        ROW_FORMAT as row_format,
        TABLE_ROWS as table_rows,
        AVG_ROW_LENGTH as avg_row_length,
        DATA_LENGTH as data_length,
        MAX_DATA_LENGTH as max_data_length,
        INDEX_LENGTH as index_length,
        DATA_FREE as data_free,
        AUTO_INCREMENT as auto_increment,
        CREATE_TIME as create_time,
        UPDATE_TIME as update_time,
        CHECK_TIME as check_time,
        TABLE_COLLATION as table_collation,
        `CHECKSUM` as checksum,
        CREATE_OPTIONS as create_options,
        TABLE_COMMENT as table_comment
    FROM
        information_schema.`TABLES`
    WHERE
        TABLE_SCHEMA = (
        SELECT
            DATABASE ())
        "#
        .to_string();

        if !table_names.is_empty() {
            sql.push_str(&format!(
                "AND FIND_IN_SET(TABLE_NAME, '{}')",
                table_names.join(",")
            ));
        }

        sql.push_str("ORDER BY CREATE_TIME;");

        sqlx::query_as::<_, Table>(&sql)
            .fetch_all(pool)
            .await
            .unwrap()
            .into_iter()
            .map(|t| t.into())
            .collect::<Vec<_>>()
    }
    async fn columns(
        &self,
        pool: &Pool<sqlx::mysql::MySql>,
        table_names: &[&str],
    ) -> Vec<super::TableColumn> {
        let mut sql = r#"
    SELECT
        TABLE_CATALOG as table_catalog,
        TABLE_SCHEMA as table_schema,
        TABLE_NAME as table_name,
        COLUMN_NAME as column_name,
        ORDINAL_POSITION as ordinal_position,
        COLUMN_DEFAULT as column_default,
        IS_NULLABLE as is_nullable,
        DATA_TYPE as data_type,
        CHARACTER_MAXIMUM_LENGTH character_maximum_length,
        CHARACTER_OCTET_LENGTH as character_octet_length,
        NUMERIC_PRECISION as numeric_precision,
        NUMERIC_SCALE as numeric_scale,
        DATETIME_PRECISION as datetime_precision,
        CHARACTER_SET_NAME as character_set_name,
        COLLATION_NAME as collation_name,
        COLUMN_TYPE as column_type,
        COLUMN_KEY as column_key,
        EXTRA as extra,
        `PRIVILEGES` as privileges,
        COLUMN_COMMENT column_comment,
        GENERATION_EXPRESSION as generation_expression,
        SRS_ID as srs_id
    FROM
        information_schema.COLUMNS
    WHERE
        TABLE_SCHEMA = (
        SELECT
            DATABASE ())
        "#
        .to_string();

        if !table_names.is_empty() {
            sql.push_str(&format!(
                "AND FIND_IN_SET(TABLE_NAME, '{}')",
                table_names.join(",")
            ));
        }
        sql.push_str("ORDER BY ORDINAL_POSITION;");

        sqlx::query_as::<_, TableColumn>(&sql)
            .fetch_all(pool)
            .await
            .unwrap()
            .into_iter()
            .map(|col| col.into())
            .collect::<Vec<super::TableColumn>>()
    }
}

#[derive(Default, Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub struct Table {
    pub table_catalog: Option<String>,
    pub table_schema: Option<String>,
    pub table_name: Option<String>,
    /// enum('BASE TABLE','VIEW','SYSTEM VIEW')
    pub table_type: Option<String>,
    pub engine: Option<String>,
    pub version: Option<i64>,
    /// enum('Fixed','Dynamic','Compressed','Redundant','Compact','Paged')
    pub row_format: Option<String>,
    pub table_rows: Option<u64>,
    pub avg_row_length: Option<u64>,
    pub data_length: Option<u64>,
    pub max_data_length: Option<u64>,
    pub index_length: Option<u64>,
    pub data_free: Option<u64>,
    pub auto_increment: Option<u64>,
    // pub create_time: Option<u64>,
    // pub update_time: Option<u64>,
    // pub check_time: Option<u64>,
    pub table_collation: Option<String>,
    pub checksum: Option<i64>,
    pub create_options: Option<String>,
    pub table_comment: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub struct TableColumn {
    pub table_catalog: Option<String>,
    pub table_schema: Option<String>,
    pub table_name: Option<String>,
    pub column_name: Option<String>,
    pub ordinal_position: Option<u32>,
    pub column_default: Option<String>,
    pub is_nullable: Option<String>,
    pub data_type: Option<String>,
    pub character_maximum_length: Option<i64>,
    pub character_octet_length: Option<i64>,
    pub numeric_precision: Option<u64>,
    pub numeric_scale: Option<u64>,
    pub datetime_precision: Option<u32>,
    pub character_set_name: Option<String>,
    pub collation_name: Option<String>,
    pub column_type: Option<String>,
    /// enum('','PRI','UNI','MUL')
    pub column_key: Option<String>,
    pub extra: Option<String>,
    pub privileges: Option<String>,
    pub column_comment: Option<String>,
    pub generation_expression: Option<String>,
    pub srs_id: Option<u32>,
}

impl From<Table> for super::Table {
    fn from(t: Table) -> Self {
        Self {
            schema: t.table_schema,
            name: t.table_name,
            comment: t.table_comment,
        }
    }
}

impl From<TableColumn> for super::TableColumn {
    fn from(c: TableColumn) -> Self {
        let ty = mysql_2_rust(&c.column_type.clone().unwrap_or_default().to_uppercase());
        Self {
            schema: c.table_schema.clone(),
            table_name: c.table_name.clone(),
            name: Some(super::column_keywords(
                c.column_name.clone().unwrap().as_str(),
            )),
            default: c.column_default.clone(),
            is_nullable: {
                let ft = ty.clone();
                if ft.contains("Time") {
                    Some("Yes".to_string())
                } else {
                    c.is_nullable.clone()
                }
            },
            column_type: c.column_type.clone(),
            comment: c.column_comment.clone(),
            field_type: Some(ty),
            multi_world: Some({
                c.column_name
                    .clone()
                    .unwrap()
                    .contains(|c| c == '_' || c == '-')
            }),
            max_length: c.character_maximum_length,
        }
    }
}

/// Mysql 类型转换为Rust对应类型
fn mysql_2_rust(t: &str) -> String {
    match t.to_uppercase().as_str() {
        "TINYINT(1)" | "BOOL" | "BOOLEAN" => "bool".to_string(),
        "TINYINT" => "i8".to_string(),
        "TINYINT UNSIGNED" => "u8".to_string(),
        "SMALLINT" => "i16".to_string(),
        "SMALLINT UNSIGNED" => "u16".to_string(),
        "MEDIUMINT" => "i32".to_string(),
        "MEDIUMINT UNSIGNED" => "u32".to_string(),
        "INT" | "INTEGER" => "i32".to_string(),
        "INT UNSIGNED" | "INTEGER UNSIGNED" => "u32".to_string(),
        "BIGINT" => "i64".to_string(),
        "BIGINT UNSIGNED" | "SERIAL" => "u64".to_string(),
        "DECIMAL" => "bigdecimal::BigDecimal".to_string(),
        "DOUBLE" | "DOUBLE PRECISION" | "NUMERIC" => "f64".to_string(),
        "FLOAT" => "f32".to_string(),
        "BIT" => "u8".to_string(),
        "DATE" => "time::Date".to_string(),
        "TIME" => "time::Time".to_string(),
        "DATETIME" => "time::PrimitiveDateTime".to_string(),
        "TIMESTAMP" => "time::offsetDateTime".to_string(),
        "YEAR" => "time::Date".to_string(),
        "CHAR" | "VARCHAR" | "BINARY" | "VARBINARY" | "TINYBLOB" | "TINYTEXT" | "BLOB" | "TEXT"
        | "MEDIUMBLOB" | "MEDIUMTEXT" | "LONGBLOB" | "LONGTEXT" | "ENUM" | "SET" => {
            "String".to_string()
        }
        // GEOMETRY, POINT, LINESTRING, POLYGON
        // MULTIPOINT, MULTILINESTRING, MULTIPOLYGON, GEOMETRYCOLLECTION
        "JSON" => "serde_json:JsonValue".to_string(),

        _ => "String".to_string(),
    }
}
