#![allow(unused)]
#![feature(allocator_api)]

extern crate alloc;

//---    
use core::task::Context;
use core::task::Poll;
use core::sync::atomic::Ordering;
use core::sync::atomic::AtomicUsize;
use core::future::Future;
use core::fmt::Write;
use core::fmt::Debug;
use core::cell::RefCell;
use core::convert::Infallible;
use core::pin::Pin;

use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use hashbrown::HashMap;

use parking_lot::Mutex;
use parking_lot::RwLock;

use owo_colors::OwoColorize;
use owo_colors::Styled;

use tower::Service;

use tracing::Subscriber;
use tracing::Metadata;
use tracing::Level;
use tracing::Event;
use tracing::span::Id;
use tracing::span::Record;
use tracing::span::Attributes;
use tracing::field::Field;
use tracing::field::Visit;
use tracing::subscriber::SetGlobalDefaultError;

use atlas_core::tracing::TracingSubscriber;

//---
#[derive(Debug, Clone)]
pub enum TracingServiceRequest {
    GetHistory,
    ClearHistory,
}

#[derive(Debug)]
pub enum TracingServiceResponse {
    History(Vec<String>),
    Success(usize),
}

pub struct TracingSubscriberService<B> {
    subscriber: Arc<TracingSubscriber<B>>,
}

impl TracingSubscriberService<String> {
    pub fn new(capacity: usize) -> Self {
        Self {
            subscriber: Arc::new(TracingSubscriber::with_capacity(capacity)),
        }
    }

    pub fn as_global_default(&self) -> Result<(), SetGlobalDefaultError> {
        tracing::subscriber::set_global_default(self.subscriber.clone())
    }

    pub fn subscriber(&self) -> Arc<TracingSubscriber<String>> {
        self.subscriber.clone()
    }
}

impl Service<TracingServiceRequest> for TracingSubscriberService<String> {
    type Response = TracingServiceResponse;
    
    type Error = Infallible;
    
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    
    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    
    fn call(&mut self, req: TracingServiceRequest) -> Pin<Box<dyn Future<Output = Result<TracingServiceResponse, Infallible>> + Send>> {
        let subscriber = Arc::clone(&self.subscriber);
        
        Box::pin(async move {
            match req {
                TracingServiceRequest::GetHistory => {
                    let entries = subscriber.entries().write().iter().cloned().collect();
                    Ok(TracingServiceResponse::History(entries))
                }
                TracingServiceRequest::ClearHistory => {
                    subscriber.entries().write().clear();
                    Ok(TracingServiceResponse::Success(0))
                }
            }
        })
    }
}
