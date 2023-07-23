/// mod.rs 文件模板
pub const MOD_TEMPLATE: &str = r#"
use async_static::async_static;
use sqlx::{MySql, Pool};

{% for table_name, _ in table_names %}
mod {{table_name}};
pub use {{table_name}}::*;
{% endfor %}

async_static! {
    static ref DB: Pool<MySql> = pool().await;
}

async fn pool() -> anyhow::Result<Pool<MySql>> {
    Ok(sqlx::mysql::MySqlPool::connect("mysql://root:123qwe@127.0.0.1/mine").await?)
}

/// 分页返回封装
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PageRes<T> {
    page: i64,
    page_size: i64,
    total: i64,
    list: Vec<T>,
    first: bool,
    last: bool,
    has_next: bool,
    has_pre: bool,
    total_pages: i64,
}

impl<T> std::default::Default for PageRes<T> {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 15,
            total: 0,
            list: vec![],
            first: true,
            last: false,
            has_next: false,
            has_pre: false,
            total_pages: 0,
        }
    }
}

impl<T> PageRes<T>
where
    T: Serialize + Clone,
{
    pub fn new(total: i64, mut page: i64, page_size: i64, list: &[T]) -> Self {
        if page <= 0 {
            page = 1;
        }
        let total_pages = (total as f64 / page_size as f64).ceil() as i64;
        Self {
            page,
            page_size,
            total,
            list: list.iter().cloned().collect::<Vec<_>>(),
            first: page == 1,
            last: page == total_pages,
            has_next: page < total_pages,
            has_pre: page > 1,
            total_pages,
        }
    }
}
"#;

/// 模型模板
pub const MODEL_TEMPLATE: &str = r#"
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use super::DB;
use crate::{error::MineError, result::MineResult};

/// {{table.comment}}
#[derive(
    Debug,
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    FromRow,
    Validate,
)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct {{ struct_name }} { {% if has_columns %}{% for column in columns %}
    /// {{column.comment}}
    {%if column.field_type == "String" -%}#[validate(length(max = {{column.max_length}}))]{%- endif%}
    pub {{column.name}}: Option<{{column.field_type}}>,{% endfor %}{% endif %}
}

impl std::fmt::Display for {{ struct_name }} {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::json!(self))
    }
}

impl {{ struct_name }} {
    fn table_name() -> String {
        "{{table.name}}".to_string()
    }

    fn columns() -> String {
        "{{ column_names }}".to_string()
    }

    pub async fn fetch_by_id(id: u64) -> MineResult<Self> {
        let sql = format!(
            "select {} from {} where id = ?",
            Self::columns(),
            Self::table_name()
        );
        sqlx::query_as::<_, Self>(&sql)
            .bind(id)
            .fetch_one(DB.await)
            .await
            .map_err(|e| {
                log::error!("{e}");
                MineError::SqlError
            })
    }

    pub async fn fetch_all(req: &{{ struct_name }}Req) -> MineResult<Vec<Self>> {
        let mut sql = format!("select {} from {}", Self::columns(), Self::table_name());

        let mut where_sql = " WHERE 1=1 ".to_string();

        {% if has_columns %}{% for column in columns %}
        if let Some({{column.name}}) = &req.{{column.name}} {
        {%if column.field_type == "String"%}
            where_sql.push_str(&format!(" and {} like '%{}%' ",  "{{column.name}}", {{column.name}}));
        {%else%}
            where_sql.push_str(&format!(" and {} = {} ",  "{{column.name}}", {{column.name}}));
        {%endif%}
        }
        {% endfor %}{% endif %}

        sql.push_str(&where_sql);

        sqlx::query_as::<_, Self>(&sql)
            .fetch_all(DB.await)
            .await
            .map_err(|e| {
                log::error!("{e}");
                MineError::SqlError
            })
    }

    pub async fn insert(&mut self) -> MineResult<Self> {
        let sql = format!(
            "INSERT INTO {} ({}) VALUES({})",
            Self::table_name(),
            Self::columns(),
            "{% for column in columns %}?,{% endfor %}".trim_end_matches(',')
        );
        let id = sqlx::query(&sql)
            .bind(&self.id)
            {% if has_columns %}{% for column in columns %}
            .bind(&self.{{column.name}})
            {% endfor %}{% endif %}
            .execute(DB.await)
            .await
            .map_err(|e| {
                log::error!("{e}");
                MineError::SqlError
            })?
            .last_insert_id();
        Self::fetch_by_id(id).await
    }

    pub async fn update(&mut self) -> MineResult<bool> {
        let sql = format!(
            "UPDATE {} set account = ?, set {} where id = ?",
            Self::table_name(),
            "{% for column in columns %}{{column.name}} = ?,{% endfor %}".trim_end_matches(',')
        );
        sqlx::query(&sql)
            {% if has_columns %}{% for column in columns %}
            .bind(&self.{{ column.name }})
            {% endfor %}{% endif %}
            .bind(&self.id)
            .execute(DB.await)
            .await
            .map_err(|e| {
                log::error!("{e}");
                MineError::SqlError
            })
            .map(|r| r.rows_affected() > 0)
    }

    pub async fn delete(&self) -> MineResult<bool> {
        let sql = format!("DELETE FROM {} WHERE id = ?", Self::table_name());
        sqlx::query(&sql)
            .bind(self.id)
            .execute(DB.await)
            .await
            .map_err(|e| {
                log::error!("{e}");
                MineError::SqlError
            })
            .map(|r| r.rows_affected() > 0)
    }

    async fn count(where_sql: &str) -> MineResult<(i64,)> {
        let count_sql = format!(
            "SELECT count(*) FROM {} WHERE {}",
            Self::table_name(),
            where_sql
        );

        sqlx::query_as::<_, (i64,)>(&count_sql)
            .fetch_one(DB.await)
            .await
            .map_err(|e| {
                log::error!("{e}");
                MineError::SqlError
            })
    }

    pub async fn page(req: &{{ struct_name }}Req) -> MineResult<super::PageRes<Self>> {
        let mut where_sql = " 1 = 1 ".to_string();
        {% if has_columns %}{% for column in columns %}
        if let Some({{column.name}}) = &req.{{column.name}} {
            {%if column.field_type == "String"%}
                where_sql.push_str(&format!(" and {} like '%{}%' ",  "{{column.name}}", {{column.name}}));
            {%else%}
                where_sql.push_str(&format!(" and {} = {} ",  "{{column.name}}", {{column.name}}));
            {%endif%}
        }
        {% endfor %}{% endif %}

        let (count,) = Self::count(&where_sql).await?;
        
        let page_size = req.page_size.unwrap_or(20);
        let mut page = req.page.unwrap_or(0) - 1;
        if page < 0 {
            page = 0;
        }
        where_sql.push_str(&format!(" LIMIT {}, {} ", page * page_size, page_size));

        let res = match count > 0 {
            true => {
                let mut sql = format!(
                    "SELECT {} FROM {} WHERE ",
                    Self::columns(),
                    Self::table_name()
                );

                sql.push_str(&where_sql);
                sqlx::query_as::<_, Self>(&sql)
                    .fetch_all(DB.await)
                    .await
                    .map_err(|e| {
                        log::error!("{e}");
                        MineError::SqlError
                    })?
            }
            false => Vec::new(),
        };
        Ok(super::PageRes::new(count, page, page_size, &res))
    }
}


/// {{table.comment}}
#[derive(
    Debug,
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    FromRow,
    Validate,
)]
pub struct {{ struct_name }}Req { 
    pub time_type: Option<>,
    /// 开始时间
    pub start_at: Option<u64>,
    /// 结束时间
    pub end_at: Option<u64>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,

    {% if has_columns %}{% for column in columns %}
    /// {{column.comment}}
    pub {{column.name}}: Option<{{column.field_type}}>,{% endfor %}{% endif %}
}
"#;
