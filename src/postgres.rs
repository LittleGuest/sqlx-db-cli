use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool};

#[derive(Default, Debug, Serialize, Deserialize, FromRow)]
struct Table {
    table_catalog: String,
    table_schema: String,
    table_name: String,
}

#[derive(Default, Debug, Serialize, Deserialize, FromRow)]
struct TableColumn {
    table_catalog: String,
    table_schema: String,
    table_name: String,
    column_name: String,
    ordinal_position: i32,
    column_default: Option<String>,
    is_nullable: String,
    data_type: String,
    character_maximum_length: Option<i32>,
}

impl From<Table> for super::Table {
    fn from(t: Table) -> Self {
        Self {
            schema: t.table_schema,
            name: t.table_name.clone(),
            comment: t.table_name,
        }
    }
}

impl From<TableColumn> for super::Column {
    fn from(c: TableColumn) -> Self {
        let ty = t2t(&c.data_type.clone().to_uppercase()).to_string();
        Self {
            schema: Some(c.table_schema.clone()),
            table_name: Some(c.table_name.clone()),
            name: Some(super::column_keywords(c.column_name.clone().as_str())),
            default: c.column_default.clone(),
            is_nullable: {
                if ty.contains("Time") {
                    Some("Yes".to_string())
                } else {
                    Some(c.is_nullable)
                }
            },
            column_type: Some(c.data_type),
            comment: Some("".to_string()),
            field_type: Some(ty),
            multi_world: Some(c.column_name.clone().contains(|c| c == '_' || c == '-')),
            max_length: {
                if let Some(l) = c.character_maximum_length {
                    Some(l as i64)
                } else {
                    Some(50)
                }
            },
        }
    }
}

/// Rust type            Postgres type(s)
/// bool                    BOOL
/// i8                      “CHAR”
/// i16                     SMALLINT, SMALLSERIAL, INT2
/// i32                     INT, SERIAL, INT4
/// i64                     BIGINT, BIGSERIAL, INT8
/// f32                     REAL, FLOAT4
/// f64                     DOUBLE PRECISION, FLOAT8
/// &str, String            VARCHAR, CHAR(N), TEXT, NAME
/// &[u8], Vec<u8>          BYTEA
/// ()                      VOID
/// PgInterval              INTERVAL
/// PgRange<T>              INT8RANGE, INT4RANGE, TSRANGE, TSTZRANGE, DATERANGE, NUMRANGE
/// PgMoney                 MONEY
/// PgLTree                 LTREE
/// PgLQuery                LQUERY
///
/// bigdecimal::BigDecimal  NUMERIC
///
/// time::PrimitiveDateTime TIMESTAMP
/// time::OffsetDateTime    TIMESTAMPTZ
/// time::Date              DATE
/// time::Time              TIME
/// [PgTimeTz]              TIMETZ
///
/// uuid::Uuid              UUID
///
/// ipnetwork::IpNetwork    INET, CIDR
/// std::net::IpAddr        INET, CIDR
///
/// mac_address::MacAddress MACADDR
///
/// bit_vec::BitVec         BIT, VARBIT
///
/// serde_json::Value       JSON, JSONB
///
/// PostgreSQL 类型转换为Rust对应类型
fn t2t(ty: &str) -> &str {
    match ty.to_uppercase().as_str() {
        "BOOL" => "bool",
        "CHAR" => "i8",
        "SMALLINT" | "SMALLSERIAL" | "INT2" => "i16",
        "INT" | "SERIAL" | "INT4" => "i32",
        "BIGINT" | "BIGSERIAL" | "INT8" => "i64",
        "REAL" | "FLOAT4" => "f32",
        "DOUBLE PRECISION" | "FLOAT8" => "f64",
        "BYTEA" => "Vec<u8>",
        "VOID" => "()",
        "INTERVAL" => "sqlx_postgres::types::PgInterval",
        "INT8RANGE" | "INT4RANGE" | "TSRANGE" | "TSTZRANGE" | "DATERANGE" | "NUMRANGE" => {
            "sqlx_postgres::types::PgRange<T> "
        }
        "MONEY" => "sqlx_postgres::types::PgMoney",
        "LTREE" => "sqlx_postgres::types::PgLTree",
        "LQUERY" => "sqlx_postgres::types::PgLQuery",
        "YEAR" => "time::Date",
        "DATE" => "time::Date",
        "TIME" => "time::Time",
        "TIMESTAMP" => "time::PrimitiveDateTime",
        "TIMESTAMPTZ" => "time::OffsetDateTime",
        "TIMETZ" => "sqlx_postgres::types::PgTimeTz",
        "NUMERIC" => "bigdecimal::BigDecimal",
        "JSON" | "JSONB" => "serde_json:JsonValue",
        "UUID" => "uuid::Uuid",
        "INET" | "CIDR" => "std::net::IpAddr",
        "MACADDR" => "mac_address::MacAddress",
        "BIT" | "VARBIT" => "bit_vec::BitVec",
        _ => "String",
    }
}

pub async fn tables(
    database: &str,
    pool: &Pool<sqlx::Postgres>,
    table_names: &[&str],
) -> anyhow::Result<Vec<super::Table>> {
    let mut sql = format!("SELECT table_catalog, table_schema, table_name FROM information_schema.tables WHERE table_catalog = '{database}' and table_schema = 'public'");

    if !table_names.is_empty() {
        sql.push_str(&format!("and table_name in ('{}')", table_names.join(",")));
    }

    Ok(sqlx::query_as::<_, Table>(&sql)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|t| t.into())
        .collect::<Vec<_>>())
}

pub async fn columns(
    database: &str,
    pool: &Pool<sqlx::Postgres>,
    table_names: &[&str],
) -> anyhow::Result<Vec<super::Column>> {
    let mut sql = format!("select table_catalog, table_schema, table_name, column_name, ordinal_position, column_default, is_nullable, data_type, character_maximum_length from information_schema.columns where table_catalog = '{database}' and table_schema = 'public'");

    if !table_names.is_empty() {
        sql.push_str(&format!("and table_name in ('{}')", table_names.join(",")));
    }

    Ok(sqlx::query_as::<_, TableColumn>(&sql)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|col| col.into())
        .collect::<Vec<super::Column>>())
}
