use std::fmt::Debug;
use either::{Either, Left, Right};

#[derive(Debug, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum CommandResult<T, E> {
    Ok(T),
    Rejected(String),
    Err(E),
}

impl<T, E> CommandResult<T, E> {
    #[must_use = "if you intended to assert that this is ok, consider `.unwrap()` instead"]
    #[inline]
    pub const fn is_ok(&self) -> bool {
        matches!(*self, Self::Ok(_))
    }

    /// Returns `true` if the result is [`Ok`] and the value inside of it matches a predicate.
    ///
    /// # Examples
    ///
    /// ```
    /// use weather_disintegrate::CommandResult;
    ///
    /// let x: CommandResult<u32, &str> = CommandResult::Ok(2);
    /// assert_eq!(x.is_ok_and(|x| x > 1), true);
    ///
    /// let x: CommandResult<u32, &str> = CommandResult::Ok(0);
    /// assert_eq!(x.is_ok_and(|x| x > 1), false);
    ///
    /// let x: CommandResult<u32, &str> = CommandResult::Err("hey");
    /// assert_eq!(x.is_ok_and(|x| x > 1), false);
    /// ```
    #[must_use]
    #[inline]
    pub fn is_ok_and(self, f: impl FnOnce(T) -> bool) -> bool {
        match self {
            Self::Rejected(_) | Self::Err(_) => false,
            Self::Ok(x) => f(x),
        }
    }

    /// Returns `true` if the result is [`Rejected`].
    ///
    /// # Examples
    ///
    /// ```
    /// use weather_disintegrate::CommandResult;
    ///
    /// let x: CommandResult<i32, &str> = CommandResult::Ok(-3);
    /// assert_eq!(x.is_rejected(), false);
    ///
    /// let x: CommandResult<i32, &str> = CommandResult::Err("Some error message");
    /// assert_eq!(x.is_rejected(), false);
    ///
    /// let x: CommandResult<i32, &str> = CommandResult::Rejected("command not allowed".to_string());
    /// assert_eq!(x.is_rejected(), true);
    /// ```
    #[must_use = "if you intended to assert that this is rejected, consider `.unwrap_rejected()` instead"]
    #[inline]
    pub const fn is_rejected(&self) -> bool {
        matches!(*self, Self::Rejected(_))
    }

    /// Returns `true` if the result is [`Err`].
    ///
    /// # Examples
    ///
    /// ```
    /// use weather_disintegrate::CommandResult;
    ///
    /// let x: CommandResult<i32, &str> = CommandResult::Ok(-3);
    /// assert_eq!(x.is_err(), false);
    ///
    /// let x: CommandResult<i32, &str> = CommandResult::Err("Some error message");
    /// assert_eq!(x.is_err(), true);
    /// ```
    #[must_use = "if you intended to assert that this is err, consider `.unwrap_err()` instead"]
    #[inline]
    pub const fn is_err(&self) -> bool {
        matches!(*self, Self::Err(_))
    }

    /// Returns `true` if the result is [`Err`] and the value inside of it matches a predicate.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Error, ErrorKind};
    /// use std::process::Command;
    /// use weather_disintegrate::CommandResult;
    ///
    /// let x: CommandResult<u32, Error> = CommandResult::Err(Error::new(ErrorKind::NotFound, "!"));
    /// assert_eq!(x.is_err_and(|x| x.kind() == ErrorKind::NotFound), true);
    ///
    /// let x: CommandResult<u32, Error> = CommandResult::Err(Error::new(ErrorKind::PermissionDenied, "!"));
    /// assert_eq!(x.is_err_and(|x| x.kind() == ErrorKind::NotFound), false);
    ///
    /// let x: CommandResult<u32, Error> = CommandResult::Ok(123);
    /// assert_eq!(x.is_err_and(|x| x.kind() == ErrorKind::NotFound), false);
    /// ```
    #[must_use]
    #[inline]
    pub fn is_err_and(self, f: impl FnOnce(E) -> bool) -> bool {
        match self {
            Self::Ok(_) | Self::Rejected(_) => false,
            Self::Err(e) => f(e),
        }
    }

    pub const fn ok(payload: T) -> Self {
        Self::Ok(payload)
    }

    pub fn rejected(message: impl Into<String>) -> Self {
        Self::Rejected(message.into())
    }

    pub const fn err(error: E) -> Self {
        Self::Err(error)
    }

    #[allow(clippy::missing_const_for_fn)]
    #[inline]
    pub fn as_ok(self) -> Option<T> {
        match self {
            Self::Ok(x) => Some(x),
            Self::Rejected(_) | Self::Err(_) => None,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    #[inline]
    pub fn as_rejected(self) -> Option<String> {
        match self {
            Self::Rejected(msg) => Some(msg),
            Self::Ok(_) | Self::Err(_) => None,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    #[inline]
    pub fn as_err(self) -> Option<E> {
        match self {
            Self::Err(x) => Some(x),
            Self::Ok(_) | Self::Rejected(_) => None,
        }
    }

    /// Maps a `CommandResult<T, E>` to `CommandResult<U, E>` by applying a function to a
    /// contained [`Ok`] value, leaving an [`Rejected`] or [`Err`] value untouched.
    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(self, op: F) -> CommandResult<U, E> {
        match self {
            Self::Ok(t) => CommandResult::Ok(op(t)),
            Self::Rejected(msg) => CommandResult::Rejected(msg),
            Self::Err(e) => CommandResult::Err(e),
        }
    }

    /// Returns the provided default (if [`CommandResult::Err`] or [`CommandResult::Rejected`]), or
    /// applies a function to the contained value (if [`CommandResult::Ok`]),
    ///
    /// Arguments passed to `map_or` are eagerly evaluated; if you are passing
    /// the result of a function call, it is recommended to use [`map_or_else`],
    /// which is lazily evaluated.
    ///
    /// [`map_or_else`]: CommandResult::map_or_else
    ///
    /// # Examples
    ///
    /// ```
    /// use weather_disintegrate::CommandResult;
    /// let x: CommandResult<_, &str> = CommandResult::Ok("foo");
    /// assert_eq!(x.map_or(42, |v| v.len()), 3);
    ///
    /// let x: CommandResult<&str, _> = CommandResult::Err("bar");
    /// assert_eq!(x.map_or(42, |v| v.len()), 42);
    /// ```
    #[inline]
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            Self::Ok(t) => f(t),
            Self::Rejected(_) | Self::Err(_) => default,
        }
    }

    /// Maps a `CommandResult<T, E>` to `U` by applying fallback function `default` to
    /// a contained either [`CommandResult::Rejected`] or [`CommandResult::Err`] value, or function
    /// `f` to a contained [`CommandResult::Ok`] value.
    ///
    /// This function can be used to unpack a successful result while handling an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use weather_disintegrate::CommandResult;
    /// let k = 21;
    ///
    /// let x : CommandResult<_, &str> = CommandResult::Ok("foo");
    /// assert_eq!(x.map_or_else(|e| k * 2, |v| v.len()), 3);
    ///
    /// let x : CommandResult<&str, _> = CommandResult::Err("bar");
    /// assert_eq!(x.map_or_else(|e| k * 2, |v| v.len()), 42);
    /// ```
    #[inline]
    pub fn map_or_else<U, D: FnOnce(Either<String, E>) -> U, F: FnOnce(T) -> U>(
        self,
        default: D,
        f: F,
    ) -> U {
        match self {
            Self::Ok(t) => f(t),
            Self::Rejected(msg) => default(Left(msg)),
            Self::Err(e) => default(Right(e)),
        }
    }

    /// Maps a `CommandResult<T, E>` to `CommandResult<T, F>` by applying a function to a
    /// contained [`CommandResult::Err`] value, leaving an [`CommandResult::Ok`] or
    /// [`CommandResult::Rejected`] value untouched.
    ///
    /// This function can be used to pass through a successful result while handling an error.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use weather_disintegrate::CommandResult;
    /// fn stringify(x: u32) -> String { format!("error code: {x}") }
    ///
    /// let x: CommandResult<u32, u32> = CommandResult::Ok(2);
    /// assert_eq!(x.map_err(stringify), CommandResult::Ok(2));
    ///
    /// let x: CommandResult<u32, u32> = CommandResult::Err(13);
    /// assert_eq!(x.map_err(stringify), CommandResult::Err("error code: 13".to_string()));
    /// ```
    #[inline]
    pub fn map_err<F, O: FnOnce(E) -> F>(self, op: O) -> CommandResult<T, F> {
        match self {
            Self::Ok(t) => CommandResult::Ok(t),
            Self::Rejected(msg) => CommandResult::Rejected(msg),
            Self::Err(e) => CommandResult::Err(op(e)),
        }
    }

    /// Calls the provided closure with a reference to the contained value (if [`CommandResult::Ok`]).
    #[inline]
    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Self {
        if let Self::Ok(ref t) = self {
            f(t);
        }

        self
    }

    /// Returns the contained [`CommandResult::Ok`] value, consuming the `self` value.
    ///
    /// Because this function may panic, its use is generally discouraged.
    /// Instead, prefer to use pattern matching and handle the [`CommandResult::Rejected`] or
    /// [`CommandResult::Err`] cases explicitly, or call [`unwrap_or`], [`unwrap_or_else`], or
    /// [`unwrap_or_default`].
    ///
    /// [`unwrap_or`]: CommandResult::unwrap_or
    /// [`unwrap_or_else`]: CommandResult::unwrap_or_else
    /// [`unwrap_or_default`]: CommandResult::unwrap_or_default
    ///
    /// # Panics
    ///
    /// Panics if the value is either an [`CommandResult::Rejected`] or [`CommandResult::Err`],
    /// with a panic message provided by the [`CommandResult::Rejected`] or [`CommandResult::Err`]'s
    /// value.
    ///
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use weather_disintegrate::CommandResult;
    /// let x: CommandResult<u32, &str> = CommandResult::Ok(2);
    /// assert_eq!(x.unwrap(), 2);
    /// ```
    ///
    /// ```should_panic
    /// use weather_disintegrate::CommandResult;
    /// let x: CommandResult<u32, &str> = CommandResult::Err("emergency failure");
    /// x.unwrap(); // panics with `emergency failure`
    /// ```
    #[inline]
    #[track_caller]
    pub fn unwrap(self) -> T
        where
            E: Debug,
    {
        match self {
            Self::Ok(t) => t,
            Self::Rejected(msg) => {
                panic!("called `CommandResult::unwrap() on a `Rejected` value: {msg}")
            }
            Self::Err(e) => panic!("called `CommandResult::unwrap()` on an `Err` value: {e:?}"),
        }
    }

    /// Returns the contained [`CommandResult::Ok`] value or a default
    ///
    /// Consumes the `self` argument then, if [`CommandResult::Ok`], returns the contained
    /// value, otherwise if [`CommandResult::Rejected`] or [`CommandResult::Err`], returns the
    /// default value for that type.
    #[inline]
    pub fn unwrap_or_default(self) -> T
        where
            T: Default,
    {
        match self {
            Self::Ok(x) => x,
            Self::Rejected(_) | Self::Err(_) => Default::default(),
        }
    }

    /// Returns the contained [`CommandResult::Ok`] value or a provided default.
    ///
    /// Arguments passed to `unwrap_or` are eagerly evaluated; if you are passing
    /// the result of a function call, it is recommended to use [`unwrap_or_else`],
    /// which is lazily evaluated.
    ///
    /// [`unwrap_or_else`]: CommandResult::unwrap_or_else
    ///
    /// # Examples
    ///
    /// ```
    /// use weather_disintegrate::CommandResult;
    /// let default = 2;
    /// let x: CommandResult<u32, &str> = CommandResult::Ok(9);
    /// assert_eq!(x.unwrap_or(default), 9);
    ///
    /// let x: CommandResult<u32, &str> = CommandResult::Err("error");
    /// assert_eq!(x.unwrap_or(default), default);
    /// ```
    #[allow(clippy::missing_const_for_fn)]
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Ok(t) => t,
            Self::Rejected(_) | Self::Err(_) => default,
        }
    }

    /// Returns the contained [`CommandResult::Ok`] value or computes it from a closure.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use either::Either;
    /// use weather_disintegrate::CommandResult;
    /// fn count(x: Either<String, &str>) -> usize { x.either(|l| l.len() + 1, |r| r.len() * 2) }
    ///
    /// assert_eq!(CommandResult::Ok(2).unwrap_or_else(count), 2);
    /// assert_eq!(CommandResult::Err("foo").unwrap_or_else(count), 6);
    /// ```
    #[inline]
    pub fn unwrap_or_else<F: FnOnce(Either<String, E>) -> T>(self, op: F) -> T {
        match self {
            Self::Ok(t) => t,
            Self::Rejected(msg) => op(Left(msg)),
            Self::Err(e) => op(Right(e)),
        }
    }
}

impl<T, E> From<E> for CommandResult<T, E> {
    fn from(error: E) -> Self {
        Self::Err(error)
    }
}

