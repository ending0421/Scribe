//! Example demonstrating the convenient logging macros.
//!
//! This example shows various ways to use Scribe's logging macros:
//! - Simple logging with automatic tag detection
//! - Logging with explicit tags
//! - Thread-local tag planting
//! - Format string support

use scribe::{
    scribe_d, scribe_e, scribe_i, scribe_tag_d, scribe_tag_e, scribe_tag_i, scribe_tag_v,
    scribe_tag_w, scribe_v, scribe_w, tag, tree,
};

fn main() {
    println!("=== Scribe Macro Usage Examples ===\n");

    // Example 1: Simple logging with automatic tag detection
    println!("1. Automatic tag detection (from backtrace):");
    scribe_v!("This is a verbose message");
    scribe_d!("This is a debug message");
    scribe_i!("This is an info message");
    scribe_w!("This is a warning message");
    scribe_e!("This is an error message");
    println!();

    // Example 2: Format strings
    println!("2. Format string support:");
    let username = "alice";
    let user_id = 12345;
    let score = 98.5;

    scribe_i!("User {} logged in", username);
    scribe_d!("Processing request for user_id: {}", user_id);
    scribe_i!("Score: {:.2}%", score);
    scribe_w!("Memory usage: {}MB / {}MB", 850, 1024);
    scribe_e!("Failed to connect to {}:{}", "localhost", 8080);
    println!();

    // Example 3: Multiple arguments
    println!("3. Multiple format arguments:");
    scribe_i!(
        "Connection from {} at {} - status: {}",
        "192.168.1.100",
        "2024-01-15 10:30:45",
        "OK"
    );
    scribe_d!("Values: {}, {}, {}", 1, "two", 3.0);
    println!();

    // Example 4: Explicit tags
    println!("4. Explicit tag usage:");
    scribe_tag_i!("network", "Connection established");
    scribe_tag_d!("database", "Query executed in 45ms");
    scribe_tag_w!("cache", "Cache miss for key: user_profile_12345");
    scribe_tag_e!("auth", "Invalid token provided");
    println!();

    // Example 5: Explicit tags with formatting
    println!("5. Explicit tags with format strings:");
    let endpoint = "/api/users";
    let duration_ms = 123;
    scribe_tag_i!(
        "http",
        "Request to {} completed in {}ms",
        endpoint,
        duration_ms
    );
    scribe_tag_d!("sql", "SELECT * FROM users WHERE id = {}", user_id);
    println!();

    // Example 6: Thread-local tag planting
    println!("6. Thread-local tag planting:");
    tag("session_init").plant();

    scribe_i!("Session started");
    scribe_d!("Loading user preferences");
    scribe_i!("User interface initialized");

    tag::uproot(); // Clear thread-local tag
    println!();

    // Example 7: Nested context with tag planting
    println!("7. Nested context:");
    process_request("user_request_001");
    println!();

    // Example 8: Complex data structures
    println!("8. Debug formatting:");
    let data = vec![1, 2, 3, 4, 5];
    scribe_d!("Processing data: {:?}", data);

    let config = ("localhost", 8080, true);
    scribe_i!("Server config: {:?}", config);
    println!();

    // Example 9: Error handling scenario
    println!("9. Error handling scenario:");
    match connect_to_server() {
        Ok(()) => scribe_i!("Successfully connected to server"),
        Err(e) => scribe_e!("Connection failed: {}", e),
    }
    println!();

    // Example 10: Performance monitoring
    println!("10. Performance monitoring:");
    let start = std::time::Instant::now();
    perform_operation();
    let duration = start.elapsed();
    scribe_i!("Operation completed in {:?}", duration);
    println!();

    println!("=== Example completed ===");
}

fn process_request(request_id: &str) {
    // Plant a tag for this entire function context
    tag(request_id).plant();

    scribe_i!("Processing request");
    scribe_d!("Validating input");
    authenticate_user();
    scribe_d!("Fetching data");
    scribe_i!("Request completed");

    // Clear the tag
    tag::uproot();
}

fn authenticate_user() {
    // This will use the parent's planted tag
    scribe_d!("Checking credentials");
    scribe_i!("User authenticated successfully");
}

fn connect_to_server() -> Result<(), &'static str> {
    scribe_d!("Attempting to connect...");
    Err("Connection timeout")
}

fn perform_operation() {
    scribe_v!("Starting operation");
    std::thread::sleep(std::time::Duration::from_millis(100));
    scribe_v!("Operation in progress");
    std::thread::sleep(std::time::Duration::from_millis(100));
    scribe_v!("Operation complete");
}
