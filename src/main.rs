use axum::{
    extract::Query,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::sync::RwLock;
use std::time::{Duration, Instant};

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_ICON: &str = "i8958";
const BUS_ICON: &str = "i7956";
const BOAT_ICON: &str = "i12186";
const ERROR_ICON: &str = "i555"; // Warning icon
const CACHE_TTL_SECS: u64 = 3600; // 1 hour cache for stops

// Popular stops in Nantes for dropdown suggestions
const POPULAR_STOPS: &[(&str, &str)] = &[
    ("COMM", "Commerce"),
    ("GANO", "Gare de Nantes"),
    ("CRQU", "Place du Cirque"),
    ("MEDI", "Médiathèque"),
    ("HBLI", "Hôtel de Ville"),
    ("ORVL", "Orvault Grand Val"),
    ("NEUP", "Neustrie"),
    ("CTRE", "Centre"),
    ("VERT", "Vertou"),
    ("STDO", "Saint-Donatien"),
    ("CICE", "Cité des Congrès"),
    ("JAUR", "Jaurès"),
    ("5050", "50 Otages"),
    ("PLDU", "Place du Duc"),
    ("LNCS", "Ligne Campus"),
];

// ============================================================================
// Cache for valid stop codes
// ============================================================================

struct StopsCache {
    stops: HashSet<String>,
    last_update: Option<Instant>,
}

static STOPS_CACHE: Lazy<RwLock<StopsCache>> = Lazy::new(|| {
    RwLock::new(StopsCache {
        stops: HashSet::new(),
        last_update: None,
    })
});

async fn refresh_stops_cache() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let stops: Vec<NaolibStop> = client
        .get("https://open.tan.fr/ewp/arrets.json")
        .send()
        .await?
        .json()
        .await?;

    let stop_codes: HashSet<String> = stops.into_iter().map(|s| s.code_lieu).collect();

    if let Ok(mut cache) = STOPS_CACHE.write() {
        cache.stops = stop_codes;
        cache.last_update = Some(Instant::now());
        tracing::info!("Stops cache refreshed with {} entries", cache.stops.len());
    }

    Ok(())
}

fn is_cache_valid() -> bool {
    if let Ok(cache) = STOPS_CACHE.read() {
        if let Some(last_update) = cache.last_update {
            return last_update.elapsed().as_secs() < CACHE_TTL_SECS;
        }
    }
    false
}

async fn ensure_cache_fresh() {
    if !is_cache_valid() {
        if let Err(e) = refresh_stops_cache().await {
            tracing::warn!("Failed to refresh stops cache: {}", e);
        }
    }
}

fn is_valid_stop_code(code: &str) -> bool {
    if let Ok(cache) = STOPS_CACHE.read() {
        // If cache is empty, accept any code (fail open)
        if cache.stops.is_empty() {
            return true;
        }
        return cache.stops.contains(code);
    }
    true // Fail open if can't read cache
}

// ============================================================================
// Naolib API structures
// ============================================================================

#[derive(Debug, Deserialize)]
struct NaolibLigne {
    #[serde(rename = "numLigne")]
    num_ligne: String,
}

#[derive(Debug, Deserialize)]
struct NaolibPassage {
    sens: u8,
    terminus: String,
    temps: String,
    ligne: NaolibLigne,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct NaolibStop {
    #[serde(rename = "codeLieu")]
    code_lieu: String,
    libelle: String,
}

// ============================================================================
// LaMetric response structures
// ============================================================================

#[derive(Debug, Serialize)]
struct LaMetricFrame {
    icon: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct LaMetricResponse {
    frames: Vec<LaMetricFrame>,
}

impl LaMetricResponse {
    fn error(message: &str) -> Self {
        LaMetricResponse {
            frames: vec![LaMetricFrame {
                icon: ERROR_ICON.to_string(),
                text: message.to_string(),
            }],
        }
    }

    fn single(icon: &str, text: &str) -> Self {
        LaMetricResponse {
            frames: vec![LaMetricFrame {
                icon: icon.to_string(),
                text: text.to_string(),
            }],
        }
    }
}

// ============================================================================
// Query parameters and configuration
// ============================================================================

#[derive(Debug, Deserialize)]
struct QueryParams {
    stop: Option<String>,
    line: Option<String>,
    direction: Option<u8>,
    limit: Option<usize>,
    #[serde(default)]
    show_terminus: bool,
}

struct Config {
    stop_code: String,
    line: Option<String>,
    direction: Option<u8>,
    limit: usize,
    show_terminus: bool,
}

#[derive(Debug)]
enum ConfigError {
    MissingStopCode,
    InvalidStopCode(String),
    InvalidDirection(u8),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingStopCode => write!(f, "Stop code required"),
            ConfigError::InvalidStopCode(code) => write!(f, "Invalid stop: {}", code),
            ConfigError::InvalidDirection(d) => write!(f, "Direction must be 1 or 2, got {}", d),
        }
    }
}

impl Config {
    fn from_env() -> Self {
        let stop_code = env::var("NAOLIB_STOP_CODE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_default();

        let line = env::var("NAOLIB_LINE").ok().filter(|s| !s.is_empty());

        let direction = env::var("NAOLIB_DIRECTION")
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|d| d.parse().ok());

        let limit = env::var("NAOLIB_LIMIT")
            .ok()
            .and_then(|l| l.parse().ok())
            .unwrap_or(2);

        Config {
            stop_code,
            line,
            direction,
            limit,
            show_terminus: false,
        }
    }

    fn with_query_params(mut self, params: &QueryParams) -> Result<Self, ConfigError> {
        // Override with query params if provided
        if let Some(ref stop) = params.stop {
            if !stop.is_empty() {
                self.stop_code = stop.to_uppercase();
            }
        }

        if let Some(ref line) = params.line {
            if !line.is_empty() {
                self.line = Some(line.to_uppercase());
            }
        }

        if let Some(direction) = params.direction {
            if direction != 1 && direction != 2 {
                return Err(ConfigError::InvalidDirection(direction));
            }
            self.direction = Some(direction);
        }

        if let Some(limit) = params.limit {
            self.limit = limit.clamp(1, 10);
        }

        self.show_terminus = params.show_terminus;

        // Validate stop code
        if self.stop_code.is_empty() {
            return Err(ConfigError::MissingStopCode);
        }

        if !is_valid_stop_code(&self.stop_code) {
            return Err(ConfigError::InvalidStopCode(self.stop_code.clone()));
        }

        Ok(self)
    }
}

// ============================================================================
// Core logic
// ============================================================================

async fn fetch_passages(config: &Config) -> Result<Vec<NaolibPassage>, reqwest::Error> {
    let url = format!(
        "https://open.tan.fr/ewp/tempsattente.json/{}",
        config.stop_code
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let passages: Vec<NaolibPassage> = client.get(&url).send().await?.json().await?;

    Ok(passages)
}

fn filter_passages(passages: Vec<NaolibPassage>, config: &Config) -> Vec<NaolibPassage> {
    passages
        .into_iter()
        .filter(|p| {
            if p.temps.is_empty() {
                return false;
            }
            if let Some(ref line) = config.line {
                if p.ligne.num_ligne.to_uppercase() != *line {
                    return false;
                }
            }
            if let Some(direction) = config.direction {
                if p.sens != direction {
                    return false;
                }
            }
            true
        })
        .take(config.limit)
        .collect()
}

fn get_icon_for_line(line_num: &str) -> &'static str {
    match line_num {
        "1" | "2" | "3" => DEFAULT_ICON, // Tramway
        l if l.starts_with('C') => BUS_ICON, // Chronobus
        l if l.starts_with('N') => BOAT_ICON, // Navibus
        _ => BUS_ICON,
    }
}

fn format_for_lametric(passages: Vec<NaolibPassage>, show_terminus: bool) -> LaMetricResponse {
    if passages.is_empty() {
        return LaMetricResponse::single(DEFAULT_ICON, "Aucun");
    }

    let frames: Vec<LaMetricFrame> = passages
        .into_iter()
        .map(|p| {
            let icon = get_icon_for_line(&p.ligne.num_ligne).to_string();
            let text = if show_terminus {
                // Shorten terminus for LaMetric display
                let terminus_short = if p.terminus.len() > 12 {
                    format!("{}.", &p.terminus[..11])
                } else {
                    p.terminus.clone()
                };
                format!("{} {} {}", p.ligne.num_ligne, terminus_short, p.temps)
            } else {
                format!("L{} {}", p.ligne.num_ligne, p.temps)
            };
            LaMetricFrame { icon, text }
        })
        .collect();

    LaMetricResponse { frames }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

async fn handler(
    Query(params): Query<QueryParams>,
) -> Result<Json<LaMetricResponse>, (StatusCode, Json<LaMetricResponse>)> {
    // Ensure cache is fresh (non-blocking if already fresh)
    ensure_cache_fresh().await;

    let base_config = Config::from_env();

    let config = match base_config.with_query_params(&params) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Configuration error: {}", e);
            let message = match &e {
                ConfigError::MissingStopCode => "No stop",
                ConfigError::InvalidStopCode(_) => "Bad stop",
                ConfigError::InvalidDirection(_) => "Bad dir",
            };
            return Err((StatusCode::BAD_REQUEST, Json(LaMetricResponse::error(message))));
        }
    };

    let passages = match fetch_passages(&config).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("API error: {}", e);
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(LaMetricResponse::error("API err")),
            ));
        }
    };

    let filtered = filter_passages(passages, &config);
    let response = format_for_lametric(filtered, config.show_terminus);

    Ok(Json(response))
}

async fn health() -> &'static str {
    "OK"
}

// ============================================================================
// Stops endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
struct StopsQueryParams {
    search: Option<String>,
    limit: Option<usize>,
}

async fn stops_handler(
    Query(params): Query<StopsQueryParams>,
) -> Result<Json<Vec<NaolibStop>>, (StatusCode, String)> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let stops: Vec<NaolibStop> = client
        .get("https://open.tan.fr/ewp/arrets.json")
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let limit = params.limit.unwrap_or(100).min(500);

    let filtered: Vec<NaolibStop> = if let Some(ref search) = params.search {
        let search_lower = search.to_lowercase();
        stops
            .into_iter()
            .filter(|s| {
                s.libelle.to_lowercase().contains(&search_lower)
                    || s.code_lieu.to_lowercase().contains(&search_lower)
            })
            .take(limit)
            .collect()
    } else {
        stops.into_iter().take(limit).collect()
    };

    Ok(Json(filtered))
}

#[derive(Debug, Serialize)]
struct PopularStop {
    code: &'static str,
    name: &'static str,
}

async fn popular_stops_handler() -> Json<Vec<PopularStop>> {
    let stops: Vec<PopularStop> = POPULAR_STOPS
        .iter()
        .map(|(code, name)| PopularStop { code, name })
        .collect();
    Json(stops)
}

// ============================================================================
// Info endpoint for documentation
// ============================================================================

#[derive(Debug, Serialize)]
struct ApiInfo {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    endpoints: Vec<EndpointInfo>,
    parameters: Vec<ParamInfo>,
    examples: Vec<ExampleInfo>,
}

#[derive(Debug, Serialize)]
struct EndpointInfo {
    path: &'static str,
    method: &'static str,
    description: &'static str,
}

#[derive(Debug, Serialize)]
struct ParamInfo {
    name: &'static str,
    #[serde(rename = "type")]
    param_type: &'static str,
    required: bool,
    description: &'static str,
}

#[derive(Debug, Serialize)]
struct ExampleInfo {
    description: &'static str,
    url: &'static str,
}

async fn info_handler() -> Json<ApiInfo> {
    Json(ApiInfo {
        name: "NaoLaMetric",
        version: env!("CARGO_PKG_VERSION"),
        description: "LaMetric Time app for Nantes public transport (TAN/Naolib) real-time arrivals",
        endpoints: vec![
            EndpointInfo {
                path: "/",
                method: "GET",
                description: "Get next arrivals formatted for LaMetric Time",
            },
            EndpointInfo {
                path: "/stops",
                method: "GET",
                description: "Search all available stops",
            },
            EndpointInfo {
                path: "/popular-stops",
                method: "GET",
                description: "Get list of popular stops for dropdown",
            },
            EndpointInfo {
                path: "/health",
                method: "GET",
                description: "Health check endpoint",
            },
            EndpointInfo {
                path: "/info",
                method: "GET",
                description: "API documentation",
            },
        ],
        parameters: vec![
            ParamInfo {
                name: "stop",
                param_type: "string",
                required: true,
                description: "Stop code (e.g., COMM, GANO, CRQU)",
            },
            ParamInfo {
                name: "line",
                param_type: "string",
                required: false,
                description: "Filter by line number (e.g., 1, 2, C1)",
            },
            ParamInfo {
                name: "direction",
                param_type: "integer",
                required: false,
                description: "Filter by direction (1 or 2)",
            },
            ParamInfo {
                name: "limit",
                param_type: "integer",
                required: false,
                description: "Number of results (1-10, default: 2)",
            },
            ParamInfo {
                name: "show_terminus",
                param_type: "boolean",
                required: false,
                description: "Show destination in output (default: false)",
            },
        ],
        examples: vec![
            ExampleInfo {
                description: "Next arrivals at Commerce",
                url: "/?stop=COMM",
            },
            ExampleInfo {
                description: "Line 1 at Commerce, direction 1",
                url: "/?stop=COMM&line=1&direction=1",
            },
            ExampleInfo {
                description: "Next 5 arrivals with terminus",
                url: "/?stop=GANO&limit=5&show_terminus=true",
            },
            ExampleInfo {
                description: "Search stops containing 'gare'",
                url: "/stops?search=gare",
            },
        ],
    })
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Pre-warm the stops cache
    tracing::info!("Warming up stops cache...");
    if let Err(e) = refresh_stops_cache().await {
        tracing::warn!("Failed to warm cache (will retry on first request): {}", e);
    }

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let app = Router::new()
        .route("/", get(handler))
        .route("/health", get(health))
        .route("/stops", get(stops_handler))
        .route("/popular-stops", get(popular_stops_handler))
        .route("/info", get(info_handler));

    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
