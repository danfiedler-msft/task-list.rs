use std::path::Path;

use tower_http::services::{ServeDir, ServeFile};

/// Static-file service for the built SPA. Requests that match no file fall back to
/// `index.html` **with its own 200 status** (via `fallback`, not `not_found_service`,
/// which would force a 404) so client-side deep links load the app. `/api/*` never
/// reaches here — it is handled by the API router.
pub fn spa_service(web_dist_dir: &str) -> ServeDir<ServeFile> {
    let index = Path::new(web_dist_dir).join("index.html");
    ServeDir::new(web_dist_dir).fallback(ServeFile::new(index))
}
