use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool};

pub struct Sqlite;

#[async_trait]
impl super::Database for Sqlite {
    async fn tables(
        &self,
        pool: &Pool<sqlx::mysql::MySql>,
        table_names: &[&str],
    ) -> anyhow::Result<Vec<super::Table>> {
        let mut sql = r#"
    SELECT type, name, tbl_name, rootpage, sql
    FROM sqlite_master
    WHERE type = 'table'
        "#
        .to_string();

        if !table_names.is_empty() {
            sql.push_str("AND (1 = 2");
            table_names
                .iter()
                .map(|&t| format!("OR name = '{t}'"))
                .collect::<Vec<_>>()
                .join(" ");

            sql.push_str(&format!(
                "AND FIND_IN_SET(TABLE_NAME, {})",
                table_names.join(",")
            ));
            sql.push(')');
        }
        sql.push_str("ORDER by rootpage;");

        Ok(sqlx::query_as::<_, Table>(&sql)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|t| t.into())
            .collect::<Vec<_>>())
    }
    async fn columns(
        &self,
        pool: &Pool<sqlx::mysql::MySql>,
        table_names: &[&str],
    ) -> anyhow::Result<Vec<super::TableColumn>> {
        let sql = "pragma table_info('#{table_names}');";

        let mut cols = vec![];
        for table_name in table_names.iter() {
            let columns = sqlx::query_as::<_, TableColumn>(sql)
                .fetch_all(pool)
                .await?;

            println!("== {:?}", columns);
            let mut columns = columns
                .iter()
                .map(|c| c.into())
                .collect::<Vec<super::TableColumn>>()
                .iter_mut()
                .map(|c| {
                    c.table_name = Some(table_name.to_string());
                    c.to_owned()
                })
                .collect::<Vec<_>>();
            cols.append(&mut columns);
        }
        Ok(cols)
    }
}

/// 表信息来自 sqlite_master
#[derive(Default, Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub struct Table {
    /// 项目的类型：table，index，view，trigger
    r#type: Option<String>,
    /// 项目的名称
    name: Option<String>,
    /// 所从属的表名，如索引所在的表名
    tbl_name: Option<String>,
    /// 项目在数据库页中存储的编号
    rootpage: Option<i64>,
    /// SQL语句
    sql: Option<String>,
}

/// 表列信息
#[derive(Default, Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all(deserialize = "SCREAMING_SNAKE_CASE"))]
pub struct TableColumn {
    /// 列ID
    cid: Option<u64>,
    /// 列名
    name: Option<String>,
    /// 类型：如：varchar(50)  int
    r#type: Option<String>,
    /// 是否为空：1-不为空，0-为空
    notnull: Option<u8>,
    dflt_value: Option<String>,
    /// 是否为主键：1-主键，0-非主键
    pk: Option<u8>,
}

impl From<Table> for super::Table {
    fn from(t: Table) -> Self {
        Self {
            name: t.name,
            ..Default::default()
        }
    }
}

impl From<&Table> for super::Table {
    fn from(t: &Table) -> Self {
        Self {
            name: t.name.clone(),
            ..Default::default()
        }
    }
}

impl From<TableColumn> for super::TableColumn {
    fn from(col: TableColumn) -> Self {
        let ty = sqlite_type(col.r#type.clone().unwrap().as_str());
        Self {
            name: Some(super::column_keywords(col.name.clone().unwrap().as_str())),
            default: col.dflt_value,
            is_nullable: {
                if let Some(is_null) = col.notnull {
                    if is_null == 1 {
                        Some("NotNull".to_string())
                    } else {
                        Some("Null".to_string())
                    }
                } else {
                    None
                }
            },
            column_type: col.r#type.clone(),
            field_type: Some(sqlite_to_rust(ty.0.as_str()).into()),
            multi_world: Some(super::multi_world(col.name.unwrap().as_str())),
            ..Default::default()
        }
    }
}

impl From<&TableColumn> for super::TableColumn {
    fn from(col: &TableColumn) -> Self {
        let ty = sqlite_type(col.r#type.clone().unwrap().as_str());
        Self {
            name: Some(super::column_keywords(col.name.clone().unwrap().as_str())),
            default: col.dflt_value.clone(),
            is_nullable: {
                if let Some(is_null) = col.notnull {
                    if is_null == 1 {
                        Some("NotNull".to_string())
                    } else {
                        Some("Null".to_string())
                    }
                } else {
                    None
                }
            },
            column_type: col.r#type.clone(),
            field_type: Some(sqlite_to_rust(ty.0.as_str()).into()),
            multi_world: Some(super::multi_world(col.name.clone().unwrap().as_str())),
            max_length: Some(255),
            comment: col.name.clone(),
            ..Default::default()
        }
    }
}

/// Rust type             SQLite type(s)
/// bool                    BOOLEAN
/// i8                      INTEGER
/// i16                     INTEGER
/// i32                     INTEGER
/// i64                     BIGINT, INT8
/// u8                      INTEGER
/// u16                     INTEGER
/// u32                     INTEGER
/// f32                     REAL
/// f64                     REAL
/// &str, String            TEXT
/// &[u8], Vec<u8>          BLOB
///
/// time::PrimitiveDateTime DATETIME
/// time::OffsetDateTime    DATETIME
/// time::Date              DATE
/// time::Time              TIME
///
/// Sqlite类型转换为Rust类型
fn sqlite_to_rust(ty: &str) -> &str {
    match ty.to_uppercase().as_str() {
        "BOOLEAN" => "bool",
        "INTEGER" => "i32",
        "BIGINT" | "INT8" => "i64",
        "REAL" => "f64",
        "BLOB" => "Vec<u8>",
        "DATE" => "time::Date",
        "TIME" => "time::Time",
        "DATETIME" => "time::OffsetDateTime",
        _ => "String",
    }
}

/// 根据sqlite字段类型截取类型和长度
/// date、datetime、int没有长度
/// varchar有长度
fn sqlite_type(t: &str) -> (String, Option<u16>) {
    let rg = Regex::new("^(.*)\\((\\d+)\\)$").unwrap();
    if let Some(caps) = rg.captures(t) {
        (
            caps.get(1).map_or("".to_string(), |tt| tt.as_str().into()),
            caps.get(2)
                .map_or(Some(0), |l| Some(l.as_str().parse::<u16>().unwrap_or(0))),
        )
    } else {
        (t.to_string(), None)
    }
}
