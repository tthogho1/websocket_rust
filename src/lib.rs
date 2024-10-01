use bb8_redis::{bb8, RedisConnectionManager};
//use redis::AsyncCommands;

pub async fn create_pool(redis_url: &str) -> Result<bb8::Pool<RedisConnectionManager>, Box<dyn std::error::Error>> {
    let manager = RedisConnectionManager::new(redis_url)?;
    let pool = bb8::Pool::builder()
        .max_size(15)
        .build(manager)
        .await?;
    Ok(pool)
}

pub async fn get_connection(pool: &bb8::Pool<RedisConnectionManager>) -> Result<bb8::PooledConnection<'_, RedisConnectionManager>, Box<dyn std::error::Error>> {
    let conn = pool.get().await?;
    Ok(conn)
}