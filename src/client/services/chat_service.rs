#[derive(Default)]
pub struct ChatService;

impl ChatService {
    pub fn send_private_message(&mut self, _to: &str, _msg: &str) {
        // TODO: implementazione
    }
    pub fn get_private_messages(&self, _with: &str) -> Vec<String> {
        // TODO: implementazione
        vec![]
    }
}
