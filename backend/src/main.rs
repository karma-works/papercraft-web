use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::{Query, State, Multipart, DefaultBodyLimit},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use serde::{Serialize, Deserialize};
use std::io::Write;
use cgmath::Rad;

mod config;
mod context;
pub use context::GlobalContext;
mod paper;
mod pdf_metrics;
mod vector_export;
mod util_3d;
// mod util_gl;

#[cfg(test)]
mod svg_tests;

use paper::{Papercraft, RenderablePapercraft, EdgeIndex, EdgeToggleFlapAction, FaceIndex, IslandKey, PaperOptions};
use util_3d::Vector2;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server
    Serve {
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    /// Import a model and print summary
    Import {
        /// Path to the model file (PDO, OBJ, STL, glTF)
        path: std::path::PathBuf,
    },
}

struct AppState {
    project: Option<Papercraft>,
}

#[derive(Serialize)]
struct Status {
    status: String,
    has_model: bool,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Action {
    ToggleFlap { edge: EdgeIndex, action: EdgeToggleFlapAction },
    Cut { edge: EdgeIndex, offset: Option<f32> },
    Join { edge: EdgeIndex, priority_face: Option<FaceIndex> },
    MoveIsland { island: IslandKey, delta: [f32; 2] },
    RotateIsland { island: IslandKey, angle: f32, center: [f32; 2] },
    SetOptions { options: PaperOptions, relocate_pieces: bool },
    PackIslands,
}

async fn get_status(State(state): State<Arc<Mutex<AppState>>>) -> Json<Status> {
    let state = state.lock().unwrap();
    Json(Status {
        status: "ok".to_string(),
        has_model: state.project.is_some(),
    })
}

async fn upload_model(
    State(state): State<Arc<Mutex<AppState>>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_string();
                let file_name = field.file_name().unwrap_or("model.obj").to_string();
                let data = field.bytes().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                if name == "file" {
                    let temp_dir = std::env::temp_dir();
                    let temp_path = temp_dir.join(&file_name);
                    {
                        let mut file = std::fs::File::create(&temp_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                        file.write_all(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    }

                    eprintln!("Attempting to import file: {} ({} bytes)", file_name, data.len());
                    eprintln!("Temp path: {:?}", temp_path);
                    
                    let (project, _) = paper::import::import_model_file(&temp_path)
                        .map_err(|e| {
                            eprintln!("=== Import Error ===");
                            eprintln!("File: {}", file_name);
                            eprintln!("Size: {} bytes", data.len());
                            eprintln!("Temp path: {:?}", temp_path);
                            eprintln!("Error: {:?}", e);
                            eprintln!("Error chain:");
                            for (i, cause) in e.chain().enumerate() {
                                eprintln!("  {}: {}", i, cause);
                            }
                            eprintln!("====================");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                    
                    let mut state = state.lock().unwrap();
                    state.project = Some(project.clone());

                    eprintln!("=== Import Success ===");
                    eprintln!("File: {}", file_name);
                    eprintln!("Islands: {}", project.islands().count());
                    eprintln!("======================");
                    
                    return Ok(Json(project.renderable()).into_response());
                }
            }
            Ok(None) => break,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        }
    }
    Ok(StatusCode::OK.into_response())
}

async fn get_project(State(state): State<Arc<Mutex<AppState>>>) -> Result<Json<RenderablePapercraft>, StatusCode> {
    let state = state.lock().unwrap();
    if let Some(ref project) = state.project {
        Ok(Json(project.renderable()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn perform_action(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(action): Json<Action>,
) -> Result<Json<RenderablePapercraft>, StatusCode> {
    let mut state = state.lock().unwrap();
    if let Some(ref mut project) = state.project {
        println!("Received action");
        match action {
            Action::ToggleFlap { edge, action } => {
                println!("Action: ToggleFlap");
                project.edge_toggle_flap(edge, action);
            }
            Action::Cut { edge, offset } => {
                project.edge_cut(edge, offset);
            }
            Action::Join { edge, priority_face } => {
                project.edge_join(edge, priority_face);
            }
            Action::MoveIsland { island, delta } => {
                println!("Action: MoveIsland delta={:?}", delta);
                if let Some(island) = project.island_by_key_mut(island) {
                    println!("Found island, translating...");
                    island.translate(Vector2::new(delta[0], delta[1]));
                } else {
                    println!("Island not found!");
                }
            }
            Action::RotateIsland { island, angle, center } => {
                if let Some(island) = project.island_by_key_mut(island) {
                    island.rotate(Rad(angle), Vector2::new(center[0], center[1]));
                }
            }
            Action::SetOptions { options, relocate_pieces } => {
                project.set_options(options, relocate_pieces);
            }
            Action::PackIslands => {
                project.pack_islands();
            }
        }
        Ok(Json(project.renderable()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize)]
struct ExportParams {
    format: String,  // "svg" or "pdf"
    page: Option<u32>,  // For SVG: specific page, None = all pages
}

async fn export_file(
    State(state): State<Arc<Mutex<AppState>>>,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let state = state.lock().unwrap();
    let project = state.project.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    
    match params.format.as_str() {
        "svg" => {
            let svg = if let Some(page) = params.page {
                vector_export::generate_svg(project, page)
            } else {
                vector_export::generate_svg_multipage(project)
            };
            
            match svg {
                Ok(content) => Ok((
                    [(axum::http::header::CONTENT_TYPE, "image/svg+xml")],
                    content,
                ).into_response()),
                Err(e) => {
                    eprintln!("SVG export error: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        "pdf" => {
            match vector_export::generate_pdf(project) {
                Ok(pdf_bytes) => Ok((
                    [(axum::http::header::CONTENT_TYPE, "application/pdf")],
                    pdf_bytes,
                ).into_response()),
                Err(e) => {
                    eprintln!("PDF export error: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Import { path }) => {
            println!("Importing model: {:?}", path);
            match paper::import::import_model_file(&path) {
                Ok((project, is_native)) => {
                    println!("Successfully imported model.");
                    println!("Native format: {}", is_native);
                    println!("Islands: {}", project.num_islands());
                    println!("Faces: {}", project.faces().count());
                    println!("Edges: {}", project.edges().count());
                }
                Err(e) => {
                    eprintln!("Import error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Serve { port }) => {
            serve(port).await;
        }
        None => {
            serve(3000).await;
        }
    }
}

async fn serve(port: u16) {
    let mut initial_project = None;
    let sphere_path = std::path::Path::new("examples/sphere.pdo");
    if sphere_path.exists() {
        println!("Loading default model: {:?}", sphere_path);
        match paper::import::import_model_file(sphere_path) {
            Ok((project, _)) => {
                initial_project = Some(project);
            }
            Err(e) => {
                eprintln!("Failed to load default model: {}", e);
            }
        }
    }

    let state = Arc::new(Mutex::new(AppState { project: initial_project }));

    let app = Router::new()
        .route("/api/status", get(get_status))
        .route("/api/upload", post(upload_model))
        .route("/api/project", get(get_project))
        .route("/api/action", post(perform_action))
        .route("/api/export", get(export_file))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    println!("Backend listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

