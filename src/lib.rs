//! sqlx 代码生成器
//!
//! 指定数据库和表名，生成对应的struct
//!
#![allow(unused_variables)]

use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self},
    io::Write,
};

use async_trait::async_trait;
use clap::{Parser, Subcommand};
use heck::ToUpperCamelCase;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sqlx::{Any, MySql, Pool};
use template::{MODEL_TEMPLATE, MOD_TEMPLATE};

mod mysql;
mod postgres;
mod sqlite;
mod template;

lazy_static! {
    pub static ref KEYWORDS: Vec<&'static str> = {
        // Rust1.70 关键字
        vec![
            "as", "async", "await","break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false",
            "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
            "ref", "return", "Self", "self", "static", "struct", "super", "trait", "true", "type","union",
            "unsafe", "use", "where", "while", "abstract",  "become", "box", "do",
             "final", "macro", "override", "priv", "try", "typeof", "unsized", "virtual",
            "yield",
        ]
    };
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Table {
    pub schema: String,
    pub name: String,
    pub comment: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Column {
    pub schema: Option<String>,
    pub table_name: Option<String>,
    pub name: Option<String>,
    pub default: Option<String>,
    pub max_length: Option<i64>,
    pub is_nullable: Option<String>,
    pub column_type: Option<String>,
    pub comment: Option<String>,

    // 对应 Rust 类型
    pub field_type: Option<String>,
    pub multi_world: Option<bool>,
}

// #[async_trait]
// pub trait Database {
//     type DB: sqlx::Database;
//     /// 获取指定表信息
//     async fn tables(
//         &self,
//         pool: &Pool<Self::DB>,
//         table_names: &[&str],
//     ) -> anyhow::Result<Vec<Table>>;

//     /// 获取指定表的字段
//     async fn columns(
//         &self,
//         pool: &Pool<Self::DB>,
//         table_names: &[&str],
//     ) -> anyhow::Result<Vec<Column>>;
// }

/// 驱动类型
#[derive(Debug, Clone, Copy, Subcommand)]
pub enum Driver {
    Sqlite,
    Mysql,
    Postgres,
}

/// 代码生成器
/// Driver::Sqlite      sqlite://test.sqlite
/// Driver::Mysql       mysql://root:root@localhost:3306/test
/// Driver::Postgres    postgres://root:root@localhost:5432/test
///
#[derive(Parser, Debug)]
#[command(author, version, about,long_about = None)]
pub struct Generator {
    /// 数据库驱动
    #[command(subcommand)]
    pub driver: Driver,
    /// 数据库账号
    #[clap(short)]
    pub username: String,
    /// 数据库密码
    #[clap(short)]
    pub password: String,
    /// 数据库地址
    #[clap(short('H'))]
    pub host: String,
    /// 数据库端口号
    #[clap(short('P'))]
    pub port: u16,
    /// 指定的数据库名称
    #[clap(short('D'))]
    pub database: String,
    /// 代码生成的路径
    #[clap(default_value = "target/models/")]
    pub path: String,
    /// 指定要生成代码的表名，多个用英文逗号拼接，为空表示全部
    #[clap(short('t'), long, default_value = "")]
    pub table_names: String,
}

impl Display for Generator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"
            driver_url: {}
            path: {}
            table_names: {}
           "#,
            self.driver_url(),
            self.path,
            self.table_names
        )
    }
}

impl Generator {
    pub fn driver_url(&self) -> String {
        match self.driver {
            Driver::Sqlite => format!("sqlite://{}", self.database),
            Driver::Mysql => format!(
                "mysql://{}:{}@{}:{}/{}",
                self.username, self.password, self.host, self.port, self.database
            ),
            Driver::Postgres => format!(
                "postgres://{}:{}@{}:{}/{}",
                self.username, self.password, self.host, self.port, self.database
            ),
        }
    }

    // pub async fn db<DB>(&self) -> anyhow::Result<Box<Pool<DB>>>
    // where
    //     DB: sqlx::Database,
    // {
    //     match self.driver {
    //         Driver::Sqlite => Ok(Box::new(
    //             sqlx::SqlitePool::connect(&self.driver_url()).await?,
    //         )),
    //         Driver::Mysql => Ok(Box::new(
    //             sqlx::MySqlPool::connect(&self.driver_url()).await?,
    //         )),
    //         Driver::Postgres => Ok(Box::new(sqlx::PgPool::connect(&self.driver_url()).await?)),
    //     }
    // }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.deal_path();
        self.generator().await?;
        Ok(())
    }

    ///  处理路径，当路径不以 / 结尾时，自动添加 /
    fn deal_path(&mut self) {
        if !self.path.is_empty() && !self.path.ends_with('/') {
            self.path.push('/')
        }
    }

    /// 生成器
    ///
    /// ```text
    /// path: 指定生成路径
    /// table_names: 指定生成的表明，为空则生成全部
    /// ```
    pub async fn generator(&self) -> anyhow::Result<()> {
        println!("{self}");
        println!("====== start ======");

        // let db = self.db().await?;

        // TODO 什么是 trait object?
        // let tobj: &dyn Database<DB = dyn sqlx::Database> = {
        //     match self.driver {
        //         Driver::Sqlite => &sqlite::Sqlite,
        //         Driver::Mysql => &mysql::Mysql,
        //         Driver::Postgres => &postgres::Postgres,
        //     }
        // };

        let table_names = self
            .table_names
            .split(',')
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>();

        // let tables = tobj.tables(&db, &table_names).await?;
        // let tables_columns = tobj.columns(&db, &table_names).await?;

        let (tables, tables_columns) = match self.driver {
            Driver::Sqlite => {
                let pool = sqlx::SqlitePool::connect(&self.driver_url()).await?;
                let tables = sqlite::tables(&pool, &table_names).await?;
                let tables_columns = sqlite::columns(&pool, &table_names).await?;
                (tables, tables_columns)
            }
            Driver::Mysql => {
                let pool = sqlx::MySqlPool::connect(&self.driver_url()).await?;
                let tables = mysql::tables(&pool, &table_names).await?;
                let tables_columns = mysql::columns(&pool, &table_names).await?;
                (tables, tables_columns)
            }
            Driver::Postgres => {
                let pool = sqlx::PgPool::connect(&self.driver_url()).await?;
                let tables = postgres::tables(&self.database, &pool, &table_names).await?;
                let tables_columns = postgres::columns(&self.database, &pool, &table_names).await?;
                (tables, tables_columns)
            }
        };
        if tables.is_empty() {
            println!("tables is empty");
            return Ok(());
        }

        if tables_columns.is_empty() {
            println!("table columns is empty");
            return Ok(());
        }

        // 将tables转换为map，K：表名，V：表信息
        let table_map: HashMap<String, Table> =
            tables.into_iter().map(|t| (t.name.to_owned(), t)).collect();

        // 组装表信息和表列信息，K：表名，V：表列信息
        // FIXME：有没有办法直接将Vec分组，类似Java的Collectors.groupby
        let table_column_map =
            table_map
                .keys()
                .fold(HashMap::new(), |mut table_column_map, table_name| {
                    table_column_map.insert(
                        table_name,
                        tables_columns
                            .iter()
                            .filter(|table_column| {
                                Some(table_name.clone()) == table_column.table_name
                            })
                            .collect::<Vec<_>>(),
                    );
                    table_column_map
                });

        // 创建生成目录
        fs::create_dir_all(&self.path)?;

        // 创建模板引擎
        let mut tera = tera::Tera::default();
        table_map.iter().for_each(|(table_name, table)| {
            let column = table_column_map.get(&table_name);
            // 创建上下文
            let mut ctx = tera::Context::new();
            ctx.insert("struct_name", &table_name.to_upper_camel_case());
            ctx.insert("table", &table);
            let mut has_columns = false;
            if let Some(columns) = column {
                has_columns = !columns.is_empty();
                ctx.insert("column_num", &columns.len());
                ctx.insert("columns", &columns);
                ctx.insert(
                    "column_names",
                    &columns
                        .iter()
                        .map(|c| c.name.clone().unwrap())
                        .collect::<Vec<String>>()
                        .join(","),
                );
            }
            ctx.insert("has_columns", &has_columns);

            // 渲染模板
            let render_string = tera.render_str(MODEL_TEMPLATE, &ctx).expect("渲染模板错误");
            // 创建文件
            let file_name = format!("{}{}.rs", self.path, &table_name);
            let mut tf = fs::File::create(&file_name).expect("创建文件失败");
            tf.write_all(render_string.as_bytes())
                .expect("写入数据错误");

            println!("the {} has been generated", &file_name);
        });

        let mut ctx = tera::Context::new();
        ctx.insert("table_names", &table_map);
        let render_string = tera.render_str(MOD_TEMPLATE, &ctx)?;

        // 创建 mod.rs 文件
        let mod_file_name = format!("{}mod.rs", self.path);
        let mut tf = fs::File::create(&mod_file_name).expect("创建文件失败");
        tf.write_all(render_string.as_bytes())?;

        println!("the {} has been generated", &mod_file_name);
        println!("====== over ======");
        Ok(())
    }
}

/// 判断字段名称是否是由多个单词组成
pub fn multi_world(name: &str) -> bool {
    name.contains(|c| c == '_' || c == '-')
}

/// 列名是否为Rust关键字，若为关键字，则需要在其前加 r#
pub fn column_keywords(name: &str) -> String {
    if KEYWORDS.contains(&name) {
        format!("r#{}", name)
    } else {
        name.to_string()
    }
}
