use crate::models::ApiError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub struct Error(pub StatusCode, pub String, pub String);
pub type Result<T> = std::result::Result<T, Error>;
impl Error {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self(
            StatusCode::BAD_REQUEST,
            "INVALID_INPUT".into(),
            message.into(),
        )
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self(StatusCode::CONFLICT, "CONFLICT".into(), message.into())
    }
    pub fn missing(message: impl Into<String>) -> Self {
        Self(StatusCode::NOT_FOUND, "NOT_FOUND".into(), message.into())
    }
    pub fn denied() -> Self {
        Self(
            StatusCode::FORBIDDEN,
            "ADMIN_REQUIRED".into(),
            "此操作只允许管理员执行".into(),
        )
    }
    pub fn internal(e: impl std::fmt::Display) -> Self {
        eprintln!("内部错误: {e}");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR".into(),
            "操作失败，请检查本地磁盘及运行日志".into(),
        )
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(ApiError {
                code: self.1,
                message: self.2,
                request_id: uuid::Uuid::new_v4().to_string(),
                details: None,
            }),
        )
            .into_response()
    }
}
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::internal(e)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::internal(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::internal(e)
    }
}
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        eprintln!("网络请求失败: {e}");
        Self(
            StatusCode::BAD_GATEWAY,
            "SYNC_UNAVAILABLE".into(),
            "无法连接管理员，已保留当前本地版本，请检查局域网连接".into(),
        )
    }
}
