use super::WhiteBoardStorage;
use crate::whiteboard::WhiteBoardData;
use mongodb::{ bson::Document, Collection };
use redis::{ Client, Commands, RedisError, AsyncCommands };
use super::mongo::MongoDBStorage;
use futures::Future;
use serde::{ Serialize, Deserialize };
use serde_json::Value;
use std::time::{ SystemTime, UNIX_EPOCH };

pub struct RedisStorage {
    project_id: u32,
    redis_cli: Client,
    mongo_collection: Collection<Document>,

    data: Option<WhiteBoardData>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RedisSavingData {
    project_id: u32,
    data: WhiteBoardData,
}

impl RedisSavingData {
    fn new(project_id: u32, data: WhiteBoardData) -> Self {
        return Self {
            project_id,
            data,
        };
    }
}

impl RedisStorage {
    pub fn new(project_id: u32, redis_cli: Client, mongo_collection: Collection<Document>) -> Self {
        return Self {
            project_id,
            redis_cli,
            mongo_collection,
            data: None,
        };
    }

    fn get_mongo_storage(&self) -> MongoDBStorage {
        return MongoDBStorage::new(self.get_project_id(), self.mongo_collection.clone(), None);
    }

    async fn load_whiteboard_data(&self) -> WhiteBoardData {
        println!("Loading whiteboard data");
        let mut con = self.redis_cli.get_multiplexed_async_connection();

        let key = self.get_cache_key();

        let cached_value: Option<String> = con.await.unwrap().get(key).await.unwrap();

        if cached_value.is_none() {
            println!("cache miss");
            let mut mongo_storage = self.get_mongo_storage();
            return mongo_storage.get_whiteboard().await.clone();
        } else {
            println!("cache hit");
            let saved_data: RedisSavingData = serde_json
                ::from_str(cached_value.unwrap().as_str())
                .unwrap();
            return saved_data.data;
        }
    }

    fn get_cache_key(&self) -> String {
        return format!("whiteboard:{}", self.get_project_id());
    }

    async fn save_data_in_cache(&self, data: RedisSavingData) {
        println!("Saving whiteboard data");

        let mut con = self.redis_cli.get_multiplexed_async_connection();

        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local value = ARGV[1]
            local timestamp = ARGV[2]
            
            -- Set the key with the provided value and an expiration of 3600 seconds (1 hour)
            redis.call("SET", key, value, "EX", 9)
            
            -- Store the key and timestamp in the hash set "updated_whiteboards"
            redis.call("HSET", "updated_whiteboards", key, timestamp)            
            return "OK"
        "#
        );

        let key = self.get_cache_key();
        let string_data = serde_json::to_string(&data).unwrap();
        let result: String = script
            .key(key)
            .arg(string_data)
            .arg(Self::get_current_time_ns())
            .invoke_async(&mut con.await.unwrap()).await
            .unwrap();
        println!("{}", result);
    }

    fn get_current_time_ns() -> String {
        let now = SystemTime::now();
        let since_the_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        return since_the_epoch.as_nanos().to_string();
    }
}

impl WhiteBoardStorage for RedisStorage {
    fn get_project_id(&self) -> u32 {
        todo!()
    }

    async fn get_saving_data(&mut self) -> String {
        return "".to_string();
    }

    async fn get_whiteboard(&mut self) -> &WhiteBoardData {
        self.data = Some(self.load_whiteboard_data().await);
        return self.data.as_ref().unwrap();
    }

    async fn save(&mut self) {
        return;
    }

    async fn set_whiteboard(&mut self, value: WhiteBoardData) {
        let saving_data = RedisSavingData::new(self.project_id, value);
        self.save_data_in_cache(saving_data).await;
    }
}
