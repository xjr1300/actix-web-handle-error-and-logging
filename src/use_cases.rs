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
            Err(RegisterUserError::Unexpected(anyhow!(
                "予期しないエラーが発生しました。",
            )))
        }
        "bar" => {
            // リポジトリ・エラー
            Err(RegisterUserError::Repository(anyhow!(
                "ユーザーをデータベースに登録するときにエラーが発生しました。"
            )))
        }
        "baz" => {
            // パスワードが弱い
            Err(RegisterUserError::WeakPassword)
        }
        "qux" => {
            // ユーザー名が既に登録されている
            Err(RegisterUserError::UserAlreadyExists(user.user_name))
        }
        _ => {
            // 成功
            Ok(())
        }
    }
}
