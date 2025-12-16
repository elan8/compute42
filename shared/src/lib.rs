use serde::{Deserialize, Serialize};
use ts_rs::TS;
use chrono::{DateTime, Utc};

// ============================================================================
// PRODUCT TYPES
// ============================================================================

/// Product enumeration for different subscription tiers
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub enum Product {
    Starter,
    Pro,
    Enterprise,
}

// ============================================================================
// AUTHENTICATION TYPES
// ============================================================================

/// Subscription information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct Subscription {
    pub id: String,
    pub product: Product, // Using Product enum instead of plan_type string
    pub status: String, // "active", "expired", "cancelled"
    pub paddle_subscription_id: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub owner_id: Option<String>,
    pub assigned_by: Option<String>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// User role
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub enum UserRole {
    User,
    Admin,
}

/// User information returned by the API
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct User {
    pub id: String,
    pub email: String,
    pub email_verified: bool,
    pub role: UserRole,
    pub enabled: bool,
    pub eula_accepted: bool,
    pub eula_accepted_at: Option<DateTime<Utc>>,
    pub product: Option<Product>, // Dynamically set from subscription data
    pub paddle_customer_id: Option<String>, // For Paddle webhook customer lookups
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub password_changed_at: Option<DateTime<Utc>>,
}

/// Authentication response containing user data and tokens
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct AuthResponse {
    pub user: User,
    pub access_token: String,
    pub refresh_token: String,
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Token refresh response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
}

/// User registration request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// User login request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Email verification request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct VerifyEmailRequest {
    pub email: String,
    pub code: String,
}

/// Password change request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// Resend verification email request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct ResendVerificationRequest {
    pub email: String,
}

/// Subscription plan selection request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct SelectSubscriptionRequest {
    pub product: Product, // Using Product enum instead of plan string
}

/// Plan type enumeration (deprecated - use Product enum instead)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub enum PlanType {
    Pro,
    // Enterprise, // Not available for now
}

/// Subscription plan information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct SubscriptionPlan {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_available: bool,
    pub paddle_product_id: Option<String>,
    pub max_users: Option<i32>, // For future enterprise plans
}

/// Price information for a subscription plan
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct Price {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub amount: i64, // Amount in cents
    pub currency_code: String,
    pub billing_cycle: Option<BillingCycle>,
    pub trial_period: Option<BillingCycle>,
    pub tax_mode: String,
    pub status: String,
    pub paddle_price_id: Option<String>,
}

/// Billing cycle information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct BillingCycle {
    pub frequency: i32,
    pub interval: String, // "day", "week", "month", "year"
}

/// EULA acceptance request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct AcceptEulaRequest {
    pub accepted: bool,
}

// ============================================================================
// SUPPORT TICKET TYPES
// ============================================================================

/// Support ticket status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub enum TicketStatus {
    Open,
    InProgress,
    WaitingForUser,
    Resolved,
    Closed,
}

/// Support ticket priority
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Support ticket type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub enum TicketType {
    Bug,
    FeatureRequest,
    GeneralSupport,
    TechnicalIssue,
    Billing,
    Other,
}

/// Support ticket
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct SupportTicket {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub ticket_type: TicketType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub closed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub assigned_to: Option<String>,
    pub tags: Vec<String>,
    pub attachments: Vec<String>,
}

/// New support ticket request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct NewSupportTicket {
    pub title: String,
    pub description: String,
    pub priority: TicketPriority,
    pub ticket_type: TicketType,
    pub tags: Vec<String>,
}

/// Support message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct SupportMessage {
    pub id: String,
    pub ticket_id: String,
    pub user_id: String,
    pub content: String,
    pub is_internal: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub attachments: Vec<String>,
    // New fields for displaying user information
    pub user_email: Option<String>,
    pub is_admin: Option<bool>,
}

/// New support message request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct NewSupportMessage {
    pub content: String,
    pub is_internal: bool,
    pub attachments: Vec<String>,
}

/// Ticket update request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct TicketUpdate {
    pub status: Option<TicketStatus>,
    pub priority: Option<TicketPriority>,
    pub assigned_to: Option<String>,
    pub tags: Option<Vec<String>>,
    pub closed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Ticket filter
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct TicketFilter {
    pub status: Option<TicketStatus>,
    pub priority: Option<TicketPriority>,
    pub ticket_type: Option<TicketType>,
    pub assigned_to: Option<String>,
    pub user_id: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub search: Option<String>,
}

/// Ticket statistics
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct TicketStats {
    pub total_tickets: u32,
    pub open_tickets: u32,
    pub in_progress_tickets: u32,
    pub resolved_tickets: u32,
    pub closed_tickets: u32,
    pub average_response_time_hours: f64,
    pub average_resolution_time_hours: f64,
}

// ============================================================================
// API RESPONSE TYPES
// ============================================================================

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub message: Option<String>,
}

/// Standard API error response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct ApiError {
    pub error: String,
    pub status: u16,
    pub details: Option<String>,
}

// ============================================================================
// VALIDATION TYPES
// ============================================================================

/// Password validation rules
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct PasswordValidation {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digit: bool,
    pub require_special: bool,
}

impl Default for PasswordValidation {
    fn default() -> Self {
        Self {
            min_length: 6,
            require_uppercase: false,
            require_lowercase: false,
            require_digit: false,
            require_special: false,
        }
    }
}

/// Email validation rules
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct EmailValidation {
    pub require_verification: bool,
    pub max_length: usize,
}

impl Default for EmailValidation {
    fn default() -> Self {
        Self {
            require_verification: true,
            max_length: 254, // RFC 5321 limit
        }
    }
}

// ============================================================================
// CONFIGURATION TYPES
// ============================================================================

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct AppConfig {
    pub auth: AuthConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_expiry: u64, // seconds
    pub refresh_token_expiry: u64, // seconds
    pub password_validation: PasswordValidation,
    pub email_validation: EmailValidation,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Validate email format
pub fn validate_email(email: &str) -> bool {
    if email.is_empty() || email.len() > 254 {
        return false;
    }
    
    // Basic email validation - in production, use a proper email validation library
    email.contains('@') && 
    email.contains('.') && 
    !email.starts_with('@') && 
    !email.ends_with('@') &&
    !email.starts_with('.') && 
    !email.ends_with('.')
}

/// Validate password against rules
pub fn validate_password(password: &str, rules: &PasswordValidation) -> Result<(), String> {
    if password.len() < rules.min_length {
        return Err(format!("Password must be at least {} characters long", rules.min_length));
    }
    
    if rules.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter".to_string());
    }
    
    if rules.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain at least one lowercase letter".to_string());
    }
    
    if rules.require_digit && !password.chars().any(|c| c.is_numeric()) {
        return Err("Password must contain at least one digit".to_string());
    }
    
    if rules.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err("Password must contain at least one special character".to_string());
    }
    
    Ok(())
}

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default password validation rules
pub const DEFAULT_PASSWORD_VALIDATION: PasswordValidation = PasswordValidation {
    min_length: 6,
    require_uppercase: false,
    require_lowercase: false,
    require_digit: false,
    require_special: false,
};

/// Default email validation rules
pub const DEFAULT_EMAIL_VALIDATION: EmailValidation = EmailValidation {
    require_verification: true,
    max_length: 254,
};

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub mod auth {
    pub use super::{
        User, UserRole, AuthResponse, RegisterRequest, LoginRequest, RefreshRequest, RefreshResponse,
        VerifyEmailRequest, ChangePasswordRequest, ResendVerificationRequest
    };
}

pub mod subscription {
    pub use super::{
        Product, PlanType, SubscriptionPlan, Price, BillingCycle, AcceptEulaRequest
    };
}

pub mod support {
    pub use super::{
        SupportTicket, NewSupportTicket, SupportMessage, NewSupportMessage,
        TicketUpdate, TicketFilter, TicketStats, TicketStatus, TicketPriority, TicketType
    };
}

pub mod config {
    pub use super::{
        AppConfig, AuthConfig, ServerConfig, DatabaseConfig,
        PasswordValidation, EmailValidation,
        DEFAULT_PASSWORD_VALIDATION, DEFAULT_EMAIL_VALIDATION
    };
}

pub mod validation {
    pub use super::{validate_email, validate_password};
}

pub mod api {
    pub use super::{ApiResponse, ApiError};
}

pub mod frontend;
