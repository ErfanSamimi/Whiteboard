use chrono::{ DateTime, Utc };
use crate::project::Project;
use serde::{Serialize, Deserialize};
use super::common::AppState;
use super::auth::{Claims, AuthError};
use axum::extract::State;
use axum::{Json, debug_handler};
use sqlx::{ FromRow, PgPool };
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectCreationInput {
    name: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectUserOutput {
    id: i64,
    username: String,
    email: String,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ProjectOutput {
    id: i64,
    collaborators: Vec<ProjectUserOutput>,
    name: String,
    owner: ProjectUserOutput,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl ProjectOutput {
    pub fn from_db_rows(rows: &Vec<ProjectDetailDbRow>) -> Vec<Self> {
        let mut map: HashMap<i64, Self> = HashMap::new();
        for r in rows {
            let entry = map.entry(r.proj_id.unwrap()).or_insert(Self {
                id: r.proj_id.unwrap(),
                collaborators: Vec::new(),
                name: r.proj_name.clone().unwrap(),
                owner: ProjectUserOutput {
                    id: r.own_id.unwrap(),
                    username: r.own_username.clone().unwrap(),
                    email: r.own_email.clone().unwrap(),
                    created_at: r.own_create,
                    updated_at: r.own_update,
                },
                created_at: r.proj_create,
                updated_at: r.proj_update,
            });

            if let Some(colabrator_id) = r.colab_id {
                entry.collaborators.push(ProjectUserOutput {
                    id: colabrator_id,
                    username: r.colab_username.clone().unwrap(),
                    email: r.colab_email.clone().unwrap(),
                    created_at: r.colab_create,
                    updated_at: r.colab_update,
                });
            }
        }

        return map.into_values().collect();
    }


    async fn get_project_detail(pool: &PgPool, project_id: i64) -> Result<Option<Self>, sqlx::Error>{
        let proj_data = sqlx::query_as_unchecked!(
            ProjectDetailDbRow,
            r#"
                SELECT projects.id as proj_id, projects.name as proj_name, projects.created_at as proj_create, projects.updated_at as proj_update,
                    owners.id as own_id, owners.username as own_username, owners.email as own_email, owners.created_at as own_create, owners.updated_at as own_update,
                    colabs.id as colab_id, colabs.username as colab_username, colabs.email as colab_email, colabs.created_at as colab_create, colabs.updated_at as colab_update
                    FROM projects

                    LEFT JOIN projects_collaborators AS pc ON projects.id = pc.project_id
                    LEFT JOIN users AS owners ON projects.owner_id = owners.id
                    LEFT JOIN users AS colabs ON colabs.id = pc.user_id

                        WHERE projects.id = $1;
            "#,
            project_id
        )
        .fetch_all(pool)
        .await?;

        
        let mut proj_detail = Self::from_db_rows(&proj_data);
        if proj_detail.len() == 1{
            return Ok(Some(proj_detail.remove(0)));
        }
        else {
            return Ok(None)
        }

    }
}

#[derive(Debug, FromRow)]
pub struct ProjectDetailDbRow {
    proj_id: Option<i64>,
    proj_name: Option<String>,
    proj_create: Option<DateTime<Utc>>,
    proj_update: Option<DateTime<Utc>>,

    own_id: Option<i64>,
    own_username: Option<String>,
    own_email: Option<String>,
    own_create: Option<DateTime<Utc>>,
    own_update: Option<DateTime<Utc>>,

    colab_id: Option<i64>,
    colab_username: Option<String>,
    colab_email: Option<String>,
    colab_create: Option<DateTime<Utc>>,
    colab_update: Option<DateTime<Utc>>,
}


#[debug_handler]
pub async fn project_creation_view(
    claims: Claims,
    State(state): State<AppState>,
    Json(payload): Json<ProjectCreationInput>
) -> Result<Json<Option<ProjectOutput>>, AuthError> {

    println!("{}", claims);
    println!("{:?}", payload);

    let mut proj = Project::create_new(payload.name, claims.get_user_id());
    proj.create_row(&state.pool).await.unwrap();


    let project_id = proj.get_id().unwrap();
    let output_data =   ProjectOutput::get_project_detail(
        &state.pool, project_id
    ).await.unwrap();

    return Ok(
        Json(
          output_data
        )
    );
}



#[cfg(test)]
mod tests {
    use super::*;
    extern crate dotenv;
    use dotenv::dotenv;
    use std::env;
    use sqlx::PgPool;
    use sqlx::postgres::PgPoolOptions;

    use std::sync::Once;
    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            env_logger::init();
        });
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
    async fn test_retrieve_project_data() {
        init();
        let pool = get_test_pool().await;

        let data = ProjectOutput::get_project_detail(&pool, 38).await.unwrap().unwrap();
        println!("{:?}", data);
    }

    
    }

