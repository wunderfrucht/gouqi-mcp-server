//! Metadata caching system for the JIRA MCP Server
//!
//! Provides TTL-based caching for JIRA metadata to improve performance
//! and reduce API calls to the JIRA instance.

use crate::error::{JiraMcpError, JiraMcpResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{debug, info};

/// Metadata cache with TTL management
#[derive(Debug)]
pub struct MetadataCache {
    /// Board name to ID mappings
    board_mappings: RwLock<HashMap<String, CacheEntry<String>>>,

    /// Board metadata
    board_info: RwLock<HashMap<String, CacheEntry<BoardInfo>>>,

    /// Project key to info mappings
    project_info: RwLock<HashMap<String, CacheEntry<ProjectInfo>>>,

    /// Issue types per project
    project_issue_types: RwLock<HashMap<String, CacheEntry<Vec<IssueTypeInfo>>>>,

    /// User account ID mappings
    user_mappings: RwLock<HashMap<String, CacheEntry<UserMapping>>>,

    /// Current user cache
    current_user: RwLock<Option<CacheEntry<UserMapping>>>,

    /// Cache configuration
    ttl: Duration,

    /// Cleanup task handle
    #[allow(dead_code)]
    cleanup_task: Option<JoinHandle<()>>,
}

/// Cache entry with timestamp
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    value: T,
    created_at: Instant,
}

/// Board information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardInfo {
    pub id: String,
    pub name: String,
    pub type_: String, // scrum, kanban, etc.
    pub project_key: Option<String>,
}

/// Project information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub key: String,
    pub name: String,
    pub project_type: String,
    pub lead: Option<String>,
}

/// Issue type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueTypeInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub subtask: bool,
}

/// User mapping information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMapping {
    pub account_id: String,
    pub display_name: String,
    pub email_address: Option<String>,
    pub username: Option<String>, // For Server instances
}

impl<T> CacheEntry<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            created_at: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
}

impl MetadataCache {
    /// Create a new metadata cache with the given TTL
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            board_mappings: RwLock::new(HashMap::new()),
            board_info: RwLock::new(HashMap::new()),
            project_info: RwLock::new(HashMap::new()),
            project_issue_types: RwLock::new(HashMap::new()),
            user_mappings: RwLock::new(HashMap::new()),
            current_user: RwLock::new(None),
            ttl: Duration::from_secs(ttl_seconds),
            cleanup_task: None,
        }
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) -> JoinHandle<()> {
        let cache = Arc::clone(&self);
        let cleanup_interval = self.ttl / 2; // Cleanup twice as often as TTL

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                cache.cleanup_expired().await;
            }
        })
    }

    /// Clean up expired entries
    async fn cleanup_expired(&self) {
        debug!("Running cache cleanup");

        let mut cleaned_count = 0;

        // Clean board mappings
        if let Ok(mut board_mappings) = self.board_mappings.write() {
            board_mappings.retain(|_, entry| {
                let expired = entry.is_expired(self.ttl);
                if expired {
                    cleaned_count += 1;
                }
                !expired
            });
        }

        // Clean board info
        if let Ok(mut board_info) = self.board_info.write() {
            board_info.retain(|_, entry| {
                let expired = entry.is_expired(self.ttl);
                if expired {
                    cleaned_count += 1;
                }
                !expired
            });
        }

        // Clean project info
        if let Ok(mut project_info) = self.project_info.write() {
            project_info.retain(|_, entry| {
                let expired = entry.is_expired(self.ttl);
                if expired {
                    cleaned_count += 1;
                }
                !expired
            });
        }

        // Clean project issue types
        if let Ok(mut project_issue_types) = self.project_issue_types.write() {
            project_issue_types.retain(|_, entry| {
                let expired = entry.is_expired(self.ttl);
                if expired {
                    cleaned_count += 1;
                }
                !expired
            });
        }

        // Clean user mappings
        if let Ok(mut user_mappings) = self.user_mappings.write() {
            user_mappings.retain(|_, entry| {
                let expired = entry.is_expired(self.ttl);
                if expired {
                    cleaned_count += 1;
                }
                !expired
            });
        }

        // Clean current user
        if let Ok(mut current_user) = self.current_user.write() {
            if let Some(entry) = current_user.as_ref() {
                if entry.is_expired(self.ttl) {
                    *current_user = None;
                    cleaned_count += 1;
                }
            }
        }

        if cleaned_count > 0 {
            debug!("Cleaned {} expired cache entries", cleaned_count);
        }
    }

    /// Get board ID by name
    pub fn get_board_id(&self, board_name: &str) -> Option<String> {
        let board_mappings = self.board_mappings.read().ok()?;
        let entry = board_mappings.get(board_name)?;

        if entry.is_expired(self.ttl) {
            None
        } else {
            Some(entry.value.clone())
        }
    }

    /// Set board ID mapping
    pub fn set_board_id(&self, board_name: String, board_id: String) -> JiraMcpResult<()> {
        let mut board_mappings = self
            .board_mappings
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to acquire write lock for board mappings"))?;

        board_mappings.insert(board_name, CacheEntry::new(board_id));
        Ok(())
    }

    /// Get board info by ID
    pub fn get_board_info(&self, board_id: &str) -> Option<BoardInfo> {
        let board_info = self.board_info.read().ok()?;
        let entry = board_info.get(board_id)?;

        if entry.is_expired(self.ttl) {
            None
        } else {
            Some(entry.value.clone())
        }
    }

    /// Set board info
    pub fn set_board_info(&self, board_id: String, info: BoardInfo) -> JiraMcpResult<()> {
        let mut board_info = self
            .board_info
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to acquire write lock for board info"))?;

        board_info.insert(board_id, CacheEntry::new(info));
        Ok(())
    }

    /// Get project info by key
    pub fn get_project_info(&self, project_key: &str) -> Option<ProjectInfo> {
        let project_info = self.project_info.read().ok()?;
        let entry = project_info.get(project_key)?;

        if entry.is_expired(self.ttl) {
            None
        } else {
            Some(entry.value.clone())
        }
    }

    /// Set project info
    pub fn set_project_info(&self, project_key: String, info: ProjectInfo) -> JiraMcpResult<()> {
        let mut project_info = self
            .project_info
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to acquire write lock for project info"))?;

        project_info.insert(project_key, CacheEntry::new(info));
        Ok(())
    }

    /// Get issue types for a project
    pub fn get_project_issue_types(&self, project_key: &str) -> Option<Vec<IssueTypeInfo>> {
        let project_issue_types = self.project_issue_types.read().ok()?;
        let entry = project_issue_types.get(project_key)?;

        if entry.is_expired(self.ttl) {
            None
        } else {
            Some(entry.value.clone())
        }
    }

    /// Set issue types for a project
    pub fn set_project_issue_types(
        &self,
        project_key: String,
        issue_types: Vec<IssueTypeInfo>,
    ) -> JiraMcpResult<()> {
        let mut project_issue_types = self.project_issue_types.write().map_err(|_| {
            JiraMcpError::cache("Failed to acquire write lock for project issue types")
        })?;

        project_issue_types.insert(project_key, CacheEntry::new(issue_types));
        Ok(())
    }

    /// Get user mapping by identifier (username or email)
    pub fn get_user_mapping(&self, identifier: &str) -> Option<UserMapping> {
        let user_mappings = self.user_mappings.read().ok()?;
        let entry = user_mappings.get(identifier)?;

        if entry.is_expired(self.ttl) {
            None
        } else {
            Some(entry.value.clone())
        }
    }

    /// Set user mapping
    pub fn set_user_mapping(&self, identifier: String, mapping: UserMapping) -> JiraMcpResult<()> {
        let mut user_mappings = self
            .user_mappings
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to acquire write lock for user mappings"))?;

        user_mappings.insert(identifier, CacheEntry::new(mapping));
        Ok(())
    }

    /// Get current user
    pub fn get_current_user(&self) -> Option<UserMapping> {
        let current_user = self.current_user.read().ok()?;
        let entry = current_user.as_ref()?;

        if entry.is_expired(self.ttl) {
            None
        } else {
            Some(entry.value.clone())
        }
    }

    /// Set current user
    pub fn set_current_user(&self, user: UserMapping) -> JiraMcpResult<()> {
        let mut current_user = self
            .current_user
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to acquire write lock for current user"))?;

        *current_user = Some(CacheEntry::new(user));
        Ok(())
    }

    /// Resolve "me" or "current_user" to account ID
    pub fn resolve_user_reference(&self, user_ref: &str) -> Option<String> {
        match user_ref.to_lowercase().as_str() {
            "me" | "current_user" => self.get_current_user().map(|u| u.account_id),
            _ => {
                // Try to find by display name, username, or email
                self.get_user_mapping(user_ref).map(|u| u.account_id)
            }
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let board_mappings_count = self.board_mappings.read().map(|m| m.len()).unwrap_or(0);

        let board_info_count = self.board_info.read().map(|m| m.len()).unwrap_or(0);

        let project_info_count = self.project_info.read().map(|m| m.len()).unwrap_or(0);

        let project_issue_types_count = self
            .project_issue_types
            .read()
            .map(|m| m.len())
            .unwrap_or(0);

        let user_mappings_count = self.user_mappings.read().map(|m| m.len()).unwrap_or(0);

        let has_current_user = self
            .current_user
            .read()
            .map(|u| u.is_some())
            .unwrap_or(false);

        CacheStats {
            board_mappings_count,
            board_info_count,
            project_info_count,
            project_issue_types_count,
            user_mappings_count,
            has_current_user,
            ttl_seconds: self.ttl.as_secs(),
        }
    }

    /// Clear all cache entries
    pub fn clear_all(&self) -> JiraMcpResult<()> {
        info!("Clearing all cache entries");

        self.board_mappings
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to clear board mappings"))?
            .clear();

        self.board_info
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to clear board info"))?
            .clear();

        self.project_info
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to clear project info"))?
            .clear();

        self.project_issue_types
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to clear project issue types"))?
            .clear();

        self.user_mappings
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to clear user mappings"))?
            .clear();

        *self
            .current_user
            .write()
            .map_err(|_| JiraMcpError::cache("Failed to clear current user"))? = None;

        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub board_mappings_count: usize,
    pub board_info_count: usize,
    pub project_info_count: usize,
    pub project_issue_types_count: usize,
    pub user_mappings_count: usize,
    pub has_current_user: bool,
    pub ttl_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test_value".to_string());

        // Should not be expired immediately
        assert!(!entry.is_expired(Duration::from_secs(1)));

        // Should be expired with zero TTL
        assert!(entry.is_expired(Duration::from_secs(0)));
    }

    #[test]
    fn test_board_id_cache() {
        let cache = MetadataCache::new(300);

        // Initially empty
        assert!(cache.get_board_id("test-board").is_none());

        // Set and get
        cache
            .set_board_id("test-board".to_string(), "123".to_string())
            .unwrap();
        assert_eq!(cache.get_board_id("test-board"), Some("123".to_string()));
    }

    #[test]
    fn test_user_reference_resolution() {
        let cache = MetadataCache::new(300);

        // Set current user
        let user = UserMapping {
            account_id: "user123".to_string(),
            display_name: "Test User".to_string(),
            email_address: Some("test@example.com".to_string()),
            username: Some("testuser".to_string()),
        };
        cache.set_current_user(user).unwrap();

        // Should resolve "me" to account ID
        assert_eq!(
            cache.resolve_user_reference("me"),
            Some("user123".to_string())
        );

        assert_eq!(
            cache.resolve_user_reference("current_user"),
            Some("user123".to_string())
        );
    }

    #[test]
    fn test_cache_stats() {
        let cache = MetadataCache::new(300);

        // Initial stats
        let stats = cache.get_stats();
        assert_eq!(stats.board_mappings_count, 0);
        assert_eq!(stats.ttl_seconds, 300);
        assert!(!stats.has_current_user);

        // Add some data
        cache
            .set_board_id("test".to_string(), "123".to_string())
            .unwrap();

        let stats = cache.get_stats();
        assert_eq!(stats.board_mappings_count, 1);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = Arc::new(MetadataCache::new(1)); // 1 second TTL

        // Set a value
        cache
            .set_board_id("test".to_string(), "123".to_string())
            .unwrap();
        assert!(cache.get_board_id("test").is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be expired
        assert!(cache.get_board_id("test").is_none());
    }
}
