// util/Lock.rs
use redis::{self, Commands, Client};
use uuid::Uuid;
use std::time::Duration;
use std::thread::sleep;

pub struct RedisLock {
    client: redis::Client,
    lock_key: String,
    owner_token: String,
}

impl RedisLock {
    pub fn new(client: &redis::Client, lock_key: &str) -> Self {
        Self {
            client: client.clone(),
            lock_key: lock_key.to_string(),
            owner_token: Uuid::new_v4().to_string(),
        }
    }

    // Create a new RedisLock with a client and lock_key
    pub fn new_with_client(redis_url: &str, lock_key: &str) -> redis::RedisResult<RedisLock> {
        let client = Client::open(redis_url)?;
        Ok(RedisLock::new(&client, lock_key))
    }

    pub fn lock(&self, ttl_sec: u64) -> redis::RedisResult<bool> {
        print!("Attempting to acquire lock on key: {}\n", self.lock_key);
        let timeout = Duration::from_secs(30);
        let mut retries = 10;
        let mut con;

        loop {
            println!("Attempting to get connection to Redis...!!!");
            match self.client.get_connection_with_timeout(timeout) {
                Ok(conn) => {
                    con = conn;
                    break;
                }
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(e.into()); // リトライ上限に達したらエラーを返す
                    }
                    sleep(Duration::from_secs(5)); // 1秒待機
                }
            }
        };

        //let mut con = self.client.get_connection()?;
        let res: Option<String> = redis::cmd("SET")
            .arg(&self.lock_key)
            .arg(&self.owner_token)
            .arg("NX")
            .arg("EX")
            .arg(ttl_sec)
            .query(&mut con)?;
        Ok(res.is_some())
    }

    pub fn unlock(&self) -> redis::RedisResult<()> {
        let mut con = self.client.get_connection()?;
        let script = redis::Script::new(
            r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
                return redis.call("DEL", KEYS[1])
            else
                return 0
            end
        "#,
        );
        script.key(&self.lock_key).arg(&self.owner_token).invoke(&mut con)?;
        Ok(())
    }
}

impl Drop for RedisLock {
    fn drop(&mut self) {
        if let Err(e) = self.unlock() {
            // output error to console 
            eprintln!("Failed to unlock Redis key {}: {}", self.lock_key, e);
        }else{
            println!("Successfully unlocked Redis key {}", self.lock_key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use redis::{Client, Commands};
    use std::thread;
    use std::time::Duration;

    fn setup_client() -> Client {
        dotenv().ok();
        print!("Loading environment variables from .env file\n");
        let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
        // print redis_url to console
        println!("Connecting to Redis at {}", redis_url);
        Client::open(redis_url).unwrap()
    }

    #[test]
    fn test_lock_acquired() {
        print!("Testing lock acquisition\n");
        let client = setup_client();
        let lock = RedisLock::new(&client, "test_lock");

        assert!(lock.lock(10).unwrap());
        
        // Clean up
        lock.unlock().unwrap();
    }

    #[test]
    fn test_lock_conflict() {
        let client = setup_client();
        let lock_1 = RedisLock::new(&client, "test_lock");
        let lock_2 = RedisLock::new(&client, "test_lock");

        assert!(lock_1.lock(10).unwrap());
        assert!(!lock_2.lock(10).unwrap());

        // Clean up
        lock_1.unlock().unwrap();
    }

    #[test]
    fn test_lock_expiry() {
        let client = setup_client();
        let lock = RedisLock::new(&client, "test_lock");

        assert!(lock.lock(2).unwrap()); // Lock expires after 2 seconds
        thread::sleep(Duration::from_secs(3)); // Sleep for longer than lock expiry
        assert!(lock.lock(2).unwrap()); // Should be able to acquire again

        // Clean up
        lock.unlock().unwrap();
    }

    #[test]
    fn test_unlock() {
        let client = setup_client();
        let lock = RedisLock::new(&client, "test_lock");

        assert!(lock.lock(10).unwrap());
        lock.unlock().unwrap();
        
        // Verify that key no longer exists
        let mut con = client.get_connection().unwrap();
        let res: Option<String> = con.get("test_lock").unwrap();
        assert!(res.is_none());
    }
}
