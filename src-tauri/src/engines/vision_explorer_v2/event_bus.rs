//! Event Bus - Core event-driven communication system for Vision Explorer V2
//!
//! This module implements a publish-subscribe event bus that allows all agents
//! to communicate asynchronously without direct coupling.

use crate::engines::vision_explorer_v2::core::Event;
use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, warn};

/// Callback function type for event subscribers
pub type EventCallback = Arc<dyn Fn(&Event) -> futures::future::BoxFuture<'static, Result<()>> + Send + Sync>;

/// An event subscriber registration
struct Subscriber {
    id: String,
    callback: EventCallback,
    filter: Option<String>, // Optional event type filter (e.g., "TaskCompleted")
}

/// Central event bus for all agent communication
#[derive(Clone)]
pub struct EventBus {
    subscribers: Arc<RwLock<Vec<Subscriber>>>,
    event_queue: Arc<Mutex<VecDeque<Event>>>,
    processing: Arc<Mutex<bool>>,
    max_queue_size: usize,
}

impl EventBus {
    /// Create a new event bus with default configuration
    pub fn new() -> Self {
        Self::with_capacity(10000)
    }

    /// Create a new event bus with specified queue capacity
    pub fn with_capacity(max_queue_size: usize) -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
            event_queue: Arc::new(Mutex::new(VecDeque::with_capacity(max_queue_size))),
            processing: Arc::new(Mutex::new(false)),
            max_queue_size,
        }
    }

    /// Subscribe to all events or filtered events
    pub async fn subscribe<F>(
        &self,
        subscriber_id: String,
        callback: F,
        filter: Option<String>,
    ) -> Result<()>
    where
        F: Fn(&Event) -> futures::future::BoxFuture<'static, Result<()>> + Send + Sync + 'static,
    {
        let subscriber = Subscriber {
            id: subscriber_id.clone(),
            callback: Arc::new(callback),
            filter,
        };

        let mut subs = self.subscribers.write().await;
        subs.push(subscriber);
        debug!("Subscriber {} registered to event bus", subscriber_id);
        Ok(())
    }

    /// Unsubscribe from events
    pub async fn unsubscribe(&self, subscriber_id: &str) -> Result<()> {
        let mut subs = self.subscribers.write().await;
        subs.retain(|s| s.id != subscriber_id);
        debug!("Subscriber {} unregistered from event bus", subscriber_id);
        Ok(())
    }

    /// Publish an event to the bus (queues it for processing)
    pub async fn publish(&self, event: Event) -> Result<()> {
        let mut queue = self.event_queue.lock().await;
        
        if queue.len() >= self.max_queue_size {
            warn!(
                "Event queue full ({} events), dropping oldest events",
                self.max_queue_size
            );
            // Drop oldest events to make room
            for _ in 0..10 {
                queue.pop_front();
            }
        }

        queue.push_back(event);
        Ok(())
    }

    /// Process all queued events and dispatch to subscribers
    pub async fn process_events(&self) -> Result<usize> {
        let mut processing = self.processing.lock().await;
        if *processing {
            debug!("Event processing already in progress, skipping");
            return Ok(0);
        }
        *processing = true;

        let mut event_count = 0;
        loop {
            let event = {
                let mut queue = self.event_queue.lock().await;
                queue.pop_front()
            };

            match event {
                Some(evt) => {
                    event_count += 1;
                    self.dispatch_event(&evt).await;
                }
                None => break,
            }
        }

        *processing = false;
        debug!("Processed {} events from queue", event_count);
        Ok(event_count)
    }

    /// Dispatch a single event to all matching subscribers
    async fn dispatch_event(&self, event: &Event) {
        let subscribers = self.subscribers.read().await;
        let event_type = event_type_name(event);

        for subscriber in subscribers.iter() {
            // Check if subscriber has a filter that matches
            if let Some(ref filter) = subscriber.filter {
                if filter != &event_type {
                    continue;
                }
            }

            // Call subscriber callback
            if let Err(e) = (subscriber.callback)(event).await {
                warn!(
                    "Error in subscriber {} while processing {}: {}",
                    subscriber.id, event_type, e
                );
            }
        }
    }

    /// Get current queue size
    pub async fn queue_size(&self) -> usize {
        self.event_queue.lock().await.len()
    }

    /// Get list of subscriber IDs
    pub async fn subscriber_ids(&self) -> Vec<String> {
        self.subscribers
            .read()
            .await
            .iter()
            .map(|s| s.id.clone())
            .collect()
    }

    /// Clear all events from queue
    pub async fn clear_queue(&self) {
        self.event_queue.lock().await.clear();
    }
}

/// Extract the event type name for filtering
fn event_type_name(event: &Event) -> String {
    match event {
        Event::TaskAssigned { .. } => "TaskAssigned".to_string(),
        Event::TaskCompleted { .. } => "TaskCompleted".to_string(),
        Event::NodeDiscovered { .. } => "NodeDiscovered".to_string(),
        Event::CredentialsReceived { .. } => "CredentialsReceived".to_string(),
        Event::LoginTakeoverRequest { .. } => "LoginTakeoverRequest".to_string(),
        Event::SkipLogin => "SkipLogin".to_string(),
        Event::ManualLoginComplete => "ManualLoginComplete".to_string(),
        Event::LoginTimeout { .. } => "LoginTimeout".to_string(),
        Event::Log { .. } => "Log".to_string(),
        Event::Stop => "Stop".to_string(),
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_subscribe_and_publish() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let callback = move |_: &Event| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        };

        bus.subscribe("test_subscriber".to_string(), callback, None)
            .await
            .unwrap();

        let event = Event::Stop;
        bus.publish(event).await.unwrap();
        bus.process_events().await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_event_filtering() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let callback = move |_: &Event| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        };

        // Subscribe only to Stop events
        bus.subscribe(
            "stop_subscriber".to_string(),
            callback,
            Some("Stop".to_string()),
        )
        .await
        .unwrap();

        // Publish various events
        bus.publish(Event::SkipLogin).await.unwrap();
        bus.publish(Event::Stop).await.unwrap();

        bus.process_events().await.unwrap();

        // Should only receive 1 event (Stop)
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let callback = move |_: &Event| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        };

        bus.subscribe("test".to_string(), callback, None)
            .await
            .unwrap();
        bus.unsubscribe("test").await.unwrap();

        bus.publish(Event::Stop).await.unwrap();
        bus.process_events().await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
}
