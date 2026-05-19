export type StatusReport = {
	version: string;
	storage_root: string;
	workspace_root: string;
	index_root: string;
	index_db_path: string;
	index_vectors_path: string;
	index_db_exists: boolean;
	index_vectors_exists: boolean;
	global_config_path: string;
	global_config_exists: boolean;
	deprecated_workspace_config: string | null;
	legacy_global_index: boolean;
	rerank_enabled: boolean;
	rerank_model_path: string;
	rerank_model_ready: boolean;
	log_level: string;
};

export type SearchHit = {
	path: string;
	preview: string;
};
