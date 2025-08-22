// Stato UI generico per la GUI
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub loading: bool,
    pub error: Option<String>,
}
