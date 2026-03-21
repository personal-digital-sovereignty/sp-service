use axum::{
    extract::{Path as AxumPath, State, Query},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Serialize)]
pub struct VaultNode {
    pub id: String,
    pub name: String,
    #[serde(rename = "is_dir")]
    pub is_dir: bool,
    pub r#type: String, // "file" or "directory"
    pub path: String,
    pub children: Vec<VaultNode>,
}

/// Escaneia recursivamente o FileSystem nativo do Rust
#[async_recursion::async_recursion]
async fn scan_directory(path: &Path, root_path: &Path) -> Vec<VaultNode> {
    let mut nodes = Vec::new();

    if let Ok(mut entries) = fs::read_dir(path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let metadata = entry.metadata().await;
            if metadata.is_err() { continue; }
            let metadata = metadata.unwrap();

            let filename = entry.file_name().to_string_lossy().to_string();
            
            // Ignorar ocultos/sistemas
            if filename.starts_with('.') || filename.ends_with('~') { continue; }

            let abs_path = entry.path();
            // Id relativo para a navegação do Vue
            let rel_id = abs_path.strip_prefix(root_path).unwrap_or(&abs_path).to_string_lossy().to_string();

            if metadata.is_dir() {
                let children = scan_directory(&abs_path, root_path).await;
                nodes.push(VaultNode {
                    id: rel_id.clone(),
                    name: filename.clone(),
                    is_dir: true,
                    r#type: "directory".to_string(),
                    path: abs_path.to_string_lossy().to_string(),
                    children,
                });
            } else {
                nodes.push(VaultNode {
                    id: rel_id.clone(),
                    name: filename.clone(),
                    is_dir: false,
                    r#type: "file".to_string(),
                    path: abs_path.to_string_lossy().to_string(),
                    children: vec![],
                });
            }
        }
    }

    // Ordenar: Pastas Primeiro, depois Arquivos alfabeticamente
    nodes.sort_by(|a, b| {
        if a.r#type == b.r#type {
            a.name.cmp(&b.name)
        } else if a.r#type == "directory" {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    nodes
}

// ==========================================
// Módulo de Gerência de Discos (Workspaces)
// ==========================================

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceRow {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub created_at: Option<chrono::NaiveDateTime>,
}

#[derive(Deserialize)]
pub struct CreateWorkspaceReq {
    pub name: String,
    pub path: String,
}

/// Rota GET /v1/workspaces - Lista todos os diretórios atrelados
pub async fn list_workspaces_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, WorkspaceRow>("SELECT id, name, path, created_at FROM workspaces ORDER BY id ASC")
        .fetch_all(&state.db)
        .await;

    match rows {
        Ok(workspaces) => Json(workspaces).into_response(),
        Err(e) => {
            Json(serde_json::json!({"error": true, "message": format!("Database Error: {}", e)})).into_response()
        }
    }
}

/// Rota POST /v1/workspaces - Atrela um novo caminho do Disco ao Hub
pub async fn create_workspace_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorkspaceReq>,
) -> impl IntoResponse {
    let raw_path = PathBuf::from(&req.path);
    if !raw_path.exists() || !raw_path.is_dir() {
        return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": true, "message": "O caminho fornecido não existe ou não é um diretório no Host Linux."}))).into_response();
    }

    let absolute_str = raw_path.canonicalize().unwrap_or(raw_path).to_string_lossy().to_string();

    let res = sqlx::query("INSERT INTO workspaces (name, path) VALUES (?, ?)")
        .bind(&req.name)
        .bind(&absolute_str)
        .execute(&state.db)
        .await;

    match res {
        Ok(exec) => {
            let wk = WorkspaceRow {
                id: exec.last_insert_rowid(),
                name: req.name,
                path: absolute_str,
                created_at: None,
            };
            (axum::http::StatusCode::CREATED, Json(wk)).into_response()
        },
        Err(e) => {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true, "message": format!("Database Error: {}", e)}))).into_response()
        }
    }
}

/// Rota DELETE /v1/workspaces/:id - Desatrela um Workspace e Invoca O.S RAG Flush (Fase 33)
pub async fn delete_workspace_handler(
    AxumPath(workspace_id): AxumPath<i64>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse {
    // 1. Remove Fisicamente da Tabela Workspaces do Banco de Dados
    let res = sqlx::query("DELETE FROM workspaces WHERE id = ?")
        .bind(workspace_id)
        .execute(&state.db)
        .await;

    match res {
        Ok(exec) if exec.rows_affected() > 0 => {
            // 2. Dispara o Míssil Assíncrono para o Backend Python (FastAPI porta 8001) para executar o Flush Vetorial
            // Isso previne alucinações fantasmagóricas no LlamaIndex de Arquivos do Workspace que sumiram!
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                match client.delete("http://127.0.0.1:8000/v1/chroma/flush")
                    .timeout(std::time::Duration::from_secs(10))
                    .send()
                    .await {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            tracing::error!("🚨 [Sovereign Core] O RAG Flush falhou no Backend Python. Vetores Fantasmas podem estar ativos! HTTP {}", resp.status());
                        } else {
                            tracing::info!("💥 [Sovereign Core] RAG Flush Vectorial executado com SUCESSO via The Gateway (Python API Destruiu Coleção).");
                        }
                    },
                    Err(e) => tracing::error!("🚨 [Sovereign Core] Conexão com Backend Python Perdida ao Erradicar Workspace: {}", e),
                }
            });

            (axum::http::StatusCode::OK, Json(serde_json::json!({"status": "deleted"}))).into_response()
        },
        Ok(_) => (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"error": true, "message": "Workspace não encontrado"}))).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": true, "message": format!("Database Error: {}", e)}))).into_response(),
    }
}

/// Rota GET /v1/workspaces/:id/tree - Varredura Brutal do Diretório Alvo
pub async fn workspace_tree_handler(
    AxumPath(workspace_id): AxumPath<i64>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse {
    // 1. Validar Físicamente qual o Path Absoluto pertencente ao ID
    let ws = sqlx::query_as::<_, WorkspaceRow>("SELECT id, name, path, created_at FROM workspaces WHERE id = ?")
        .bind(workspace_id)
        .fetch_optional(&state.db)
        .await;

    let (target_path_str, target_name) = match ws {
        Ok(Some(row)) => (row.path, row.name),
        _ => return (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!([{"id": "error", "name": "Workspace Não Encontrado", "is_dir": true, "type": "directory", "path": "", "children": []}]))).into_response(),
    };

    let root = PathBuf::from(&target_path_str);
    
    // Constrói a Arvore Assincronamente ancorada SOMENTE na raiz aprovada
    let children = scan_directory(&root, &root).await;

    // A raiz do diretório pro Front Vue
    let root_node = VaultNode {
        id: "root".to_string(), // Manteve root pro UI n quebrar
        name: target_name,
        is_dir: true,
        r#type: "directory".to_string(),
        path: root.to_string_lossy().to_string(),
        children,
    };

    Json(vec![root_node]).into_response()
}

#[derive(Deserialize)]
pub struct ReadDocQuery {
    pub workspace_id: Option<i64>,
}

/// Rota GET /v1/vault/document/:id - Leitura direta do O.S Binário
pub async fn vault_document_read(
    AxumPath(file_id): AxumPath<String>,
    Query(query): Query<ReadDocQuery>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse {
    // Decodifica a URL String
    let decoded_id = urlencoding::decode(&file_id).unwrap_or(std::borrow::Cow::Borrowed(&file_id)).to_string();
    
    // FETCH THE WORKSPACE PATH
    let mut ws_root = state.vault_path.clone();

    if let Some(w_id) = query.workspace_id {
        let ws = sqlx::query_as::<_, WorkspaceRow>("SELECT id, name, path, created_at FROM workspaces WHERE id = ?")
            .bind(w_id)
            .fetch_optional(&state.db)
            .await;

        if let Ok(Some(row)) = ws {
            ws_root = PathBuf::from(row.path);
        }
    }

    let abs_path = if std::path::Path::new(&decoded_id).is_absolute() {
        PathBuf::from(&decoded_id)
    } else {
        ws_root.join::<PathBuf>(decoded_id.strip_prefix('/').unwrap_or(&decoded_id).into())
    };

    match fs::read_to_string(&abs_path).await {
        Ok(content) => {
            let file_name = abs_path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let res = serde_json::json!({
                "id": decoded_id,
                "name": file_name,
                "path": abs_path.to_string_lossy().to_string(),
                "file_path": abs_path.to_string_lossy().to_string(),
                "content": content,
            });
            Json(res).into_response()
        },
        Err(e) => {
            let err_res = serde_json::json!({
                "error": true,
                "message": format!("Sovereign OS File System Error: {}", e)
            });
            Json(err_res).into_response()
        }
    }
}

// ------------------- CRUD MUTATIONS -------------------

#[derive(Deserialize)]
pub struct FsCreateReq {
    pub workspace_id: i64,
    pub r#type: String, // "folder" or "file"
    pub name: String,
    pub path: String, // Caminho relativo da UI (ou Vazio se Root)
}

pub async fn vault_fs_create_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FsCreateReq>,
) -> impl IntoResponse {
    let ws = sqlx::query_as::<_, WorkspaceRow>("SELECT id, name, path, created_at FROM workspaces WHERE id = ?")
        .bind(req.workspace_id)
        .fetch_optional(&state.db)
        .await;

    let target_path_str = match ws {
        Ok(Some(row)) => row.path,
        _ => return (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"detail":"Workspace Não Encontrado ou Corrompido"}))).into_response(),
    };

    let ws_root = PathBuf::from(&target_path_str);

    let parent = if req.path.is_empty() {
        ws_root.clone()
    } else {
        // Desaninhamos caminhos escapados de the frontend UI
        ws_root.join::<PathBuf>(req.path.strip_prefix('/').unwrap_or(&req.path).into())
    };
    
    // BLINDAGEM ANTI-TRAVERSAL ATÔMICA
    if !parent.starts_with(&ws_root) {
        return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"detail":"Path manipulation prevented O.S"}))).into_response();
    }
    
    let target = parent.join(&req.name);

    if req.r#type == "folder" {
        if let Err(e) = fs::create_dir_all(&target).await {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("Failed to create folder: {}", e)}))).into_response();
        }
    } else {
        if let Err(e) = fs::File::create(&target).await {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("Failed to create file: {}", e)}))).into_response();
        }
    }

    (axum::http::StatusCode::OK, Json(serde_json::json!({"status":"created"}))).into_response()
}

#[derive(Deserialize)]
pub struct FsRenameReq {
    pub workspace_id: i64,
    pub path: String, // String Relativa do Componente Vue `folder/file.txt`
    pub new_name: String,
}

pub async fn vault_fs_rename_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FsRenameReq>,
) -> impl IntoResponse {
    let ws = sqlx::query_as::<_, WorkspaceRow>("SELECT id, name, path, created_at FROM workspaces WHERE id = ?")
        .bind(req.workspace_id)
        .fetch_optional(&state.db)
        .await;

    let target_path_str = match ws {
        Ok(Some(row)) => row.path,
        _ => return (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"detail":"Workspace Não Encontrado"}))).into_response(),
    };

    let ws_root = PathBuf::from(&target_path_str);
    let current = ws_root.join::<PathBuf>(req.path.strip_prefix('/').unwrap_or(&req.path).into());

    if !current.starts_with(&ws_root) {
        return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"detail":"Path manipulation prevented"}))).into_response();
    }

    let parent = current.parent().unwrap_or(&ws_root);
    let target = parent.join(&req.new_name);

    if let Err(e) = fs::rename(&current, &target).await {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("Failed to rename: {}", e)}))).into_response();
    }

    (axum::http::StatusCode::OK, Json(serde_json::json!({"status":"renamed"}))).into_response()
}

#[derive(Deserialize)]
pub struct FsDeleteReq {
    pub workspace_id: i64,
    pub path: String, // Relativo (`node.id`) provido pela TreeVue
}

pub async fn vault_fs_delete_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FsDeleteReq>,
) -> impl IntoResponse {
    let ws = sqlx::query_as::<_, WorkspaceRow>("SELECT id, name, path, created_at FROM workspaces WHERE id = ?")
        .bind(req.workspace_id)
        .fetch_optional(&state.db)
        .await;

    let target_path_str = match ws {
        Ok(Some(row)) => row.path,
        _ => return (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"detail":"Workspace Não Encontrado"}))).into_response(),
    };

    let ws_root = PathBuf::from(&target_path_str);
    let target = ws_root.join::<PathBuf>(req.path.strip_prefix('/').unwrap_or(&req.path).into());

    if !target.starts_with(&ws_root) {
        return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"detail":"Path manipulation prevented"}))).into_response();
    }

    let metadata = match fs::metadata(&target).await {
        Ok(m) => m,
        Err(e) => return (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": format!("File/Folder not found: {}", e)}))).into_response(),
    };

    if metadata.is_dir() {
        if let Err(e) = fs::remove_dir_all(&target).await {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("Failed to delete folder: {}", e)}))).into_response();
        }
    } else {
        if let Err(e) = fs::remove_file(&target).await {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("Failed to delete file: {}", e)}))).into_response();
        }
    }

    (axum::http::StatusCode::OK, Json(serde_json::json!({"status":"deleted"}))).into_response()
}

#[derive(Deserialize)]
pub struct FsMoveReq {
    pub workspace_id: i64,
    pub path: String, // String Relativa do Componente Vue `folder/file.txt` da ORIGEM
    pub target_path: String, // Destino relativo `folder/nova_pasta`
}

pub async fn vault_fs_move_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FsMoveReq>,
) -> impl IntoResponse {
    let ws = sqlx::query_as::<_, WorkspaceRow>("SELECT id, name, path, created_at FROM workspaces WHERE id = ?")
        .bind(req.workspace_id)
        .fetch_optional(&state.db)
        .await;

    let target_path_str = match ws {
        Ok(Some(row)) => row.path,
        _ => return (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"detail":"Workspace Não Encontrado"}))).into_response(),
    };

    let ws_root = PathBuf::from(&target_path_str);
    
    // Path original (absoluto host-O.S)
    let source = ws_root.join::<PathBuf>(req.path.strip_prefix('/').unwrap_or(&req.path).into());
    
    // Pasta raiz destino
    let dest_dir = if req.target_path.is_empty() {
        ws_root.clone()
    } else {
        ws_root.join::<PathBuf>(req.target_path.strip_prefix('/').unwrap_or(&req.target_path).into())
    };

    if !source.starts_with(&ws_root) || !dest_dir.starts_with(&ws_root) {
        return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"detail":"Path manipulation prevented O.S"}))).into_response();
    }

    if !source.exists() {
        return (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"detail":"Source O.S file not found"}))).into_response();
    }
    
    if !dest_dir.exists() || !dest_dir.is_dir() {
        return (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"detail":"Target must be a valid existing directory"}))).into_response();
    }

    let file_name = source.file_name().unwrap_or_default();
    let target = dest_dir.join(file_name);

    if let Err(e) = fs::rename(&source, &target).await {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("Failed to move: {}", e)}))).into_response();
    }

    (axum::http::StatusCode::OK, Json(serde_json::json!({"status":"moved"}))).into_response()
}

#[derive(Deserialize)]
pub struct WriteDocReq {
    pub workspace_id: Option<i64>, // Opcional garantindo fallback caso UI velha quebre 
    pub content: String,
}

pub async fn vault_document_write(
    AxumPath(file_id): AxumPath<String>, // Na v1 enviávamos o Absolute/Relativo encodado
    State(state): State<Arc<AppState>>,
    Json(req): Json<WriteDocReq>,
) -> impl IntoResponse {
    // Escaneia a raiz apropriada
    let mut ws_root = state.vault_path.clone();

    if let Some(w_id) = req.workspace_id {
        let ws = sqlx::query_as::<_, WorkspaceRow>("SELECT id, name, path, created_at FROM workspaces WHERE id = ?")
            .bind(w_id)
            .fetch_optional(&state.db)
            .await;

        if let Ok(Some(row)) = ws {
            ws_root = PathBuf::from(row.path);
        }
    }

    let decoded_id = urlencoding::decode(&file_id).unwrap_or(std::borrow::Cow::Borrowed(&file_id)).to_string();
    let abs_path = if Path::new(&decoded_id).is_absolute() {
        PathBuf::from(&decoded_id)
    } else {
        ws_root.join::<PathBuf>(decoded_id.strip_prefix('/').unwrap_or(&decoded_id).into())
    };

    // Segurança (O Cíbrido só escreve Arquivos em Zonas Vermelhas Autorizadas do Host)
    if !abs_path.starts_with(&ws_root) {
        return (axum::http::StatusCode::FORBIDDEN, Json(serde_json::json!({"detail":"Malicious Write Attempt Prevented O.S"}))).into_response();
    }

    if let Err(e) = fs::write(&abs_path, req.content).await {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"detail": format!("Failed to write to file: {}", e)}))).into_response();
    }

    (axum::http::StatusCode::OK, Json(serde_json::json!({"status":"saved"}))).into_response()
}

// ------------------- COGNITIVE GRAPH (NATIVE RUST) -------------------

#[derive(Serialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub val: f64,
    pub r#type: String,
    pub tags: Vec<String>,
}

#[derive(Serialize)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub r#type: String,
}

#[derive(Serialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}

#[derive(sqlx::FromRow)]
struct SensusDocRow {
    id: String,
    file_path: String,
    extracted_tags: Option<String>,
    extracted_links: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct VaultGraphQuery {
    pub workspace_id: Option<String>,
}

pub async fn vault_graph_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<VaultGraphQuery>
) -> impl IntoResponse {
    let ws_id_str = query.workspace_id.unwrap_or_else(|| "1".to_string());
    let ws_id: i64 = ws_id_str.parse().unwrap_or(1);
    
    // Catch real workspace name
    use sqlx::Row;
    let ws_name = sqlx::query("SELECT name FROM workspaces WHERE id = ?")
        .bind(ws_id)
        .fetch_one(&state.db)
        .await
        .map(|r| r.get::<String, _>("name"))
        .unwrap_or_else(|_| "Local Vault".to_string());

    let docs = sqlx::query("SELECT id, file_path, content_raw FROM sensus_documents WHERE workspace_id = ?")
        .bind(&ws_id_str)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut nodes = Vec::new();
    let mut links = Vec::new();

    // Central Cybrid Node
    nodes.push(GraphNode {
        id: "root".to_string(),
        name: format!("Sovereign System ({})", ws_name),
        path: None,
        val: 15.0,
        r#type: "folder".to_string(),
        tags: vec![],
    });

    use std::collections::HashMap;
    let mut name_to_id = HashMap::new();
    let mut doc_contents = HashMap::new();

    for doc in &docs {
        use sqlx::Row;
        let doc_id: String = doc.get("id");
        let doc_path: String = doc.get("file_path");
        let content_raw: Option<String> = doc.try_get("content_raw").ok();
        
        let node_id = format!("doc_{}", doc_id);
        let filename = doc_path.split('/').last().unwrap_or(&doc_path).to_string();
        let doc_basename = filename.strip_suffix(".md").unwrap_or(&filename).to_string();
        
        name_to_id.insert(doc_basename.clone(), node_id.clone());
        if let Some(c) = content_raw {
            doc_contents.insert(node_id.clone(), c);
        }
        
        nodes.push(GraphNode {
            id: node_id.clone(),
            name: filename,
            path: Some(doc_path.clone()),
            val: 3.0,
            r#type: "file".to_string(),
            tags: vec![],
        });

        // Hierarchy links (The thin structural tethers in the background)
        links.push(GraphLink {
            source: "root".to_string(),
            target: node_id.clone(),
            r#type: "hierarchy".to_string(),
        });
    }

    // Dynamic Obsidian Synapse Extraction (Memory Regex-less Split)
    for (node_id, content) in &doc_contents {
        for chunk in content.split("[[").skip(1) {
            if let Some(end_idx) = chunk.find("]]") {
                let link_content = &chunk[..end_idx];
                let target_name = link_content.split('|').next().unwrap_or("").trim().to_string();
                
                if let Some(target_id) = name_to_id.get(&target_name) {
                    links.push(GraphLink {
                        source: node_id.clone(),
                        target: target_id.clone(),
                        r#type: "synapse".to_string(),
                    });
                }
            }
        }
    }

    let res = GraphResponse { nodes, links };
    (axum::http::StatusCode::OK, Json(res)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query("
            CREATE TABLE IF NOT EXISTS workspaces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
        ").execute(&pool).await.unwrap();

        sqlx::query("
            INSERT INTO workspaces (id, name, path)
            SELECT 1, 'Origin Vault', '/mock/origin'
        ").execute(&pool).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_and_list_workspaces() {
        let db = setup_test_db().await;
        // Mock State for the test environment
        let (log_sender, _) = tokio::sync::broadcast::channel(16);
        let (sync_sender, _) = tokio::sync::broadcast::channel(16);
        
        let state = Arc::new(crate::AppState { 
            db, 
            http_client: reqwest::Client::new(),
            vault_path: std::path::PathBuf::from("/tmp"),
            telemetry: std::sync::Arc::new(std::sync::RwLock::new(crate::telemetry::TelemetryState::new())),
            log_sender,
            sync_sender,
        });

        // 1. Test POST /workspaces (Creation)
        // Note: Canonicalize() will fail on abstract non-existing paths, so we use a safe mock OS directory like "/" or "/tmp".
        let req = CreateWorkspaceReq { name: "Test WS".into(), path: "/tmp".into() };
        let creation_resp = create_workspace_handler(State(state.clone()), Json(req)).await.into_response();
        assert_eq!(creation_resp.status(), StatusCode::CREATED, "Failed asserting the creation of a new Workspace via Database Schema");

        // 2. Test GET /workspaces (Listing)
        let list_resp = list_workspaces_handler(State(state.clone())).await.into_response();
        assert_eq!(list_resp.status(), StatusCode::OK, "Failed asserting execution of the workpsaces query");
    }
}
