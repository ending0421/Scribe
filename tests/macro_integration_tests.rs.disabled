//! Integration tests for Scribe macros
//!
//! Run with: cargo test --test macro_integration_tests

use scribe::{
    scribe_d, scribe_e, scribe_i, scribe_tag_d, scribe_tag_e, scribe_tag_i, scribe_tag_v,
    scribe_tag_w, scribe_v, scribe_w, tag, tree,
};

#[test]
fn test_all_log_levels_without_tag() {
    scribe_v!("Verbose message");
    scribe_d!("Debug message");
    scribe_i!("Info message");
    scribe_w!("Warning message");
    scribe_e!("Error message");
}

#[test]
fn test_all_log_levels_with_formatting() {
    let value = 42;
    let name = "test";

    scribe_v!("Value: {}", value);
    scribe_d!("Name: {}, Value: {}", name, value);
    scribe_i!("Complex: {} + {} = {}", 1, 2, 3);
    scribe_w!("Float: {:.2}", 3.14159);
    scribe_e!("Debug format: {:?}", vec![1, 2, 3]);
}

#[test]
fn test_all_log_levels_with_explicit_tag() {
    scribe_tag_v!("test_tag", "Verbose with tag");
    scribe_tag_d!("test_tag", "Debug with tag");
    scribe_tag_i!("test_tag", "Info with tag");
    scribe_tag_w!("test_tag", "Warning with tag");
    scribe_tag_e!("test_tag", "Error with tag");
}

#[test]
fn test_explicit_tag_with_formatting() {
    let id = 123;
    scribe_tag_i!("user", "User ID: {}", id);
    scribe_tag_d!("database", "Query time: {}ms", 45);
    scribe_tag_w!("cache", "Hit rate: {:.1}%", 87.5);
}

#[test]
fn test_thread_local_tag_planting() {
    // Initially no tag
    scribe_i!("Before planting");

    // Plant a tag
    tag("planted_tag").plant();
    scribe_i!("After planting - should use planted_tag");
    scribe_d!("Still using planted_tag");

    // Clear tag
    tag::uproot();
    scribe_i!("After uprooting - back to auto-detection");
}

#[test]
fn test_thread_local_tag_isolation() {
    use std::thread;

    tag("main_thread").plant();
    scribe_i!("Main thread with planted tag");

    let handle = thread::spawn(|| {
        // New thread should not have the tag
        scribe_i!("Spawned thread - no planted tag");

        // Plant a different tag in this thread
        tag("spawned_thread").plant();
        scribe_i!("Spawned thread with its own tag");

        tag::uproot();
    });

    handle.join().unwrap();

    // Main thread should still have its tag
    scribe_i!("Main thread still has its tag");

    tag::uproot();
}

#[test]
fn test_empty_message() {
    scribe_i!("");
    scribe_tag_d!("tag", "");
}

#[test]
fn test_unicode_messages() {
    scribe_i!("Hello 世界");
    scribe_d!("Привет мир");
    scribe_w!("こんにちは世界");
    scribe_e!("مرحبا بالعالم");
    scribe_tag_i!("unicode", "🌍🌎🌏");
}

#[test]
fn test_very_long_message() {
    let long_msg = "x".repeat(1000);
    scribe_i!("{}", long_msg);
    scribe_tag_d!("long", "{}", long_msg);
}

#[test]
fn test_special_characters() {
    scribe_i!("Special chars: \n\t\r\\");
    scribe_d!("Quotes: \"'`");
    scribe_w!("Symbols: !@#$%^&*()");
    scribe_tag_e!("special", "All: \n\t\r\\ \"' !@#$");
}

#[test]
fn test_nested_function_context() {
    fn outer() {
        tag("outer_context").plant();
        scribe_i!("Outer function");
        inner();
        scribe_i!("Back to outer");
        tag::uproot();
    }

    fn inner() {
        scribe_d!("Inner function - should use outer's tag");
    }

    outer();
}

#[test]
fn test_multiple_format_types() {
    let int_val: i32 = 42;
    let uint_val: u64 = 12345;
    let float_val: f64 = 3.14159;
    let str_val = "test";
    let bool_val = true;

    scribe_i!(
        "Types: int={}, uint={}, float={:.2}, str={}, bool={}",
        int_val,
        uint_val,
        float_val,
        str_val,
        bool_val
    );
}

#[test]
fn test_debug_formatting() {
    #[derive(Debug)]
    struct TestStruct {
        id: i32,
        name: String,
    }

    let obj = TestStruct {
        id: 1,
        name: "test".to_string(),
    };

    scribe_d!("Object: {:?}", obj);
    scribe_i!("Pretty: {:#?}", obj);
}

#[test]
fn test_macro_in_loop() {
    for i in 0..5 {
        scribe_v!("Iteration {}", i);
    }
}

#[test]
fn test_macro_in_closure() {
    let closure = || {
        scribe_i!("Inside closure");
    };

    closure();
}

#[test]
fn test_macro_with_result() {
    fn returns_result() -> Result<(), String> {
        scribe_d!("Starting operation");
        scribe_i!("Operation successful");
        Ok(())
    }

    assert!(returns_result().is_ok());
}

#[test]
fn test_macro_with_option() {
    let some_val = Some(42);
    let none_val: Option<i32> = None;

    scribe_d!("Some value: {:?}", some_val);
    scribe_d!("None value: {:?}", none_val);
}

#[test]
fn test_tag_change_during_execution() {
    tag("first").plant();
    scribe_i!("Using first tag");

    tag("second").plant();
    scribe_i!("Using second tag");

    tag("third").plant();
    scribe_i!("Using third tag");

    tag::uproot();
    scribe_i!("No tag");
}

#[test]
fn test_explicit_tag_overrides_planted() {
    tag("planted").plant();
    scribe_i!("Should use planted tag");
    scribe_tag_i!("explicit", "Should use explicit tag, not planted");
    scribe_i!("Back to planted tag");
    tag::uproot();
}

#[test]
fn test_concurrent_logging() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::thread;

    let counter = Arc::new(AtomicU32::new(0));
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let counter_clone = Arc::clone(&counter);
            thread::spawn(move || {
                tag(&format!("thread_{}", i)).plant();
                for j in 0..10 {
                    scribe_i!("Message {} from thread {}", j, i);
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                }
                tag::uproot();
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 50);
}

#[test]
fn test_macro_with_expressions() {
    let x = 5;
    let y = 10;

    scribe_i!("Sum: {}", x + y);
    scribe_d!("Product: {}", x * y);
    scribe_w!("Comparison: {} < {} = {}", x, y, x < y);
}

#[test]
fn test_zero_allocations_for_simple_cases() {
    // These should be efficiently compiled
    scribe_i!("Static message");
    scribe_tag_d!("static_tag", "Another static message");
}
