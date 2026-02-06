#![allow(dead_code)]

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub project_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_session(&self, project_path: Option<String>) -> String {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let session = Session {
            id: session_id.clone(),
            project_path,
            created_at: now,
            updated_at: now,
        };

        self.sessions.write().insert(session_id.clone(), session);
        session_id
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        self.sessions.read().get(session_id).cloned()
    }

    pub fn update_session(&self, session_id: &str) {
        if let Some(session) = self.sessions.write().get_mut(session_id) {
            session.updated_at = Utc::now();
        }
    }

    pub fn remove_session(&self, session_id: &str) -> Option<Session> {
        self.sessions.write().remove(session_id)
    }

    pub fn list_sessions(&self) -> Vec<Session> {
        self.sessions.read().values().cloned().collect()
    }
}
