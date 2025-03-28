use mongodb::{ bson::{ doc, oid::ObjectId, Document, to_document, from_document }, Collection };
use serde::{ Serialize, Deserialize };
use serde_json::Value;
use crate::whiteboard::WhiteBoardData;
use super::WhiteBoardStorage;
#[derive(Serialize, Deserialize, Debug)]
struct MongodbSavingData {
    #[serde(rename = "_id")]
    id: ObjectId,
    project_id: i64,
    data: WhiteBoardData,
}

impl MongodbSavingData {
    fn new(id: ObjectId, project_id: i64, data: WhiteBoardData) -> Self {
        return Self {
            id,
            project_id,
            data,
        };
    }
}

pub struct MongoDBStorage {
    project_id: i64,
    collection: Collection<Document>,
    object_id: Option<ObjectId>,
    whiteboard: Option<WhiteBoardData>,
}

impl MongoDBStorage {
    pub fn new(
        project_id: i64,
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

        let saving_data = MongodbSavingData::new(object_id, self.project_id, data);

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

    async fn set_whiteboard(&mut self, value: WhiteBoardData) {
        self.whiteboard = Some(value);
    }

    async fn get_whiteboard(&mut self) -> &WhiteBoardData {
        if self.whiteboard.is_none() {
            println!("Whiteboard data is empty");
            let whiteboard = self
                .load_whiteboard_data().await
                .unwrap_or(WhiteBoardData::new_empty());
            self.whiteboard = Some(whiteboard);
        }

        return self.whiteboard.as_ref().unwrap();
    }

    fn get_project_id(&self) -> i64 {
        return self.project_id;
    }
}
