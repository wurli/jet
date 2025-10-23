// Example: Using the IOPub Broker for Different Scenarios
//
// This file demonstrates various patterns for using the IOPub message broker
// to handle incoming kernel messages.

use carpo::msg::broker::{IopubBroker, ExecutionResult, BrokerConfig};
use carpo::msg::wire::jupyter_message::Message;
use carpo::msg::wire::status::ExecutionState;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

/// Example 1: Basic code execution with message correlation
/// 
/// This is the most common pattern - execute code and wait for results.
fn example_basic_execution() {
    // Assume we have a broker and shell from somewhere
    let broker = Arc::new(IopubBroker::new());
    // let shell = ...;
    
    // Create channels for this execution
    let (channels, collector) = ExecutionResult::create_channels();
    
    // Send execute request - this returns the request ID
    let code = "print('hello')";
    // let request_id = shell.send_execute_request(&code, options);
    let request_id = "example-msg-id".to_string();
    
    // Register the request with the broker
    broker.register_request(request_id.clone(), channels);
    
    // Now collect results from the channels
    // 1. Wait for Busy status
    match collector.status_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Message::Status(msg)) => {
            assert_eq!(msg.content.execution_state, ExecutionState::Busy);
        }
        Ok(_) => panic!("Expected Status message"),
        Err(e) => panic!("Timeout: {}", e),
    }
    
    // 2. Collect any results
    loop {
        match collector.status_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Message::Status(msg)) if msg.content.execution_state == ExecutionState::Idle => {
                break;
            }
            Ok(_) => continue,
            Err(_) => { /* Check other channels */ }
        }
        
        // Check for results
        if let Ok(Message::ExecuteResult(_)) = collector.execution_rx.try_recv() {
            // Got a result
        }
        
        // Check for streams
        if let Ok(Message::Stream(_)) = collector.stream_rx.try_recv() {
            // Got stream output
        }
    }
    
    // Always cleanup
    broker.unregister_request(&request_id);
}

/// Example 2: Global monitoring of all IOPub messages
///
/// Useful for debugging, logging, or building a notification system.
fn example_global_monitoring() {
    let broker = Arc::new(IopubBroker::new());
    
    // Create a channel for monitoring
    let (monitor_tx, monitor_rx) = channel();
    
    // Register as global subscriber
    broker.add_global_subscriber(monitor_tx);
    
    // In a separate thread, log all messages
    std::thread::spawn(move || {
        while let Ok(msg) = monitor_rx.recv() {
            println!("[IOPub Monitor] Received: {}", msg.message_type());
            
            // Can pattern match for specific handling
            match msg {
                Message::ExecuteError(err) => {
                    eprintln!("Execution error: {:?}", err.content.exception.evalue);
                }
                Message::Stream(stream) => {
                    print!("{}", stream.content.text);
                }
                _ => { /* Log or ignore */ }
            }
        }
    });
}

/// Example 3: Custom broker configuration
///
/// Adjust timeouts and buffer sizes for your use case.
fn example_custom_config() {
    let config = BrokerConfig {
        // Keep more orphan messages for debugging
        orphan_buffer_max: 5000,
        
        // Clean up orphans more frequently
        orphan_max_age: Duration::from_secs(30),
        
        // Longer timeout for slow kernels
        request_timeout: Duration::from_secs(600),
        
        // More frequent cleanup
        cleanup_interval: Duration::from_secs(10),
    };
    
    let broker = Arc::new(IopubBroker::with_config(config));
    
    // Use as normal...
}

/// Example 4: Collecting streams separately
///
/// Sometimes you want to handle stdout/stderr as they arrive,
/// separate from the main execution flow.
fn example_stream_handling() {
    let broker = Arc::new(IopubBroker::new());
    let (channels, collector) = ExecutionResult::create_channels();
    
    // let request_id = shell.send_execute_request(...);
    let request_id = "example-msg-id".to_string();
    broker.register_request(request_id.clone(), channels);
    
    // Spawn a thread just for stream handling
    let stream_rx = collector.stream_rx;
    let stream_thread = std::thread::spawn(move || {
        let mut output = String::new();
        
        while let Ok(msg) = stream_rx.recv_timeout(Duration::from_secs(1)) {
            if let Message::Stream(stream) = msg {
                output.push_str(&stream.content.text);
                // Could also write to a file, buffer, etc.
            }
        }
        
        output
    });
    
    // Main thread handles execution status
    while let Ok(msg) = collector.status_rx.recv_timeout(Duration::from_secs(5)) {
        if let Message::Status(status) = msg {
            if status.content.execution_state == ExecutionState::Idle {
                break;
            }
        }
    }
    
    // Get collected streams
    let streams = stream_thread.join().unwrap();
    println!("Output: {}", streams);
    
    broker.unregister_request(&request_id);
}

/// Example 5: Checking broker statistics
///
/// Monitor the broker's health and resource usage.
fn example_broker_stats() {
    let broker = Arc::new(IopubBroker::new());
    
    // Periodically check stats
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(30));
            
            let stats = broker.stats();
            
            println!("Broker Stats:");
            println!("  Active requests: {}", stats.active_requests);
            println!("  Orphan messages: {}", stats.orphan_messages);
            println!("  Global subscribers: {}", stats.global_subscribers);
            
            // Alert if orphans are accumulating
            if stats.orphan_messages > 100 {
                eprintln!("Warning: High orphan message count!");
            }
            
            // Manually trigger cleanup if needed
            broker.cleanup();
        }
    });
}

/// Example 6: RAII pattern for automatic cleanup
///
/// Ensure requests are always unregistered, even on panic.
struct ExecutionGuard {
    broker: Arc<IopubBroker>,
    request_id: String,
}

impl ExecutionGuard {
    fn new(broker: Arc<IopubBroker>, request_id: String) -> Self {
        Self { broker, request_id }
    }
}

impl Drop for ExecutionGuard {
    fn drop(&mut self) {
        self.broker.unregister_request(&self.request_id);
    }
}

fn example_raii_cleanup() {
    let broker = Arc::new(IopubBroker::new());
    let (channels, collector) = ExecutionResult::create_channels();
    
    // let request_id = shell.send_execute_request(...);
    let request_id = "example-msg-id".to_string();
    broker.register_request(request_id.clone(), channels);
    
    // Create guard - will automatically cleanup on drop
    let _guard = ExecutionGuard::new(Arc::clone(&broker), request_id);
    
    // Do work...
    // Even if this panics, the guard will cleanup
    
    // No need to manually call unregister_request
}

/// Example 7: Handling multiple concurrent executions
///
/// The broker can handle many requests simultaneously.
fn example_concurrent_executions() {
    let broker = Arc::new(IopubBroker::new());
    
    let mut handles = vec![];
    
    // Execute 10 requests concurrently
    for i in 0..10 {
        let broker_clone = Arc::clone(&broker);
        
        let handle = std::thread::spawn(move || {
            let (channels, collector) = ExecutionResult::create_channels();
            let request_id = format!("request-{}", i);
            
            broker_clone.register_request(request_id.clone(), channels);
            
            // Do execution and collect results...
            
            broker_clone.unregister_request(&request_id);
        });
        
        handles.push(handle);
    }
    
    // Wait for all to complete
    for handle in handles {
        handle.join().unwrap();
    }
}

fn main() {
    println!("These are examples - they won't actually run without a kernel connection");
    println!("See the function implementations for patterns to use in your code");
}
