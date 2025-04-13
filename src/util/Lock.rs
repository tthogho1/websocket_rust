// util/Lock.rs
use redis::{self, Commands};
use uuid::Uuid;

pub struct RedisLock<'a> {
    client: &'a redis::Client,
    lock_key: &'a str,
    owner_token: String,
}

impl<'a> RedisLock<'a> {
    pub fn new(client: &'a redis::Client, lock_key: &'a str) -> Self {
        Self {
            client,
            lock_key,
            owner_token: Uuid::new_v4().to_string(),
        }
    }

    pub fn lock(&self, ttl_sec: u64) -> redis::RedisResult<bool> {
        let mut con = self.client.get_connection()?;
        let res: Option<String> = redis::cmd("SET")
            .arg(self.lock_key)
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
        script.key(self.lock_key).arg(&self.owner_token).invoke(&mut con)?;
        Ok(())
    }
}

impl<'a> Drop for RedisLock<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.unlock() {
            // output error to console 
            eprintln!("Failed to unlock Redis key {}: {}", self.lock_key, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::{Client, Commands};
    use std::thread;
    use std::time::Duration;

    fn setup_client() -> Client {
        Client::open("redis://127.0.0.1/").unwrap()
    }

    #[test]
    fn test_lock_acquired() {
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
