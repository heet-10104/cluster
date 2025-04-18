use crate::subapps::loadbalancer::Api;
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