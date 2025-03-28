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




    pub async fn update_collaborators(&self, pool: &PgPool, colabs: Vec<i64>) -> Result<(), sqlx::Error>{
        // Start a transaction
        let mut tx = pool.begin().await?;

        // clear all collabrators
        let collabrator_remover = sqlx::query("DELETE FROM projects_collaborators WHERE project_id = $1")
        .bind(self.get_id().unwrap())
        .execute(&mut *tx);

        // Prepare the bulk insert statement
        let mut query_str = String::from(
            r#"INSERT INTO projects_collaborators (project_id, user_id) VALUES "#,
        );

        // Generate a list of placeholders for the values
        let mut params = Vec::new();
        let mut placeholders = Vec::new();
        
        for (index, &user_id) in colabs.iter().enumerate() {
            let param_index = index + 1;
            placeholders.push(format!("(${}, ${})", param_index * 2 - 1, param_index * 2));
            params.push(self.get_id().unwrap());
            params.push(user_id);
                }

        query_str.push_str(&placeholders.join(", "));
        query_str.push_str(" ON CONFLICT (project_id, user_id) DO NOTHING"); // Avoid inserting duplicate rows

        // Execute the bulk insert
        let mut bulk_query = sqlx::query(&query_str);
        for i in params{
            bulk_query = bulk_query.bind(i);
        }


        collabrator_remover.await.unwrap();
        bulk_query.execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())

    }

    pub async fn is_collaborator(&self, pool: &PgPool, user_id: i64) -> Result<bool, sqlx::Error>{
        let result: Option<(i32,)> = sqlx::query_as("SELECT id FROM projects_collaborators WHERE project_id = $1 AND user_id = $2;")
        .bind(self.get_id().unwrap())
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        return Ok(result.is_some());
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


