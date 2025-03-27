use chrono::{ DateTime, Utc };
use crate::api::project;
use crate::project::Project;
use crate::whiteboard::storage::WhiteBoardStorage;
use serde::{Serialize, Deserialize};
use super::common::AppState;
use super::auth::{Claims, AuthError};
use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
    debug_handler,
};
use sqlx::{ FromRow, PgPool };
use std::collections::HashMap;
use crate::whiteboard::{
    WhiteBoardData,
    storage::redis::RedisStorage as WhiteBoardRedisStorage
};


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

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectColaborationInput {
    collaborator_ids: Vec<i64>,
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



    async fn get_owned_projects(pool: &PgPool, user_id: i64) -> Result<Vec<Self>, sqlx::Error>{
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

                        WHERE projects.owner_id = $1;
            "#,
            user_id
        )
        .fetch_all(pool)
        .await?;

        
        let proj_detail = Self::from_db_rows(&proj_data);
        return Ok(proj_detail);

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


pub mod permissions {
    use axum::response::IntoResponse;

    use super::*;
    pub enum ProjPermError {
        NOT_FOUND,
        NOT_OWNER,
        NOT_COLABORATOR
    }

    impl ToString for ProjPermError {
        fn to_string(&self) -> String {
            return match self {
                Self::NOT_COLABORATOR => "not collaborator of the project.",
                Self::NOT_FOUND => "project not found",
                Self::NOT_OWNER => "not owner of the proejct"
            }.to_string();
        }
    }

    impl IntoResponse for ProjPermError {
        fn into_response(self) -> axum::response::Response {
            (StatusCode::FORBIDDEN, self.to_string()).into_response()
        }
    }

    async fn get_project(project_id: i64, state: &AppState) -> Result<Project, ProjPermError>{
        let proj = Project::get_by_id(&state.pg_pool, project_id).await.unwrap();
        if proj.is_none(){
            return Err(ProjPermError::NOT_FOUND);
        }
        return Ok(proj.unwrap());
    }

    pub async fn is_owner(project_id: i64, state: &AppState, claims: &Claims) -> Result<Project, ProjPermError>{
        let proj = get_project(project_id, state).await?;
        if proj.get_owner_id() == claims.get_user_id() {
            return Ok(proj);
        }
        return Err(ProjPermError::NOT_OWNER);
    }


    pub async fn is_collaborator(project_id: i64, state: &AppState, claims: &Claims) -> Result<Project, ProjPermError>{
        let proj = get_project(project_id, state).await?;
        if proj.get_owner_id() == claims.get_user_id() {
            return Ok(proj);
        }
        if proj.is_collaborator(&state.pg_pool, claims.get_user_id()).await.unwrap(){
            return Ok(proj);
        }
        return Err(ProjPermError::NOT_COLABORATOR);
    }  
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
    proj.create_row(&state.pg_pool).await.unwrap();


    let project_id = proj.get_id().unwrap();
    let output_data =   ProjectOutput::get_project_detail(
        &state.pg_pool, project_id
    ).await.unwrap();

    return Ok(
        Json(
          output_data
        )
    );
}



#[debug_handler]
pub async fn owned_project_list_view(
    claims: Claims,
    State(state): State<AppState>
) -> Result<Json<Vec<ProjectOutput>>, AuthError> {

    println!("{}", claims);


    let output_data =   ProjectOutput::get_owned_projects(
        &state.pg_pool, claims.get_user_id()
    ).await.unwrap();

    return Ok(
        Json(
          output_data
        )
    );
}

#[debug_handler]
pub async fn add_collaborator_view(
    claims: Claims,
    State(state): State<AppState>,
    Path(project_id): Path<i64>,
    Json(payload): Json<ProjectColaborationInput>,
) -> Result<Json<Option<ProjectOutput>>, permissions::ProjPermError> {

    println!("{}", claims);
    println!("{:?}", payload);

    let proj: Project = permissions::is_owner(project_id, &state, &claims).await?;
    proj.update_collaborators(&state.pg_pool, payload.collaborator_ids).await.unwrap();


    let output_data =   ProjectOutput::get_project_detail(
        &state.pg_pool, project_id
    ).await.unwrap();

    return Ok(
        Json(
          output_data
        )
    );
}


#[debug_handler]
pub async fn get_whiteboard_data_view(
    claims: Claims,
    State(state): State<AppState>,
    Path(project_id): Path<i64>,
) -> Result<Json<WhiteBoardData>, permissions::ProjPermError> {

    println!("{}", claims);

    let proj = permissions::is_collaborator(project_id, &state, &claims);

    let database = state.mongo_client.database("whiteboard_db");
    let collection = database.collection("whiteboards");


    proj.await?;
    
    let mut storage = WhiteBoardRedisStorage::new(
        project_id, state.redis_client, collection
    );

    return Ok(
        Json(
          storage.get_whiteboard().await.clone()
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

