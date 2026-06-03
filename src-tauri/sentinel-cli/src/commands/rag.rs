use crate::args::{
    RagAction, RagCommand, RagConfigArgs, RagCreateCollectionArgs, RagIngestArgs, RagQueryArgs,
};
use crate::output::print_json;
use crate::state::{
    build_ingest_request, build_query_request, init_database, init_rag_service,
    load_or_default_core_rag_config, parse_metadata_pairs,
};
use anyhow::Result;
use sentinel_db::Database;

pub async fn run(command: RagCommand) -> Result<()> {
    match command.action {
        RagAction::Config(args) => configure(args).await,
        RagAction::Status => status().await,
        RagAction::CreateCollection(args) => create_collection(args).await,
        RagAction::Ingest(args) => ingest(args).await,
        RagAction::Query(args) => query(args).await,
    }
}

async fn configure(args: RagConfigArgs) -> Result<()> {
    let db = init_database().await?;
    let mut config = load_or_default_core_rag_config(&db).await?;

    if let Some(value) = args.vector_db_path {
        config.database_path = Some(value);
    }
    if let Some(value) = args.embedding_provider {
        config.embedding_provider = value;
    }
    if let Some(value) = args.embedding_model {
        config.embedding_model = value;
    }
    if let Some(value) = args.embedding_base_url {
        config.embedding_base_url = Some(value);
    }
    if let Some(value) = args.embedding_api_key {
        config.embedding_api_key = Some(value);
    }
    if let Some(value) = args.embedding_dimensions {
        config.embedding_dimensions = Some(value);
    }
    if let Some(value) = args.top_k {
        config.top_k = value;
    }

    db.save_rag_config(&config).await?;
    print_json(&config)
}

async fn status() -> Result<()> {
    let db = init_database().await?;
    let service = init_rag_service(db).await?;
    print_json(&service.get_status().await?)
}

async fn create_collection(args: RagCreateCollectionArgs) -> Result<()> {
    let db = init_database().await?;
    let service = init_rag_service(db).await?;
    let collection_id = service
        .create_collection(&args.name, args.description.as_deref())
        .await?;

    print_json(&serde_json::json!({
        "collection_id": collection_id,
        "name": args.name,
        "description": args.description,
    }))
}

async fn ingest(args: RagIngestArgs) -> Result<()> {
    let db = init_database().await?;
    let service = init_rag_service(db).await?;
    let metadata = parse_metadata_pairs(&args.metadata)?;
    let request = build_ingest_request(&args.file, args.collection_id, metadata)?;
    print_json(&service.ingest_source(request).await?)
}

async fn query(args: RagQueryArgs) -> Result<()> {
    let db = init_database().await?;
    let service = init_rag_service(db).await?;
    let request = build_query_request(args.query, args.collection_id, args.top_k);
    print_json(&service.query(request).await?)
}
