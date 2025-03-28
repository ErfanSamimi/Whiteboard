use super::common::{ WsEventReceive, WsEventSend };
use crate::api::auth::validate_token;
use crate::api::common::AppState;
use crate::api::project::permissions::is_collaborator;

use rand::{distributions::Alphanumeric, Rng};
use std::{collections::HashMap, sync::Arc};
use redis::{Commands, Script, Client, Connection};

pub struct RedisActions {
    conn: Connection,
}

impl RedisActions {
    pub fn new(client: &Client) -> redis::RedisResult<Self> {
        let conn = client.get_connection()?;

        Ok(Self { conn })
    }
    pub fn set_multiple_keys_atomic(
        &mut self,
        key_value_pairs: HashMap<String, String>,
        ttl: Option<u32>,
    ) -> redis::RedisResult<()> {
        let lua_script = Script::new(r#"
            for i = 1, #KEYS do
                redis.call('SET', KEYS[i], ARGV[i])
                if tonumber(ARGV[#ARGV]) then
                    redis.call('EXPIRE', KEYS[i], tonumber(ARGV[#ARGV]))
                end
            end
            return true
        "#);
    
        // Prepare keys and args
        let mut keys = Vec::new();
        let mut args = Vec::new();
    
        for (key, value) in &key_value_pairs {
            keys.push(key);
            args.push(value.clone());  // clone to store the value
        }
    
        // TTL as the last argument
        if let Some(ttl_val) = ttl {
            args.push(ttl_val.to_string());  // store the owned String
        } else {
            args.push("nil".to_string());  // ensure owned String
        }
    
        // Create an invocation and apply keys/args
        let mut invocation = lua_script.prepare_invoke();
        for key in keys {
            invocation.key(&key);
        }
        for arg in args {
            invocation.arg(&arg);
        }
    
        invocation.invoke(&mut self.conn)?;
        Ok(())
    }
    
    pub fn remove_key(&mut self, key: &str) -> redis::RedisResult<bool> {
        let deleted: i32 = self.conn.del(key)?;
        Ok(deleted > 0)
    }


    pub fn key_exists(&mut self, key: &str) -> redis::RedisResult<bool> {
        let exists: i32 = self.conn.exists(key)?;
        Ok(exists > 0)
    }

    pub fn get_key_or_raise(&mut self, key: &str) -> redis::RedisResult<String> {
        if !self.key_exists(key)? {
            Err(redis::RedisError::from((redis::ErrorKind::TypeError, "Key not found")))
        } else {
            self.conn.get(key)
        }
    }
}

pub struct WSAuthenticatedUsers {
    room_name: String,
    redis_actions: RedisActions,
}

impl WSAuthenticatedUsers {
    pub fn new(room_name: &str, redis_cli: Arc<redis::Client>) -> Self {
        Self {
            room_name: room_name.to_string(),
            redis_actions:RedisActions::new(&redis_cli).unwrap(),
        }
    }

    fn get_base_key(&self) -> String {
        format!("ws_auth_{}__", self.room_name)
    }

    fn get_token_key(&self, token: &str) -> String {
        format!("{}token_{}", self.get_base_key(), token)
    }

    fn get_user_key(&self, user_id: &str) -> String {
        format!("{}user_{}", self.get_base_key(), user_id)
    }

    pub fn is_authenticated(&mut self, token: &str) -> Option<i32> {
        let key = self.get_token_key(token);
        match self.redis_actions.get_key_or_raise(&key) {
            Ok(val) => val.parse::<i32>().ok(),
            Err(_) => None,
        }
    }

    pub fn add_auth_user(&mut self, token: &str, user_id: &str) {
        let token_key = self.get_token_key(token);
        let user_key = self.get_user_key(user_id);

        if let Ok(old_token) = self.redis_actions.get_key_or_raise(&user_key) {
            let old_token_key = self.get_token_key(&old_token);
            let _ = self.redis_actions.remove_key(&old_token_key);
        }

        let mut key_value_pairs = HashMap::new();
        key_value_pairs.insert(token_key, user_id.to_string());
        key_value_pairs.insert(user_key, token.to_string());

        let _ = self.redis_actions.set_multiple_keys_atomic(key_value_pairs, Some(3600));
    }
}


fn generate_random_string(n: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}

pub async fn authorize(project_id: i64, state: &AppState, event: &WsEventReceive, ws_auth_users: &mut WSAuthenticatedUsers) -> WsEventSend {
    match event {
        WsEventReceive::AUTH { token } => {
            let validation_result = validate_token(&token.as_str());
            match validation_result {
                Ok(claims) => {
                    let perm = is_collaborator(project_id, state, &claims).await;
                    match perm {
                        Ok(_) => {
                            let ws_token = claims.get_user_id().to_string();
                            ws_auth_users.add_auth_user(ws_token.as_str(), claims.get_user_id().to_string().as_str());
                            return WsEventSend::AuthSuccess {
                                message: "Authenticated successfully".to_string(),
                                user_token: ws_token,
                            };
                        },
                        Err(_) => {
                            return WsEventSend::Error { message: "No access to this project".to_string() };
                        }
                    }
                }
                Err(_) => {
                    return WsEventSend::Error { message: "Invalid token".to_string() };
                }
            }
        }
        _ => {
            return WsEventSend::Error { message: "Invalid event type for authorizing".to_string() };
        }
    }
}
