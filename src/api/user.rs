use axum::response::Response;
use chrono::Utc;
use http::StatusCode;
use crate::user::User;
use serde::{ Deserialize, Serialize };
use super::common::AppState;
use super::auth::Claims;
use axum::{ extract::State, response::IntoResponse };
use axum::Json;
use sqlx::{ FromRow, PgPool };
use serde_json::json;

#[derive(Deserialize, Debug)]
pub struct UserRegisterData {
    username: String,
    email: String,
    password: String,
    confirm_password: String,
}

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
            updated_at: value.get_updated_at(),
        };
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

#[derive(Debug)]
pub enum RegisterError {
    UsernameExists,
    PasswordConfirmation,
    InternalServerError,
}

impl IntoResponse for RegisterError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            RegisterError::UsernameExists => (StatusCode::BAD_REQUEST, "Username exists"),
            RegisterError::PasswordConfirmation =>
                (StatusCode::BAD_REQUEST, "Passwords does not match"),
            RegisterError::InternalServerError =>
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, we're fix it as soon as possible :)",
                ),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub async fn user_list_view(
    claims: Claims,
    State(state): State<AppState>
) -> Json<Vec<UserListOutputView>> {
    println!("{}", claims);
    let users = UserListOutputView::get_active_users(&state.pg_pool).await.unwrap();
    return Json(users);
}

pub async fn user_register_view(
    State(state): State<AppState>,
    Json(payload): Json<UserRegisterData>
) -> Result<Json<UserListOutputView>, RegisterError> {
    if payload.password != payload.confirm_password {
        return Err(RegisterError::PasswordConfirmation);
    }

    let valid_username = match User::get_by_username(&state.pg_pool, payload.username.clone()).await {
        Ok(user) => {
            match user {
                Some(_) => false,
                None => true,
            }
        }
        Err(_) => {
            return Err(RegisterError::InternalServerError);
        }
    };

    if !valid_username {
        return Err(RegisterError::UsernameExists);
    }

    let mut user = User::create_new(
        payload.username.clone(),
        payload.password,
        format!("{}_firstname", payload.username),
        format!("{}_lastname", payload.username),
        payload.email,
        false,
        true,
        false
    );

    user.create_row(&state.pg_pool).await.unwrap();
    let output = UserListOutputView::from(&user);
    
    return Ok(Json(output));
}
