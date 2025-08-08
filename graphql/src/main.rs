mod models;
mod schema;

use async_graphql::http::GraphQLPlaygroundConfig;
use async_graphql::{EmptyMutation, EmptySubscription};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::serve;
use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
};
use schema::{AppSchema, QueryRoot};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    let db_filename = std::env::var("DB_FILENAME").unwrap_or_else(|_| {
        panic!("DB_FILENAME env var not found");
    });

    let port_number = std::env::var("PORT")
        .unwrap_or("8000".to_string())
        .parse::<u16>()
        .inspect(|&val| {
            if val == 0 {
                panic!("PORT must be a positive number");
            }
        })
        .expect("PORT must be a u16 number");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_filename)
        .await?;

    let schema = AppSchema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(pool)
        .finish();

    let app = Router::new()
        .route("/", get(graphql_playground))
        .route("/graphql", get(graphql_handler).post(graphql_handler))
        .with_state(schema);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port_number));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("GraphQL server started at {}", addr);

    serve(listener, app).await?;

    Ok(())
}

async fn graphql_handler(State(schema): State<AppSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    Html(async_graphql::http::playground_source(
        GraphQLPlaygroundConfig::new("/graphql"),
    ))
}
