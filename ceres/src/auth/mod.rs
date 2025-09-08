use anyhow::Result;
use common::errors::MegaError;
use jupiter::storage::user_storage::UserStorage;

/// User authentication information extracted from Git push process
#[derive(Debug, Clone)]
pub struct PushUserInfo {
    pub user_id: i64,
    pub username: String,
    pub primary_email: String,
    pub all_emails: Vec<String>, // User's email collection
}

/// Extract user identity from AccessToken during Git push
#[allow(async_fn_in_trait)]
pub trait UserAuthExtractor {
    /// Extract user information from AccessToken
    async fn extract_user_from_token(&self, access_token: &str) -> Result<PushUserInfo, MegaError>;
    
    /// Verify if email belongs to the authenticated user
    async fn verify_email_ownership(&self, user_id: i64, email: &str) -> Result<bool, MegaError>;
    
    /// Extract user information from username (for cases where token is already validated)
    async fn extract_user_from_username(&self, username: &str) -> Result<PushUserInfo, MegaError>;
}

/// Default implementation for user authentication
pub struct DefaultUserAuthExtractor {
    user_storage: UserStorage,
}

impl DefaultUserAuthExtractor {
    pub fn new(user_storage: UserStorage) -> Self {
        Self { user_storage }
    }
}

impl UserAuthExtractor for DefaultUserAuthExtractor {
    async fn extract_user_from_token(&self, access_token: &str) -> Result<PushUserInfo, MegaError> {
        // Find token in database to get username
        let tokens = self.user_storage.list_all_tokens().await?;
        
        for token_model in tokens {
            if token_model.token == access_token {
                return self.extract_user_from_username(&token_model.username).await;
            }
        }
        
        Err(MegaError::with_message("Invalid or expired access token"))
    }
    
    async fn extract_user_from_username(&self, username: &str) -> Result<PushUserInfo, MegaError> {
        // Get user information
        let user = self.user_storage.find_user_by_name(username).await?
            .ok_or_else(|| MegaError::with_message("User not found"))?;
        
        // Get user's email collection - for now, use the primary email
        // TODO: Extend to support multiple emails per user
        let all_emails = vec![user.email.clone()];
        
        Ok(PushUserInfo {
            user_id: user.id,
            username: user.name.clone(),
            primary_email: user.email.clone(),
            all_emails,
        })
    }
    
    async fn verify_email_ownership(&self, user_id: i64, email: &str) -> Result<bool, MegaError> {
        // Get user information
        let user = self.user_storage.find_user_by_id(user_id).await?
            .ok_or_else(|| MegaError::with_message("User not found"))?;
        
        // Check if email matches primary email
        // TODO: Extend to check against all user emails when multiple emails are supported
        Ok(user.email.to_lowercase() == email.to_lowercase())
    }
}
