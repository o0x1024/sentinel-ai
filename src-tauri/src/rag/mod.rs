pub use sentinel_rag::*;

pub mod config { pub use sentinel_rag::config::*; }
pub mod models { pub use sentinel_rag::models::*; }
pub mod service {
    pub use sentinel_rag::service::*;
    pub type RagService = sentinel_rag::service::RagService<crate::services::database::DatabaseService>;
}
pub mod database { pub use sentinel_rag::database::*; }
pub mod chunker { pub use sentinel_rag::chunker::*; }
pub mod embeddings { pub use sentinel_rag::embeddings::*; }
pub mod query_utils { pub use sentinel_rag::query_utils::*; }

mod db_impl;