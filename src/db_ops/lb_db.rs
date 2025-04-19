use crate::subapps::loadbalancer::Api;
use axum::http::StatusCode;
use sqlx::PgPool;

pub async fn insert_apis(apis: &Vec<Api>, db: &PgPool) -> Result<(), sqlx::Error> {
    for api in apis.iter() {
        sqlx::query!("insert into api_info (api) values ($1)", api.url)
            .execute(db)
            .await?;
    }

    Ok(())
}

pub async fn update_hit(path: &str, db: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("update api_info set hits = hits+1 where api = ($1)", path)
        .execute(db)
        .await?;

    Ok(())
}
pub async fn update_error_code(
    path: &str,
    status: &StatusCode,
    db: &PgPool,
) -> Result<(), sqlx::Error> {
    let row = sqlx::query!("select errors from api_info where api = ($1)", path)
        .fetch_one(db)
        .await?;

    let errors = row.errors;
    match errors {
        Some(mut errors) => {
            errors.push(status.as_u16() as i32);
            sqlx::query!("update api_info set errors = ($1) where api = ($2)", &errors, path)
                .execute(db)
                .await?;
        }
        None => {}
    }

    Ok(())
}
