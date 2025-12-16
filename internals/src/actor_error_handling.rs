
use std::sync::Arc;
use log::{error, warn, info, debug};
use serde::{Deserialize, Serialize};
use crate::messages::coordination::{ErrorSeverity, ActorError, ActorHealth, HealthStatus};

/// Error handling strategies for different types of actor errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingStrategy {
    /// Retry the operation with exponential backoff
    RetryWithBackoff {
        max_retries: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    },
    /// Fall back to a degraded mode
    FallbackToDegraded,
    /// Restart the actor
    RestartActor,
    /// Propagate error to parent actor
    PropagateToParent,
    /// Log error and continue
    LogAndContinue,
    /// Trigger emergency shutdown
    EmergencyShutdown,
    /// Custom error handling strategy
    Custom(String),
}

/// Error context for better debugging and recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub actor_name: String,
    pub operation: String,
    pub timestamp: i64,
    pub error_count: u32,
    pub last_error: Option<String>,
    pub recovery_attempts: u32,
    pub strategy: ErrorHandlingStrategy,
}

/// Actor error handler - manages error recovery and propagation
pub struct ActorErrorHandler {
    // Actor system reference for coordination
    actor_system: Option<Arc<crate::actor_system::ActorSystem>>,
    
    // Error tracking per actor
    error_contexts: std::collections::HashMap<String, ErrorContext>,
    
    // Recovery strategies configuration
    recovery_strategies: std::collections::HashMap<String, ErrorHandlingStrategy>,
}

impl Default for ActorErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ActorErrorHandler {
    /// Create a new ActorErrorHandler instance
    pub fn new() -> Self {
        let mut handler = Self {
            actor_system: None,
            error_contexts: std::collections::HashMap::new(),
            recovery_strategies: std::collections::HashMap::new(),
        };
        
        // Set up default recovery strategies
        handler.setup_default_strategies();
        
        handler
    }
    
    /// Set the actor system reference
    pub fn set_actor_system(&mut self, actor_system: Arc<crate::actor_system::ActorSystem>) {
        self.actor_system = Some(actor_system);
    }
    
    /// Set up default recovery strategies for different actors
    fn setup_default_strategies(&mut self) {
        // Account actor - retry with backoff for network issues
        self.recovery_strategies.insert(
            "account".to_string(),
            ErrorHandlingStrategy::RetryWithBackoff {
                max_retries: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 10000,
            },
        );
        
        // Communication actor - restart for connection issues
        self.recovery_strategies.insert(
            "communication".to_string(),
            ErrorHandlingStrategy::RestartActor,
        );
        
        // Process actor - restart for process issues
        self.recovery_strategies.insert(
            "process".to_string(),
            ErrorHandlingStrategy::RestartActor,
        );
        
        // LSP actor - fallback to degraded mode
        self.recovery_strategies.insert(
            "lsp".to_string(),
            ErrorHandlingStrategy::FallbackToDegraded,
        );
        
        // Plot actor - log and continue for non-critical errors
        self.recovery_strategies.insert(
            "plot".to_string(),
            ErrorHandlingStrategy::LogAndContinue,
        );
        
        // File server actor - restart for file system issues
        self.recovery_strategies.insert(
            "fileserver".to_string(),
            ErrorHandlingStrategy::RestartActor,
        );
        
        // Installation actor - retry with backoff for download issues
        self.recovery_strategies.insert(
            "installation".to_string(),
            ErrorHandlingStrategy::RetryWithBackoff {
                max_retries: 5,
                initial_delay_ms: 2000,
                max_delay_ms: 30000,
            },
        );
        
        // Sysimage actor - retry with backoff for compilation issues
        self.recovery_strategies.insert(
            "sysimage".to_string(),
            ErrorHandlingStrategy::RetryWithBackoff {
                max_retries: 2,
                initial_delay_ms: 5000,
                max_delay_ms: 60000,
            },
        );
        
        // Orchestrator actor - propagate to parent (system level)
        self.recovery_strategies.insert(
            "orchestrator".to_string(),
            ErrorHandlingStrategy::PropagateToParent,
        );
        
        // Configuration actor - log and continue for config issues
        self.recovery_strategies.insert(
            "config".to_string(),
            ErrorHandlingStrategy::LogAndContinue,
        );
        
        // State actor - log and continue for state issues
        self.recovery_strategies.insert(
            "state".to_string(),
            ErrorHandlingStrategy::LogAndContinue,
        );
        
        // Execution actor - retry with backoff for execution issues
        self.recovery_strategies.insert(
            "execution".to_string(),
            ErrorHandlingStrategy::RetryWithBackoff {
                max_retries: 2,
                initial_delay_ms: 1000,
                max_delay_ms: 10000,
            },
        );
    }
    
    /// Handle an actor error with appropriate recovery strategy
    pub async fn handle_actor_error(
        &mut self,
        actor_name: &str,
        operation: &str,
        error: &str,
        severity: ErrorSeverity,
    ) -> Result<(), String> {
        debug!("ActorErrorHandler: Handling error for actor {}: {}", actor_name, error);
        
        // Update error context
        let context = self.update_error_context(actor_name, operation, error);
        
        // Get recovery strategy
        let strategy = self.get_recovery_strategy(actor_name);
        
        // Log the error
        self.log_error(actor_name, operation, error, severity.clone());
        
        // Apply recovery strategy
        match strategy {
            ErrorHandlingStrategy::RetryWithBackoff { max_retries, initial_delay_ms, max_delay_ms } => {
                self.handle_retry_with_backoff(actor_name, &context, max_retries, initial_delay_ms, max_delay_ms).await?;
            }
            ErrorHandlingStrategy::FallbackToDegraded => {
                self.handle_fallback_to_degraded(actor_name, &context).await?;
            }
            ErrorHandlingStrategy::RestartActor => {
                self.handle_restart_actor(actor_name, &context).await?;
            }
            ErrorHandlingStrategy::PropagateToParent => {
                self.handle_propagate_to_parent(actor_name, &context, error, severity).await?;
            }
            ErrorHandlingStrategy::LogAndContinue => {
                self.handle_log_and_continue(actor_name, &context).await?;
            }
            ErrorHandlingStrategy::EmergencyShutdown => {
                self.handle_emergency_shutdown(actor_name, &context, error).await?;
            }
            ErrorHandlingStrategy::Custom(strategy_name) => {
                self.handle_custom_strategy(actor_name, &context, &strategy_name).await?;
            }
        }
        
        Ok(())
    }
    
    /// Update error context for an actor
    fn update_error_context(&mut self, actor_name: &str, operation: &str, error: &str) -> ErrorContext {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        let strategy = self.get_recovery_strategy(actor_name);
        let context = self.error_contexts.entry(actor_name.to_string()).or_insert_with(|| ErrorContext {
            actor_name: actor_name.to_string(),
            operation: operation.to_string(),
            timestamp,
            error_count: 0,
            last_error: None,
            recovery_attempts: 0,
            strategy,
        });
        
        context.error_count += 1;
        context.last_error = Some(error.to_string());
        context.timestamp = timestamp;
        context.operation = operation.to_string();
        
        context.clone()
    }
    
    /// Get recovery strategy for an actor
    fn get_recovery_strategy(&self, actor_name: &str) -> ErrorHandlingStrategy {
        self.recovery_strategies.get(actor_name)
            .cloned()
            .unwrap_or(ErrorHandlingStrategy::LogAndContinue)
    }
    
    /// Log error with appropriate level
    fn log_error(&self, actor_name: &str, operation: &str, error: &str, severity: ErrorSeverity) {
        match severity {
            ErrorSeverity::Warning => {
                warn!("Actor {} warning in operation '{}': {}", actor_name, operation, error);
            }
            ErrorSeverity::Error => {
                error!("Actor {} error in operation '{}': {}", actor_name, operation, error);
            }
            ErrorSeverity::Critical => {
                error!("Actor {} CRITICAL error in operation '{}': {}", actor_name, operation, error);
            }
        }
    }
    
    /// Handle retry with exponential backoff
    async fn handle_retry_with_backoff(
        &self,
        actor_name: &str,
        context: &ErrorContext,
        max_retries: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<(), String> {
        if context.recovery_attempts >= max_retries {
            warn!("Actor {} exceeded max retries ({}), switching to fallback strategy", actor_name, max_retries);
            return self.handle_fallback_to_degraded(actor_name, context).await;
        }
        
        let delay = std::cmp::min(
            initial_delay_ms * (2_u64.pow(context.recovery_attempts)),
            max_delay_ms,
        );
        
        info!("Actor {} will retry operation '{}' in {}ms (attempt {}/{})", 
              actor_name, context.operation, delay, context.recovery_attempts + 1, max_retries);
        
        // Schedule retry
        if let Some(_actor_system) = &self.actor_system {
            // In a real implementation, this would schedule a retry
            // For now, we'll just log the retry attempt
            debug!("ActorErrorHandler: Would schedule retry for actor {} in {}ms", actor_name, delay);
        }
        
        Ok(())
    }
    
    /// Handle fallback to degraded mode
    async fn handle_fallback_to_degraded(&self, actor_name: &str, context: &ErrorContext) -> Result<(), String> {
        info!("Actor {} switching to degraded mode due to error in operation '{}'", actor_name, context.operation);
        
        // Update actor health status
        if let Some(_actor_system) = &self.actor_system {
            let _health_msg = ActorHealth {
                actor_name: actor_name.to_string(),
                status: HealthStatus::Degraded,
                message: Some(format!("Degraded mode due to: {}", context.last_error.as_ref().unwrap_or(&"Unknown error".to_string()))),
                timestamp: context.timestamp,
            };
            
            // Health update routing skipped in refactor
        }
        
        Ok(())
    }
    
    /// Handle actor restart
    async fn handle_restart_actor(&self, actor_name: &str, context: &ErrorContext) -> Result<(), String> {
        warn!("Actor {} will be restarted due to error in operation '{}'", actor_name, context.operation);
        
        if let Some(actor_system) = &self.actor_system {
            // In a real implementation, this would trigger actor restart
            // For now, we'll just log the restart attempt
            debug!("ActorErrorHandler: Would restart actor {}", actor_name);
            
            // Notify orchestrator of restart
            let _error_msg = ActorError {
                actor_name: actor_name.to_string(),
                error: format!("Restarting due to: {}", context.last_error.as_ref().unwrap_or(&"Unknown error".to_string())),
                severity: ErrorSeverity::Error,
            };
            
            let _ = actor_system.handle_actor_error(
                actor_name.to_string(),
                format!("Restarting actor due to error in {}", context.operation),
                ErrorSeverity::Error,
            ).await;
        }
        
        Ok(())
    }
    
    /// Handle error propagation to parent
    async fn handle_propagate_to_parent(&self, actor_name: &str, _context: &ErrorContext, error: &str, severity: ErrorSeverity) -> Result<(), String> {
        error!("Actor {} propagating error to parent: {}", actor_name, error);
        
        if let Some(actor_system) = &self.actor_system {
            let _error_msg = ActorError {
                actor_name: actor_name.to_string(),
                error: error.to_string(),
                severity: severity.clone(),
            };
            
            // Send to orchestrator for system-level handling
            let _ = actor_system.handle_actor_error(
                actor_name.to_string(),
                error.to_string(),
                severity,
            ).await;
        }
        
        Ok(())
    }
    
    /// Handle log and continue strategy
    async fn handle_log_and_continue(&self, actor_name: &str, context: &ErrorContext) -> Result<(), String> {
        info!("Actor {} continuing after error in operation '{}': {}", 
              actor_name, context.operation, context.last_error.as_ref().unwrap_or(&"Unknown error".to_string()));
        
        // No special handling needed, just continue
        Ok(())
    }
    
    /// Handle emergency shutdown
    async fn handle_emergency_shutdown(&self, actor_name: &str, _context: &ErrorContext, error: &str) -> Result<(), String> {
        error!("EMERGENCY SHUTDOWN triggered by actor {}: {}", actor_name, error);
        
        if let Some(actor_system) = &self.actor_system {
            actor_system.handle_emergency_shutdown(
                format!("Critical error in actor {}: {}", actor_name, error)
            ).await?;
        }
        
        Ok(())
    }
    
    /// Handle custom strategy
    async fn handle_custom_strategy(&self, actor_name: &str, context: &ErrorContext, strategy_name: &str) -> Result<(), String> {
        warn!("Actor {} using custom error handling strategy '{}' for operation '{}'", 
              actor_name, strategy_name, context.operation);
        
        // In a real implementation, this would execute custom error handling logic
        // For now, we'll just log the custom strategy
        debug!("ActorErrorHandler: Would execute custom strategy '{}' for actor {}", strategy_name, actor_name);
        
        Ok(())
    }
    
    /// Get error statistics for an actor
    pub fn get_error_stats(&self, actor_name: &str) -> Option<&ErrorContext> {
        self.error_contexts.get(actor_name)
    }
    
    /// Get all error statistics
    pub fn get_all_error_stats(&self) -> &std::collections::HashMap<String, ErrorContext> {
        &self.error_contexts
    }
    
    /// Clear error context for an actor (useful after successful recovery)
    pub fn clear_error_context(&mut self, actor_name: &str) {
        self.error_contexts.remove(actor_name);
        info!("ActorErrorHandler: Cleared error context for actor {}", actor_name);
    }
    
    /// Set custom recovery strategy for an actor
    pub fn set_recovery_strategy(&mut self, actor_name: &str, strategy: ErrorHandlingStrategy) {
        self.recovery_strategies.insert(actor_name.to_string(), strategy);
        debug!("ActorErrorHandler: Set custom recovery strategy for actor {}", actor_name);
    }
    
    /// Check if an actor is in a degraded state
    pub fn is_actor_degraded(&self, actor_name: &str) -> bool {
        if let Some(context) = self.error_contexts.get(actor_name) {
            context.error_count > 0 && context.recovery_attempts > 0
        } else {
            false
        }
    }
    
    /// Get overall system health status
    pub fn get_system_health(&self) -> HealthStatus {
        let total_errors: u32 = self.error_contexts.values().map(|ctx| ctx.error_count).sum();
        let critical_errors = self.error_contexts.values()
            .filter(|ctx| ctx.last_error.as_ref().map(|e| e.contains("critical")).unwrap_or(false))
            .count();
        
        if critical_errors > 0 {
            HealthStatus::Unhealthy
        } else if total_errors > 10 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
}

/// Error recovery coordinator - manages system-wide error recovery
pub struct ErrorRecoveryCoordinator {
    error_handler: ActorErrorHandler,
    recovery_queue: std::collections::VecDeque<RecoveryTask>,
}

#[derive(Debug, Clone)]
struct RecoveryTask {
    actor_name: String,
    operation: String,
    error: String,
    severity: ErrorSeverity,

}

impl Default for ErrorRecoveryCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorRecoveryCoordinator {
    /// Create a new ErrorRecoveryCoordinator instance
    pub fn new() -> Self {
        Self {
            error_handler: ActorErrorHandler::new(),
            recovery_queue: std::collections::VecDeque::new(),
        }
    }
    
    /// Set the actor system reference
    pub fn set_actor_system(&mut self, actor_system: Arc<crate::actor_system::ActorSystem>) {
        self.error_handler.set_actor_system(actor_system);
    }
    
    /// Queue an error for recovery
    pub fn queue_error_recovery(
        &mut self,
        actor_name: &str,
        operation: &str,
        error: &str,
        severity: ErrorSeverity,
    ) {
        let task = RecoveryTask {
            actor_name: actor_name.to_string(),
            operation: operation.to_string(),
            error: error.to_string(),
            severity,
    
        };
        
        self.recovery_queue.push_back(task);
        debug!("ErrorRecoveryCoordinator: Queued error recovery for actor {}", actor_name);
    }
    
    /// Process recovery queue
    pub async fn process_recovery_queue(&mut self) -> Result<(), String> {
        while let Some(task) = self.recovery_queue.pop_front() {
            debug!("ErrorRecoveryCoordinator: Processing recovery task for actor {}", task.actor_name);
            
            // Handle the error
            self.error_handler.handle_actor_error(
                &task.actor_name,
                &task.operation,
                &task.error,
                task.severity,
            ).await?;
        }
        
        Ok(())
    }
    
    /// Get error handler reference
    pub fn get_error_handler(&self) -> &ActorErrorHandler {
        &self.error_handler
    }
    
    /// Get mutable error handler reference
    pub fn get_error_handler_mut(&mut self) -> &mut ActorErrorHandler {
        &mut self.error_handler
    }
}
