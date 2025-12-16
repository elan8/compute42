#![allow(unused_imports)]

use shared::*;

#[test]
fn export_ts_bindings_frontend_contracts() {
    // Touch a representative subset to ensure the derive phase runs
    let _ = std::any::TypeId::of::<frontend::plots::PlotData>();
    let _ = std::any::TypeId::of::<frontend::lsp::LspHover>();
    let _ = std::any::TypeId::of::<frontend::lsp::LspCompletionItem>();
    let _ = std::any::TypeId::of::<frontend::orchestrator::ProjectInfo>();
    let _ = std::any::TypeId::of::<frontend::orchestrator::Tab>();
    let _ = std::any::TypeId::of::<frontend::orchestrator::FileServerInfo>();
    let _ = std::any::TypeId::of::<frontend::notebook::Notebook>();
    let _ = std::any::TypeId::of::<frontend::notebook::NotebookCell>();
    let _ = std::any::TypeId::of::<frontend::notebook::CellOutput>();
}

#[test]
fn export_ts_bindings_shared_dtos() {
    let _ = std::any::TypeId::of::<subscription::Product>();
    let _ = std::any::TypeId::of::<auth::User>();
    let _ = std::any::TypeId::of::<api::ApiError>();
}


