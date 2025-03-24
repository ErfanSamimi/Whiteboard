use chrono::{DateTime, Utc};
use sqlx::PgPool;


pub struct Project{
    id: Option<i64>,
    name: String,
    owner_id: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}




impl Project {
    pub fn create_new(
        name: String,
        owner_id: i64,
    ) -> Self{
        let new_project = Self{
            id: None,
            name: name,
            owner_id: owner_id,
            created_at: Utc::now(),
            updated_at: Utc::now()
        };
        return new_project;
    }



    pub async fn get_by_id(pool: &PgPool, proj_id: i64) -> Result<Option<Self>, sqlx::Error> {

        let projects = sqlx::query_as!(
            Self,
            r#"
            SELECT id, name, owner_id, created_at, updated_at
            FROM projects
            WHERE id = $1
            "#,
            proj_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(projects)
    }

    pub async fn get_user_projects(pool: &PgPool, user_id: i64) -> Result<Vec<Self>, sqlx::Error> {

        let projects = sqlx::query_as!(
            Self,
            r#"
            SELECT id, name, owner_id, created_at, updated_at
            FROM projects
            WHERE owner_id = $1
            "#,
            user_id
        )
        .fetch_all(pool)
        .await?;

        Ok(projects)
    }

    
    pub fn get_id(&self) -> Option<i64>{
        self.id
    }


    pub fn get_name(&self) -> &String{
        &self.name
    }

    pub fn get_owner_id(&self) -> i64{
        self.owner_id
    }

    pub fn get_created_at(&self) -> DateTime<Utc>{
        self.created_at
    }

    pub fn get_updated_at(&self) -> DateTime<Utc>{
        self.updated_at
    }


    pub async fn create_row(&mut self, pool: &PgPool) -> Result<(), String> {
        if self.id.is_some() {
            return Err("Can not create an already exists row. The project Id must be None to create new row.".to_string());
        }

        let created_proj = sqlx::query_as!(
            Self,
            r#"
            INSERT INTO projects (owner_id, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, owner_id, name, created_at, updated_at
            "#,
            self.owner_id, self.name, self.created_at, Utc::now()
        )
        .fetch_one(pool)
        .await.unwrap();


        *self = created_proj;
        return Ok(());

    }


    pub async fn update(&mut self, pool: &PgPool) -> Result<(), String> {
        if self.id.is_none() {
            return Err("Can not update the project. The project Id must not none to update the DB row.".to_string());
        }

        let now = Utc::now();

        let updated = sqlx::query_as!(
            Self,
            r#"
            UPDATE projects SET
                owner_id = $1,
                name = $2,
                updated_at = $3
            WHERE id = $4
            RETURNING id, owner_id, name, created_at, updated_at
            "#,
            self.owner_id,
            self.name,
            now,
            self.id
        )
        .fetch_one(pool)
        .await.unwrap();

        *self = updated;
        Ok(())
    }
}


pub struct Collabrator {
    id: Option<i64>,
    project_id: i64,
    user_id: i64,
}

impl Collabrator {
    pub fn create_new(
        project_id: i64,
        user_id: i64,
    ) -> Self{
        let new_collaborator = Self{
            id: None,
            project_id: project_id,
            user_id: user_id,
        };
        return new_collaborator;
    }

    pub fn get_id(&self) -> Option<i64>{
        self.id
    }

    pub fn get_project_id(&self) -> i64 {
        self.project_id
    }


    pub fn get_user_id(&self) -> i64 {
        self.user_id
    }



    pub async fn create_row(&mut self, pool: &PgPool) -> Result<(), String> {
        if self.id.is_some() {
            return Err("Can not create an already exists row. The project-collaborator Id must be None to create new row.".to_string());
        }

        let created_collaborator = sqlx::query_as!(
            Self,
            r#"
            INSERT INTO projects_collaborators (project_id, user_id)
            VALUES ($1, $2)
            RETURNING id, project_id, user_id
            "#,
            self.project_id, self.user_id
        )
        .fetch_one(pool)
        .await.unwrap();


        *self = created_collaborator;
        return Ok(());
    }


}