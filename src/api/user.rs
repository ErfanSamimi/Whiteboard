use chrono::Utc;
use crate::user::User;
use serde::Serialize;
use super::common::AppState;
use super::auth::Claims;
use axum::extract::State;
use axum::Json;
use sqlx::{FromRow, PgPool};

#[derive(Serialize, Debug, FromRow)]
pub struct UserListOutputView {
    id: i64,
    username: String,
    email: String,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<&User> for UserListOutputView {
    fn from(value: &User) -> Self {
        return Self {
            id: value.get_id().unwrap(),
            username: value.get_username().clone(), 
            email: value.get_email().clone(), 
            created_at: value.get_created_at(), 
            updated_at: value.get_updated_at()
         }
    }
}

impl UserListOutputView{
    pub async fn get_active_users(pool: &PgPool,) -> Result<Vec<Self>, sqlx::Error> {

        let user = sqlx::query_as!(
            Self,
            r#"
            SELECT id, username, email, created_at, updated_at
            FROM users
            WHERE is_active = true
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(user)
    }
}

pub async fn user_list_view(claims: Claims, State(state): State<AppState>,) -> Json<Vec<UserListOutputView>> {
    println!("{}", claims);
    let users = UserListOutputView::get_active_users(&state.pool).await.unwrap();
    return Json(users);
}
