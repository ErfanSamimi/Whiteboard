
use sqlx::FromRow;

use chrono::{DateTime, Utc};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use rand_core::OsRng;
use rand_core::RngCore;
use base64::{engine::general_purpose, Engine as _};
use sqlx::PgPool;
use subtle::ConstantTimeEq;

fn hash_password(password: &str, iterations: u32, salt_length: usize) ->  String {
    let mut salt = vec![0u8; salt_length];
    OsRng.fill_bytes(&mut salt); // Generate a random salt

    let mut hash = [0u8; 32]; // 32 bytes for SHA-256
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, iterations, &mut hash);


    let salt_b64 = general_purpose::STANDARD.encode(&salt);
    let hash_b64 = general_purpose::STANDARD.encode(&hash);

    format!("pbkdf2_sha256${}${}${}", iterations, salt_b64, hash_b64)
}

fn verify_password(password: &str, stored_hash: &str) -> bool {
    let parts: Vec<&str> = stored_hash.split('$').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2_sha256" {
        return false;
    }

    let iterations = match parts[1].parse::<u32>() {
        Ok(i) => i,
        Err(_) => return false,
    };

    let salt = match general_purpose::STANDARD.decode(parts[2]) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let expected_hash = match general_purpose::STANDARD.decode(parts[3]) {
        Ok(h) => h,
        Err(_) => return false,
    };

    let mut derived_hash = vec![0u8; expected_hash.len()];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, iterations, &mut derived_hash);
    // Constant-time comparison using subtle
    derived_hash.ct_eq(&expected_hash).into()
    // constant_time_eq::constant_time_eq(&derived_hash, &expected_hash)
}


#[derive(Debug, FromRow)]
pub struct User {
    id: Option<i64>,
    password: String,
    last_login: Option<DateTime<Utc>>, // Nullable field
    is_superuser: bool,
    username: String,
    first_name: String,
    last_name: String,
    email: String,
    is_staff: bool,
    is_active: bool,
    date_joined: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    pub fn create_new(
        username: String,
        raw_password: String,
        firstname: String,
        lastname:String,
        email: String,
        is_staff: bool,
        is_active: bool,
        is_superuser: bool,

    ) -> Self{
        let mut new_user = Self{
            id: None,
            password: "Password".to_string(),
            last_login: None,
            is_superuser: is_superuser,
            username: username,
            first_name: firstname,
            last_name: lastname,
            email: email,
            is_staff: is_staff,
            is_active: is_active,
            date_joined: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now()
        };
        new_user.set_password(raw_password);
        return new_user;
    }



    pub async fn get_by_id(pool: &PgPool, user_id: i64) -> Result<Option<Self>, sqlx::Error> {

        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, password, last_login, is_superuser, username,
                   first_name, last_name, email, is_staff, is_active,
                   date_joined, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    pub async fn get_active_users(pool: &PgPool,) -> Result<Vec<Self>, sqlx::Error> {

        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, password, last_login, is_superuser, username,
                   first_name, last_name, email, is_staff, is_active,
                   date_joined, created_at, updated_at
            FROM users
            WHERE is_active = true
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(user)
    }


    pub async fn get_by_username(pool: &PgPool, username: String) -> Result<Option<Self>, sqlx::Error> {

        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, password, last_login, is_superuser, username,
                   first_name, last_name, email, is_staff, is_active,
                   date_joined, created_at, updated_at
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }


    pub async fn authenticate(pool: &PgPool, username: String, raw_password: String) -> Option<Self> {
        let user = Self::get_by_username(pool, username).await.unwrap();
        if user.is_none(){
            return None;
        }
        
        let user = user.unwrap();
        println!("checking user passsword...");
        if user.check_password(raw_password){
            return Some(user);
        }
        else {
            return None;
        }
    }


    
    pub fn get_id(&self) -> Option<i64>{
        self.id
    }



    pub fn get_username(&self) -> &String{
        &self.username
    }

    pub fn get_email(&self) -> &String{
        &self.email
    }

    pub fn get_created_at(&self) -> DateTime<Utc>{
        self.created_at
    }

    pub fn get_updated_at(&self) -> DateTime<Utc>{
        self.updated_at
    }
    
    pub fn set_password(&mut self, raw_password:String) {
        let hashed_passwd = hash_password(&raw_password, 7200, 16);
        self.password = hashed_passwd
    }


    pub fn check_password(&self, raw_password:String) -> bool {
        return verify_password(&raw_password, &self.password);
    }

    pub async fn create_row(&mut self, pool: &PgPool) -> Result<(), String> {
        if self.id.is_some() {
            return Err("Can not create an already exists row. The User Id must be None to create new row.".to_string());
        }

        let created_user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (password, is_superuser, username, first_name, last_name, email, 
                               is_staff, is_active, date_joined, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, password, last_login, is_superuser, username, first_name, 
                      last_name, email, is_staff, is_active, date_joined, created_at, updated_at
            "#,
            self.password, self.is_superuser, self.username, self.first_name, self.last_name, self.email, 
            self.is_staff, self.is_active, self.date_joined, self.created_at, Utc::now()
        )
        .fetch_one(pool)
        .await.unwrap();


        *self = created_user;
        return Ok(());

    }


    pub async fn update(&mut self, pool: &PgPool) -> Result<(), String> {
        if self.id.is_none() {
            return Err("Can not update the user. The User Id must not none to update the DB row.".to_string());
        }

        let now = Utc::now();

        let updated = sqlx::query_as!(
            User,
            r#"
            UPDATE users SET
                password = $1,
                last_login = $2,
                is_superuser = $3,
                username = $4,
                first_name = $5,
                last_name = $6,
                email = $7,
                is_staff = $8,
                is_active = $9,
                updated_at = $10
            WHERE id = $11
            RETURNING id, password, last_login, is_superuser, username,
                      first_name, last_name, email, is_staff, is_active,
                      date_joined, created_at, updated_at
            "#,
            self.password,
            self.last_login,
            self.is_superuser,
            self.username,
            self.first_name,
            self.last_name,
            self.email,
            self.is_staff,
            self.is_active,
            now,
            self.id
        )
        .fetch_one(pool)
        .await.unwrap();

        *self = updated;
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    extern crate dotenv;
    use dotenv::dotenv;

    use std::env;
    use sqlx::PgPool;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "supersecret123";
        let hash = hash_password(password, 100_000, 16);
        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrongpassword", &hash));
    }

    #[test]
    fn test_invalid_hash_format() {
        let bad_hash = "pbkdf2_sha256$abc$def"; // malformed
        assert!(!verify_password("password", bad_hash));
    }

    #[tokio::test]
    async fn test_create_new_user_struct() {
        let user = User::create_new(
            "alice".to_string(),
            "securepass".to_string(),
            "Alice".to_string(),
            "Smith".to_string(),
            "alice@example.com".to_string(),
            true,
            true,
            false,
        );

        assert_eq!(user.username, "alice");
        assert!(user.check_password("securepass".to_string()));
    }

    async fn get_test_pool() -> PgPool {
        dotenv().ok();
        let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .expect("Failed to connect to DB")
    }

    #[tokio::test]
    async fn test_create_and_get_user_in_db() {
        let pool = get_test_pool().await;
        let mut user = User::create_new(
            "bob".to_string(),
            "pass123".to_string(),
            "Bob".to_string(),
            "Builder".to_string(),
            "bob@example.com".to_string(),
            true,
            true,
            false,
        );

        user.create_row(&pool).await.expect("Failed to create user");

        assert!(user.id.is_some());

        let retrieved = User::get_by_id(&pool, user.id.unwrap())
            .await
            .expect("Fetch failed");

        assert!(retrieved.is_some());
        let retrieved_user = retrieved.unwrap();
        assert_eq!(retrieved_user.username, "bob");
    }

    #[tokio::test]
    async fn test_update_user() {
        let pool = get_test_pool().await;
        let mut user = User::create_new(
            "update_test".to_string(),
            "old_pass".to_string(),
            "Up".to_string(),
            "Date".to_string(),
            "update@example.com".to_string(),
            false,
            true,
            false,
        );
        user.create_row(&pool).await.unwrap();
        user.first_name = "Updated".to_string();
        user.set_password("new_pass".to_string());

        user.update(&pool).await.expect("Failed to update");

        let fetched = User::get_by_id(&pool, user.id.unwrap()).await.unwrap().unwrap();
        assert_eq!(fetched.first_name, "Updated");
        assert!(fetched.check_password("new_pass".to_string()));
    }
}
