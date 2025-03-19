pub mod white_board_data {
    use serde::{ Deserialize, Serialize };

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct Point {
        x: f32,
        y: f32,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct Line {
        points: Vec<Point>,
        color: String,
        width: u32,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct CursorPosition {
        x: f32,
        y: f32,
        #[serde(rename = "userId")]
        user_id: String,
        color: String,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct WhiteBoardData {
        lines: Vec<Line>,
        #[serde(rename = "cursorPosition")]
        cursor_position: CursorPosition,
    }
}

pub mod whiteboard_storage {
    use super::white_board_data::WhiteBoardData;
    use mongodb::{
        bson::{ doc, oid::ObjectId, Document, to_document, from_document },
        options::{ Predicate, ServerMonitoringMode },
        results,
        Client,
        Collection,
    };
    use serde::{ Serialize, Deserialize };
    use serde_json::Value;

    pub trait WhiteBoardStorage {
        async fn get_saving_data(&mut self) -> String;
        async fn save(&mut self);
        fn set_whiteboard(&mut self, value: WhiteBoardData);
        async fn get_whiteboard(&mut self) -> &WhiteBoardData;
        fn get_project_id(&self) -> u32;
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct MongodbSavingData {
        #[serde(rename = "_id")]
        id: ObjectId,
        project_id: u32,
        data: WhiteBoardData,
    }

    impl MongodbSavingData {
        fn new(id: ObjectId, project_id: u32, data: WhiteBoardData) -> Self {
            return Self {
                id,
                project_id,
                data,
            };
        }
    }

    pub struct MongoDBStorage {
        project_id: u32,
        collection: Collection<Document>,
        object_id: Option<ObjectId>,
        whiteboard: Option<WhiteBoardData>,
    }

    impl MongoDBStorage {
        pub fn new(
            project_id: u32,
            collection: Collection<Document>,
            object_id: Option<ObjectId>
        ) -> Self {
            return Self {
                project_id: project_id,
                collection: collection,
                object_id: object_id,
                whiteboard: None,
            };
        }

        async fn load_whiteboard_data(&self) -> Result<WhiteBoardData, String> {
            let filter = doc! { "project_id": self.get_project_id() };
            let query_result = self.collection.find_one(filter).await;

            match query_result.unwrap() {
                None => {
                    return Err("No whiteboard data found in mongo collection.".to_string());
                }
                Some(value) => {
                    let saving_data: MongodbSavingData = from_document(value).unwrap();
                    return Ok(saving_data.data);
                }
            }
        }

        fn get_document_object_id(&mut self) -> ObjectId {
            if self.object_id.is_none() {
                self.object_id = Some(ObjectId::new());
            }

            return self.object_id.unwrap();
        }
    }
    impl WhiteBoardStorage for MongoDBStorage {
        async fn get_saving_data(&mut self) -> String {
            let data = self.get_whiteboard().await.clone();
            let object_id = self.get_document_object_id();


            let saving_data = MongodbSavingData::new(
                object_id,
                self.project_id,
                data
            );

            return serde_json::to_string(&saving_data).unwrap();
        }

        async fn save(&mut self) {
            let doc_id = self.get_document_object_id();
            let data = self.get_saving_data();

            let filter = doc! { "_id": doc_id };
            let json_value: Value = serde_json::from_str(data.await.as_str()).unwrap();

            // Convert serde_json::Value to bson::Document
            let update_body: Document = to_document(&json_value).unwrap();

            // Wrap the update body in a "$set" operation
            let update = doc! { "$set": update_body };
            let result = self.collection.update_one(filter, update).upsert(true).await.unwrap();

            if result.matched_count > 0 {
                println!("Document updated.")
            } else {
                println!("New document inserted.")
            }
        }

        fn set_whiteboard(&mut self, value: WhiteBoardData) {
            self.whiteboard = Some(value);
        }

        async fn get_whiteboard(&mut self) -> &WhiteBoardData {
            if self.whiteboard.is_none() {
                println!("Whiteboard data is empty");
                let whiteboard = self.load_whiteboard_data().await.unwrap();
                self.whiteboard = Some(whiteboard);
            }

            return self.whiteboard.as_ref().unwrap();
        }

        fn get_project_id(&self) -> u32 {
            return self.project_id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::whiteboard_storage::MongoDBStorage;
    use super::white_board_data::{ WhiteBoardData };
    use mongodb::{ bson::doc, options::ClientOptions, Client };
    use tokio;
    use crate::whiteboard::whiteboard_storage::WhiteBoardStorage;

    #[tokio::test]
    async fn test_get_whiteboard_data() {
        // Connect to MongoDB (use test database)
        let client_uri = "mongodb://admin:mongo_1234@127.0.0.1:27017/";
        let client = Client::with_uri_str(client_uri).await.expect("Failed to connect to MongoDB");
        let database = client.database("whiteboard_db");
        let collection = database.collection("whiteboards");

        // Create MongoDBStorage instance
        let mut storage = MongoDBStorage::new(1, collection.clone(), None);

        // Retrieve whiteboard data
        let retrieved_data = storage.get_whiteboard().await;
        println!("{:?}", retrieved_data);
        // Validate retrieved data

        println!("Test passed: Retrieved whiteboard data matches expected.");
    }
}
