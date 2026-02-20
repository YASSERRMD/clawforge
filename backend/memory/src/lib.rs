pub mod batch_embed;
pub mod embeddings;
pub mod hybrid;
pub mod mmr;
pub mod qmd_manager;
pub mod sqlite_store;
pub mod store;
pub mod temporal;
pub mod types;

pub use embeddings::{create_provider, EmbeddingProvider, EmbeddingProviderKind};
pub use hybrid::hybrid_rerank;
pub use mmr::mmr_rerank;
pub use sqlite_store::SqliteVecStore;
pub use store::{InMemoryVectorStore, MemoryStore};
pub use temporal::apply_decay;
pub use batch_embed::{BatchEmbedder, BatchEmbedProvider, EmbedItem, EmbedResult};
pub use qmd_manager::{QmdConfig, QmdCollection, QmdMemoryManager, QmdSearchResult};
