pub mod mongo;
pub mod redis;
use crate::whiteboard::WhiteBoardData;

pub trait WhiteBoardStorage {
    async fn get_saving_data(&mut self) -> String;
    async fn save(&mut self);
    async fn set_whiteboard(&mut self, value: WhiteBoardData);
    async fn get_whiteboard(&mut self) -> &WhiteBoardData;
    fn get_project_id(&self) -> i64;
}