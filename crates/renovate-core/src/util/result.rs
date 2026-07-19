use std::any::Any;
use std::fmt;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;

use crate::util::SafeParseResult;
use tracing::warn;

/// @parity lib/util/result.ts partial — async chaining helpers added; transform/catch
/// overload parity and zod-safe wrappers still pending.
/// Value returned by `Result::unwrap()`.
#[derive(Debug, PartialEq)]
pub enum Res<T, E> {
    Ok { val: T },
    Err { err: E },
}

#[derive(Debug)]
enum ResultErrorKind<E> {
    Typed(E),
    Uncaught(String),
}

#[derive(Debug)]
enum ResultState<T, E> {
    Ok(T),
    Err(ResultErrorKind<E>),
}

fn format_panic(err: Box<dyn Any + Send>) -> String {
    if let Some(value) = err.downcast_ref::<&str>() {
        value.to_string()
    } else if let Some(value) = err.downcast_ref::<String>() {
        value.clone()
    } else {
        "panicked with a non-string value".to_owned()
    }
}

fn from_error_state<T, E>(error: ResultErrorKind<E>) -> Result<T, E> {
    Result {
        state: ResultState::Err(error),
    }
}

/// A result value that keeps unhandled panics for deferred handling.
#[derive(Debug)]
pub struct Result<T, E> {
    state: ResultState<T, E>,
}

impl<T, E> Result<T, E> {
    /// Constructor mirroring `Result.ok()`.
    pub fn ok(val: T) -> Self {
        Self {
            state: ResultState::Ok(val),
        }
    }

    /// Constructor mirroring `Result.err()`.
    pub fn err(err: E) -> Self {
        Self {
            state: ResultState::Err(ResultErrorKind::Typed(err)),
        }
    }

    /// Internal constructor mirroring `_uncaught()`.
    pub fn _uncaught(err: impl core::fmt::Display) -> Self {
        Self {
            state: ResultState::Err(ResultErrorKind::Uncaught(err.to_string())),
        }
    }

    /// Wrap a synchronous callback and convert any panic to `err`.
    pub fn wrap(callback: impl FnOnce() -> T) -> Self
    where
        E: From<String> + 'static,
    {
        match catch_unwind(AssertUnwindSafe(callback)) {
            Ok(value) => Self::ok(value),
            Err(err) => {
                let message = format_panic(err);
                warn!(message, "Result: wrap callback panicked");
                Self::err(message.into())
            }
        }
    }

    /// Wrap a nullable callback.
    pub fn wrap_nullable(callback: impl FnOnce() -> Nullish<T>, err: E) -> Self
    where
        E: From<String> + 'static,
    {
        match catch_unwind(AssertUnwindSafe(callback)) {
            Ok(value) => match value {
                Nullish::Value(value) => Self::ok(value),
                Nullish::Null | Nullish::Undefined => Self::err(err),
            },
            Err(error) => {
                let message = format_panic(error);
                warn!(message, "Result: wrap_nullable callback panicked");
                Self::err(message.into())
            }
        }
    }

    /// Wrap a nullable value (non-callback helper).
    pub fn wrap_nullable_value(value: Nullish<T>, err: E) -> Self {
        match value {
            Nullish::Value(value) => Self::ok(value),
            Nullish::Null | Nullish::Undefined => Self::err(err),
        }
    }

    pub fn parse<Out>(self, parser: impl FnOnce(T) -> SafeParseResult<Out>) -> Result<Out, E>
    where
        E: From<String>,
    {
        match self.state {
            ResultState::Ok(value) => match parser(value) {
                SafeParseResult::Ok(value) => Result::ok(value),
                SafeParseResult::Err(error) => Result::err(error.into()),
            },
            ResultState::Err(error) => from_error_state(error),
        }
    }

    pub fn parse_value<Out>(
        input: T,
        parser: impl FnOnce(T) -> SafeParseResult<Out>,
    ) -> Result<Out, E>
    where
        E: From<String>,
    {
        match parser(input) {
            SafeParseResult::Ok(value) => Result::ok(value),
            SafeParseResult::Err(error) => Result::err(error.into()),
        }
    }

    pub fn transform_async<U, R>(self, f: impl FnOnce(T) -> R + Send + 'static) -> AsyncResult<U, E>
    where
        U: Send + 'static,
        R: AsyncTransformResult<U, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        AsyncResult {
            inner: Box::pin(async move {
                match self.state {
                    ResultState::Ok(value) => match catch_unwind(AssertUnwindSafe(|| f(value))) {
                        Ok(result) => result.into_async_result().inner.await,
                        Err(err) => {
                            let message = format_panic(err);
                            warn!(message, "Result: unhandled async transform error");
                            Result::<U, E>::_uncaught(message)
                        }
                    },
                    ResultState::Err(error) => from_error_state(error),
                }
            }),
        }
    }

    pub fn catch_async(
        self,
        f: impl FnOnce(E) -> AsyncResult<T, E> + Send + 'static,
    ) -> AsyncResult<T, E>
    where
        T: Send + 'static,
        E: Send + 'static,
    {
        AsyncResult {
            inner: Box::pin(async move {
                match self.state {
                    ResultState::Ok(value) => Result::ok(value),
                    ResultState::Err(ResultErrorKind::Typed(err)) => {
                        match catch_unwind(AssertUnwindSafe(|| f(err))) {
                            // The catcher returned an AsyncResult; await its inner future to obtain
                            // the custom Result value that this async block produces (matches other arms).
                            Ok(result) => result.inner.await,
                            Err(err) => {
                                let message = format_panic(err);
                                warn!(message, "Result: unexpected error in async catch handler");
                                Result::_uncaught(message)
                            }
                        }
                    }
                    ResultState::Err(error) => from_error_state(error),
                }
            }),
        }
    }

    /// Returns the union-shaped output used by the TypeScript API.
    pub fn unwrap(self) -> Res<T, E> {
        match self.state {
            ResultState::Ok(value) => Res::Ok { val: value },
            ResultState::Err(ResultErrorKind::Typed(err)) => Res::Err { err },
            ResultState::Err(ResultErrorKind::Uncaught(message)) => {
                panic!("Result: unhandled transform error: {message}")
            }
        }
    }

    /// Returns an optional fallback for an error.
    pub fn unwrap_or(self, fallback: T) -> T {
        match self.state {
            ResultState::Ok(value) => value,
            ResultState::Err(ResultErrorKind::Typed(_)) => fallback,
            ResultState::Err(ResultErrorKind::Uncaught(message)) => {
                panic!("Result: unhandled transform error while falling back: {message}")
            }
        }
    }

    /// Returns the ok value or throws the error.
    pub fn unwrap_or_throw(self) -> T
    where
        E: core::fmt::Display,
    {
        match self.state {
            ResultState::Ok(value) => value,
            ResultState::Err(ResultErrorKind::Typed(err)) => panic!("{err}"),
            ResultState::Err(ResultErrorKind::Uncaught(message)) => panic!("{message}"),
        }
    }

    /// Returns the ok value or `None` on failure.
    pub fn unwrap_or_null(self) -> Option<T> {
        match self.state {
            ResultState::Ok(value) => Some(value),
            ResultState::Err(ResultErrorKind::Typed(_)) => None,
            ResultState::Err(ResultErrorKind::Uncaught(message)) => {
                panic!("Result: unhandled transform error: {message}")
            }
        }
    }

    /// Apply a sync transformation over an ok value.
    pub fn transform<U, R>(self, f: impl FnOnce(T) -> R) -> Result<U, E>
    where
        R: TransformResult<U, E>,
    {
        match self.state {
            ResultState::Ok(value) => match catch_unwind(AssertUnwindSafe(|| f(value))) {
                Ok(result) => result.into_result(),
                Err(err) => {
                    let message = format_panic(err);
                    warn!(message, "Result: unhandled transform error");
                    Result::<U, E>::_uncaught(message)
                }
            },
            ResultState::Err(error) => from_error_state(error),
        }
    }

    /// Handle a failure with a sync callback.
    pub fn catch(self, f: impl FnOnce(E) -> Self) -> Self {
        match self.state {
            ResultState::Ok(value) => Result::ok(value),
            ResultState::Err(ResultErrorKind::Typed(err)) => {
                match catch_unwind(AssertUnwindSafe(|| f(err))) {
                    Ok(result) => result,
                    Err(callback_err) => {
                        let message = format_panic(callback_err);
                        warn!(message, "Result: unexpected error in catch handler");
                        Self::_uncaught(message)
                    }
                }
            }
            ResultState::Err(error) => from_error_state(error),
        }
    }

    /// Register a callback for ok values.
    pub fn on_value(self, f: impl FnOnce(&T)) -> Self {
        match self.state {
            ResultState::Ok(value) => match catch_unwind(AssertUnwindSafe(|| f(&value))) {
                Ok(()) => Result::ok(value),
                Err(err) => {
                    let message = format_panic(err);
                    warn!(message, "Result: error in on_value handler");
                    Self::_uncaught(message)
                }
            },
            ResultState::Err(error) => from_error_state(error),
        }
    }

    /// Register a callback for error values.
    pub fn on_error(self, f: impl FnOnce(&E)) -> Self {
        match self.state {
            ResultState::Err(ResultErrorKind::Typed(err)) => {
                match catch_unwind(AssertUnwindSafe(|| f(&err))) {
                    Ok(()) => Result::err(err),
                    Err(callback_err) => {
                        let message = format_panic(callback_err);
                        warn!(message, "Result: error in on_error handler");
                        Self::_uncaught(message)
                    }
                }
            }
            ResultState::Ok(value) => Result::ok(value),
            ResultState::Err(error) => from_error_state(error),
        }
    }
}

/// Async equivalent of `Result`.
pub struct AsyncResult<T, E> {
    inner: Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>,
}

impl<T, E> fmt::Debug for AsyncResult<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncResult").finish_non_exhaustive()
    }
}

impl<T: Send + 'static, E: Send + 'static> Result<T, E> {
    /// Internal helper mirroring `this` to async `AsyncResult`.
    fn into_async_result(self) -> AsyncResult<T, E> {
        AsyncResult {
            inner: Box::pin(async move { self }),
        }
    }
}

impl<T: Send + 'static, E: Send + 'static> AsyncResult<T, E> {
    pub fn ok(val: T) -> Self {
        Self {
            inner: Box::pin(async move { Result::ok(val) }),
        }
    }

    pub fn err(err: E) -> Self {
        Self {
            inner: Box::pin(async move { Result::err(err) }),
        }
    }

    pub fn wrap<P>(promise: P) -> Self
    where
        P: Future<Output = T> + Send + 'static,
    {
        Self {
            inner: Box::pin(async move { Result::ok(promise.await) }),
        }
    }

    pub fn wrap_result<P>(promise: P) -> Self
    where
        P: Future<Output = Result<T, E>> + Send + 'static,
    {
        Self {
            inner: Box::pin(async move { promise.await }),
        }
    }

    pub fn wrap_nullable<F>(promise: F, err: E) -> Self
    where
        F: Future<Output = Nullish<T>> + Send + 'static,
    {
        Self {
            inner: Box::pin(async move {
                match promise.await {
                    Nullish::Value(value) => Result::ok(value),
                    Nullish::Null | Nullish::Undefined => Result::err(err),
                }
            }),
        }
    }

    pub async fn unwrap(self) -> Res<T, E> {
        self.inner.await.unwrap()
    }

    pub async fn unwrap_or(self, fallback: T) -> T {
        self.inner.await.unwrap_or(fallback)
    }

    pub async fn unwrap_or_throw(self) -> T
    where
        E: core::fmt::Display,
    {
        self.inner.await.unwrap_or_throw()
    }

    pub async fn unwrap_or_null(self) -> Option<T> {
        self.inner.await.unwrap_or_null()
    }

    pub fn transform<U, R>(self, f: impl FnOnce(T) -> R + Send + 'static) -> AsyncResult<U, E>
    where
        U: Send + 'static,
        R: AsyncTransformResult<U, E> + Send + 'static,
    {
        AsyncResult {
            inner: Box::pin(async move {
                let current = self.inner.await;
                match current.state {
                    ResultState::Ok(value) => match catch_unwind(AssertUnwindSafe(|| f(value))) {
                        Ok(result) => result.into_async_result().inner.await,
                        Err(err) => {
                            let message = format_panic(err);
                            warn!(message, "AsyncResult: unhandled async transform error");
                            Result::_uncaught(message)
                        }
                    },
                    ResultState::Err(error) => from_error_state(error),
                }
            }),
        }
    }

    pub fn catch(self, f: impl FnOnce(E) -> Result<T, E> + Send + 'static) -> Self {
        Self {
            inner: Box::pin(async move {
                let current = self.inner.await;
                match current.state {
                    ResultState::Ok(_) => current,
                    ResultState::Err(ResultErrorKind::Typed(err)) => {
                        match catch_unwind(AssertUnwindSafe(|| f(err))) {
                            Ok(result) => result,
                            Err(callback_err) => {
                                let message = format_panic(callback_err);
                                warn!(
                                    message,
                                    "AsyncResult: unexpected error in async catch handler"
                                );
                                Result::_uncaught(message)
                            }
                        }
                    }
                    ResultState::Err(error) => from_error_state(error),
                }
            }),
        }
    }

    pub async fn on_value(self, f: impl FnOnce(&T)) -> Result<T, E> {
        self.inner.await.on_value(f)
    }

    pub async fn on_error(self, f: impl FnOnce(&E)) -> Result<T, E> {
        self.inner.await.on_error(f)
    }

    pub async fn parse<Out>(self, parser: impl FnOnce(T) -> SafeParseResult<Out>) -> Result<Out, E>
    where
        E: From<String>,
    {
        self.inner.await.parse(parser)
    }
}

/// Internal trait for sync transform return values.
pub trait TransformResult<T, E> {
    fn into_result(self) -> Result<T, E>;
}

impl<T, E> TransformResult<T, E> for Result<T, E> {
    fn into_result(self) -> Result<T, E> {
        self
    }
}

impl<T, E> TransformResult<T, E> for T {
    fn into_result(self) -> Result<T, E> {
        Result::ok(self)
    }
}

impl<T, E> TransformResult<T, E> for SafeParseResult<T>
where
    E: From<String>,
{
    fn into_result(self) -> Result<T, E> {
        match self {
            SafeParseResult::Ok(value) => Result::ok(value),
            SafeParseResult::Err(error) => Result::err(error.into()),
        }
    }
}

/// Internal trait for async transform return values.
pub trait AsyncTransformResult<T, E> {
    fn into_async_result(self) -> AsyncResult<T, E>;
}

impl<T: Send + 'static, E: Send + 'static> AsyncTransformResult<T, E> for AsyncResult<T, E> {
    fn into_async_result(self) -> AsyncResult<T, E> {
        self
    }
}

impl<T: Send + 'static, E: Send + 'static> AsyncTransformResult<T, E> for Result<T, E> {
    fn into_async_result(self) -> AsyncResult<T, E> {
        self.into_async_result()
    }
}

impl<T: Send + 'static, E: Send + 'static> AsyncTransformResult<T, E> for T {
    fn into_async_result(self) -> AsyncResult<T, E> {
        AsyncResult::ok(self)
    }
}

impl<T, E, F> AsyncTransformResult<T, E> for F
where
    T: Send + 'static,
    E: Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    fn into_async_result(self) -> AsyncResult<T, E> {
        AsyncResult {
            inner: Box::pin(async move { Result::ok(self.await) }),
        }
    }
}

/// Nullish helper used by nullable wrappers.
#[derive(Clone, Copy)]
pub enum Nullish<T> {
    Value(T),
    Null,
    Undefined,
}

impl<T> fmt::Debug for Nullish<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Nullish::Value(_) => f.debug_tuple("Nullish::Value").finish(),
            Nullish::Null => f.write_str("Nullish::Null"),
            Nullish::Undefined => f.write_str("Nullish::Undefined"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Ported: "wraps callback returning value" — lib/util/result.spec.ts line 34
    fn wraps_callback_returning_value() {
        let result: Res<i32, String> = Result::wrap(|| 42).unwrap();
        assert_eq!(result, Res::Ok { val: 42 });
    }

    #[test]
    fn handles_throw_in_callback() {
        let result: Res<i32, String> = Result::<i32, String>::wrap(|| panic!("oops")).unwrap();
        assert_eq!(
            result,
            Res::Err {
                err: "oops".to_owned(),
            },
        );
    }

    #[test]
    fn transforms_zod_style_values_to_result() {
        let res: Result<String, String> = Result::ok("foo").transform(|value| {
            if value.is_empty() {
                SafeParseResult::Err("value empty".to_owned())
            } else {
                SafeParseResult::Ok(value.to_uppercase())
            }
        });
        assert_eq!(
            res.unwrap(),
            Res::Ok {
                val: "FOO".to_owned()
            }
        );

        let err: Result<String, String> =
            Result::ok("").transform(|value| SafeParseResult::Err(format!("bad: {value}")));
        assert_eq!(
            err.unwrap(),
            Res::Err {
                err: "bad: ".to_owned(),
            },
        );
    }

    #[test]
    fn parses_with_safe_parse_function() {
        let parsed: Result<i32, String> =
            Result::parse_value(42i32, |value| SafeParseResult::Ok(value + 1));
        assert_eq!(parsed.unwrap(), Res::Ok { val: 43 });

        let parsed_err: Result<i32, String> =
            Result::parse_value(42i32, |_value| SafeParseResult::Err("bad".to_owned()));
        assert_eq!(
            parsed_err.unwrap(),
            Res::Err {
                err: "bad".to_owned(),
            },
        );
    }

    #[tokio::test]
    // Ported: "transforms Result to AsyncResult" — lib/util/result.spec.ts line 71
    async fn transforms_result_to_async_result() {
        let res = Result::ok("foo").transform_async(|value| AsyncResult::ok(value.to_uppercase()));
        assert_eq!(
            res.unwrap().await,
            Res::Ok {
                val: "FOO".to_owned()
            }
        );
    }
}
