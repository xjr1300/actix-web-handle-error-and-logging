use anyhow::anyhow;
use macros::UseCaseError;

/// 登録するユーザー
pub struct RegistrationUser {
    pub user_name: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error, UseCaseError)]
pub enum RegisterUserError {
    /// 予期しないエラー
    #[error("Unexpected error: {0}")]
    #[use_case_error(error_code = 1000)]
    Unexpected(anyhow::Error),

    /// リポジトリ・エラー
    #[error("Repository error: {0}")]
    #[use_case_error(error_code = 1001)]
    Repository(anyhow::Error),

    /// パスワードが弱い
    #[error("Password is weak")]
    #[use_case_error(error_code = 2000)]
    WeakPassword,

    /// ユーザー名が既に登録されている
    #[error("User already exists: {0}")]
    #[use_case_error(error_code = 2001)]
    UserAlreadyExists(String),
}

#[tracing::instrument(
    name = "register user use case",
    skip(user),
    fields(
        user_name = %user.user_name,
    )
)]
pub async fn register_user(user: RegistrationUser) -> Result<(), RegisterUserError> {
    match user.user_name.as_str() {
        "foo" => {
            // 予期しないエラー
            tracing::error!("an error was raised when validating the user name");
            Err(RegisterUserError::Unexpected(anyhow!(
                "an error was raised when validating the use name"
            )))
        }
        "bar" => {
            // リポジトリ・エラー
            tracing::error!("an error was raised when registering the user to the database",);
            Err(RegisterUserError::Repository(anyhow!(
                "an error was raised when registering the user to the database",
            )))
        }
        "baz" => {
            // パスワードが弱い
            tracing::error!("the user was attempted to register with a weak password",);
            Err(RegisterUserError::WeakPassword)
        }
        "qux" => {
            // ユーザー名が既に登録されている
            tracing::error!("the user name was already registered: {}", user.user_name);
            Err(RegisterUserError::UserAlreadyExists(user.user_name))
        }
        _ => {
            // 成功
            Ok(())
        }
    }
}
