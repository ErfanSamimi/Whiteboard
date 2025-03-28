pub mod storage;
use serde::{ Deserialize, Serialize };

type Point = (f32, f32);

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Line {
    #[serde(rename = "p")]
    points: Vec<Point>,
    #[serde(rename = "c")]
    color: String,
    #[serde(rename = "w")]
    width: u32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CursorPosition {
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
    cursor_position: Option<CursorPosition>,
}


impl WhiteBoardData {
    pub fn new_empty() -> Self {
        return Self {
            lines: Vec::new(),
            cursor_position: None
        };
    }
}


#[cfg(test)]
mod tests {
    use super::storage::mongo::MongoDBStorage;
    use super::storage::WhiteBoardStorage;

    use mongodb::Client;
    use tokio;

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
