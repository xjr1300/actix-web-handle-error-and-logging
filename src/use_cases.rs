use anyhow::anyhow;

/// 登録するユーザー
pub struct RegistrationUser {
    pub user_name: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RegisterUserError {
    /// 予期しないエラー
    #[error("Unexpected error: {0}")]
    Unexpected(anyhow::Error),

    /// リポジトリ・エラー
    #[error("Repository error: {0}")]
    Repository(anyhow::Error),

    /// パスワードが弱い
    #[error("Password is weak")]
    WeakPassword,

    /// ユーザー名が既に登録されている
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),
}

pub async fn register_user(user: RegistrationUser) -> Result<(), RegisterUserError> {
    match user.user_name.as_str() {
        "foo" => {
            // 予期しないエラー
            tracing::error!("An unexpected error raised");
            Err(RegisterUserError::Unexpected(anyhow!(
                "An unexpected error raised"
            )))
        }
        "bar" => {
            // リポジトリ・エラー
            tracing::error!("An error was raised when registering the user to the database",);
            Err(RegisterUserError::Repository(anyhow!(
                "An error was raised when registering the user to the database",
            )))
        }
        "baz" => {
            // パスワードが弱い
            tracing::error!("The user was attempted to register with a weak password",);
            Err(RegisterUserError::WeakPassword)
        }
        "qux" => {
            // ユーザー名が既に登録されている
            tracing::error!("The user name was already registered: {}", user.user_name);
            Err(RegisterUserError::UserAlreadyExists(user.user_name))
        }
        _ => {
            // 成功
            Ok(())
        }
    }
}
